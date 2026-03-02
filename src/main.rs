// acc-telemetry-rs — Assetto Corsa Competizione shared memory telemetry recorder.
//
// Windows-only: uses the Win32 named shared memory API to read live telemetry
// from ACC running on the same machine and records lap times to JSON files.

#![cfg(windows)]

mod debug_logger;
mod json_export;
mod lap_recorder;
mod shared_memory;

use std::thread;
use std::time::Duration;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_READ, PAGE_READWRITE,
};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

use debug_logger::DebugLogger;
use json_export::JsonExporter;
use lap_recorder::LapRecorder;
use shared_memory::{decode_wstr, AcSessionType, PageFileGraphic, PageFileStatic};

// ---------------------------------------------------------------------------
// Shared memory segment wrapper
// ---------------------------------------------------------------------------

/// Owns a Win32 named shared memory mapping and its mapped view.
struct SharedMemSegment {
    handle: HANDLE,
    buffer: *const u8,
}

impl SharedMemSegment {
    /// Open (or create) a named shared memory segment and map a read-only view.
    ///
    /// `name` must be a null-terminated UTF-16 slice (use `to_wide_null`).
    /// Returns `Err` with a description if any Win32 call fails.
    fn open(name: &[u16], size: usize) -> Result<Self, String> {
        // Safety: null-terminated wide string and valid size are provided by caller.
        let handle = unsafe {
            CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                None,
                PAGE_READWRITE,
                0,
                size as u32,
                PCWSTR(name.as_ptr()),
            )
        }
        .map_err(|e| format!("CreateFileMappingW failed: {e}"))?;

        // Safety: handle is valid (error already returned above if not).
        let view = unsafe { MapViewOfFile(handle, FILE_MAP_READ, 0, 0, size) };
        if view.Value.is_null() {
            unsafe {
                let _ = CloseHandle(handle);
            }
            return Err("MapViewOfFile failed: returned null pointer".to_string());
        }

        Ok(Self {
            handle,
            buffer: view.Value as *const u8,
        })
    }

    /// Reinterpret the mapped buffer as a reference to `T`.
    ///
    /// # Safety
    /// The caller must ensure:
    /// - `T` matches the binary layout written by ACC into this segment.
    /// - The segment was opened with `size >= size_of::<T>()`.
    /// - The returned reference is not used after `self` is dropped.
    unsafe fn as_ref<T>(&self) -> &T {
        &*(self.buffer as *const T)
    }
}

impl Drop for SharedMemSegment {
    fn drop(&mut self) {
        if !self.buffer.is_null() {
            unsafe {
                let view = windows::Win32::System::Memory::MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.buffer as *mut _,
                };
                let _ = UnmapViewOfFile(view);
            }
        }
        if !self.handle.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: encode a Rust &str as a null-terminated UTF-16 Vec<u16>
// ---------------------------------------------------------------------------

fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    // Initialize debug logger
    if let Err(e) = DebugLogger::init() {
        eprintln!("Warning: Failed to initialize debug logger: {e}");
    }

    let graphics_name = to_wide_null("Local\\acpmf_graphics");
    let static_name = to_wide_null("Local\\acpmf_static");

    let graphics_seg =
        match SharedMemSegment::open(&graphics_name, std::mem::size_of::<PageFileGraphic>()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to open graphics segment: {e}");
                eprintln!("Make sure ACC is running and you've started a session.");
                std::process::exit(1);
            }
        };

    let static_seg =
        match SharedMemSegment::open(&static_name, std::mem::size_of::<PageFileStatic>()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to open static segment: {e}");
                eprintln!("Make sure ACC is running and you've started a session.");
                std::process::exit(1);
            }
        };

    // Initialize lap recorder
    let mut recorder = LapRecorder::new();

    // JSON exporter will be created once we detect an active session
    let mut exporter: Option<JsonExporter> = None;

    // Print startup message
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║          ACC Telemetry Recorder - Lap Time Logger            ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    println!("Waiting for active session (monitoring for car movement)...");
    println!("Press Escape to exit.");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    // Main polling loop (~60 Hz)
    loop {
        let esc = unsafe { GetAsyncKeyState(0x1B) };

        // Get current telemetry data
        let graphics: &PageFileGraphic = unsafe { graphics_seg.as_ref() };
        let static_data: &PageFileStatic = unsafe { static_seg.as_ref() };

        // Initialize exporter once we detect an active session (car position indicates movement)
        if exporter.is_none() && graphics.normalized_car_position > 0.0 {
            // Extract session metadata
            let car_model = decode_wstr(&static_data.car_model);
            let track = decode_wstr(&static_data.track);
            let player_name = decode_wstr(&static_data.player_name);
            let session_type = AcSessionType::from_i32(graphics.session);

            // Create JSON exporter
            match JsonExporter::new(
                car_model.clone(),
                track.clone(),
                player_name.clone(),
                session_type.to_string(),
            ) {
                Ok(exp) => {
                    println!("Active session detected!");
                    println!();
                    println!("Session Info:");
                    println!("  Track:   {}", track);
                    println!("  Car:     {}", car_model);
                    println!("  Player:  {}", player_name);
                    println!("  Session: {}", session_type);
                    println!();
                    println!("Recording to: {}", exp.file_path().display());
                    println!();
                    println!("═══════════════════════════════════════════════════════════════");
                    println!();

                    exporter = Some(exp);
                }
                Err(e) => {
                    eprintln!("Failed to create JSON exporter: {e}");
                    std::process::exit(1);
                }
            }
        }

        // Only process laps if exporter is initialized
        if let Some(ref mut exp) = exporter {
            // Update recorder and check for lap completion
            if let Some(lap) = recorder.update(graphics) {
                // Print lap completion to console
                println!(
                    "Lap {} completed: {} [{}]",
                    lap.lap_number, lap.total_time_formatted, lap.status
                );

                // Write lap to JSON file
                if let Err(e) = exp.write_lap(lap) {
                    eprintln!("Warning: Failed to write lap to file: {e}");
                }
            }
        }

        // Check for exit
        if esc != 0 {
            println!();
            println!("═══════════════════════════════════════════════════════════════");

            // Finalize recording if exporter was created
            if let Some(ref mut exp) = exporter {
                println!("Exiting. Finalizing recording...");

                if let Err(e) = exp.finalize() {
                    eprintln!("Error finalizing recording: {e}");
                    std::process::exit(1);
                }

                println!("Recording saved to: {}", exp.file_path().display());
            } else {
                println!("Exiting. No active session was detected.");
            }

            println!("Thank you for using ACC Telemetry Recorder!");
            break;
        }

        // 50 Hz polling rate (matches ACC's telemetry update rate)
        thread::sleep(Duration::from_millis(20));
    }

    // Segments dropped here, cleanup handled by Drop impl
}
