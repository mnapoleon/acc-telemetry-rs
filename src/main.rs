// acc-telemetry-rs — Assetto Corsa Competizione shared memory telemetry reader.
//
// Windows-only: uses the Win32 named shared memory API to read live telemetry
// from ACC running on the same machine.  Press 1 / 2 / 3 in the console to
// print physics / graphics / static data respectively.  Press Escape to exit.

#![cfg(windows)]

mod shared_memory;

use std::thread;
use std::time::Duration;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_READ, PAGE_READWRITE,
};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

use shared_memory::{
    decode_wstr, AcSessionType, AcStatus, PageFileGraphic, PageFilePhysics, PageFileStatic,
};

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
// Print helpers (matching the original C++ printData / printData2 style)
// ---------------------------------------------------------------------------

fn print_scalar(name: &str, value: f32) {
    println!("{name} : {value}");
}

fn print_scalar_i(name: &str, value: i32) {
    println!("{name} : {value}");
}

fn print_array_f(name: &str, values: &[f32]) {
    let joined: Vec<String> = values.iter().map(|v| v.to_string()).collect();
    println!("{name} : {}", joined.join(" , "));
}

/// Print a 2-D `[N][3]` array the same way as the C++ `printData2`.
fn print_array2_f(name: &str, rows: &[[f32; 3]]) {
    print!("{name} : ");
    for (i, row) in rows.iter().enumerate() {
        let joined: Vec<String> = row.iter().map(|v| v.to_string()).collect();
        print!("{i} : {} ;", joined.join(" , "));
        println!();
    }
}

/// Print car coordinates (special-cased so the outer array size is fixed at 60).
fn print_car_coordinates(name: &str, rows: &[[f32; 3]; 60]) {
    print!("{name} : ");
    for (i, row) in rows.iter().enumerate() {
        let joined: Vec<String> = row.iter().map(|v| v.to_string()).collect();
        print!("{i} : {} ;", joined.join(" , "));
        println!();
    }
}

// ---------------------------------------------------------------------------
// Section printers
// ---------------------------------------------------------------------------

fn print_physics(seg: &SharedMemSegment) {
    // Safety: segment opened with size_of::<PageFilePhysics>().
    let pf: &PageFilePhysics = unsafe { seg.as_ref() };

    println!("---------------PHYSICS INFO---------------");
    print_array_f("acc G", &pf.acc_g);
    print_scalar("brake", pf.brake);
    print_array_f("camber rad", &pf.camber_rad);
    print_array_f("damage", &pf.car_damage);
    print_scalar("car height", pf.cg_height);
    print_scalar("drs", pf.drs);
    print_scalar("tc", pf.tc);
    print_scalar("fuel", pf.fuel);
    print_scalar("gas", pf.gas);
    print_scalar_i("gear", pf.gear);
    print_scalar_i("number of tyres out", pf.number_of_tyres_out);
    print_scalar_i("packet id", pf.packet_id);
    print_scalar("heading", pf.heading);
    print_scalar("pitch", pf.pitch);
    print_scalar("roll", pf.roll);
    print_scalar_i("rpms", pf.rpms);
    print_scalar("speed kmh", pf.speed_kmh);
    print_array2_f("contact point", &pf.tyre_contact_point);
    print_array2_f("contact normal", &pf.tyre_contact_normal);
    print_array2_f("contact heading", &pf.tyre_contact_heading);
    print_scalar("steer", pf.steer_angle);
    print_array_f("suspension travel", &pf.suspension_travel);
    print_array_f("tyre core temp", &pf.tyre_core_temperature);
    print_array_f("tyre dirty level", &pf.tyre_dirty_level);
    print_array_f("tyre wear", &pf.tyre_wear);
    print_array_f("velocity", &pf.velocity);
    print_array_f("wheel angular speed", &pf.wheel_angular_speed);
    print_array_f("wheel load", &pf.wheel_load);
    print_array_f("wheel slip", &pf.wheel_slip);
    print_array_f("wheel pressure", &pf.wheels_pressure);
}

fn print_graphics(seg: &SharedMemSegment) {
    // Safety: segment opened with size_of::<PageFileGraphic>().
    let pf: &PageFileGraphic = unsafe { seg.as_ref() };

    println!("---------------GRAPHICS INFO---------------");
    print_scalar_i("packetID", pf.packet_id);
    println!("STATUS : {}", AcStatus::from_i32(pf.status));
    println!("session : {}", AcSessionType::from_i32(pf.session));
    print_scalar_i("completed laps", pf.completed_laps);
    print_scalar_i("position", pf.position);
    println!("current time s : {}", decode_wstr(&pf.current_time));
    print_scalar_i("current time", pf.i_current_time);
    println!("last time s : {}", decode_wstr(&pf.last_time));
    print_scalar_i("last time", pf.i_last_time);
    println!("best time s : {}", decode_wstr(&pf.best_time));
    print_scalar_i("best time", pf.i_best_time);
    print_scalar("sessionTimeLeft", pf.session_time_left);
    print_scalar("distanceTraveled", pf.distance_traveled);
    print_scalar_i("isInPit", pf.is_in_pit);
    print_scalar_i("currentSectorIndex", pf.current_sector_index);
    print_scalar_i("lastSectorTime", pf.last_sector_time);
    print_scalar_i("numberOfLaps", pf.number_of_laps);
    println!("TYRE COMPOUND : {}", decode_wstr(&pf.tyre_compound));
    print_scalar("replayMult", pf.replay_time_multiplier);
    print_scalar("normalizedCarPosition", pf.normalized_car_position);
    print_car_coordinates("carCoordinates", &pf.car_coordinates);
}

fn print_static(seg: &SharedMemSegment) {
    // Safety: segment opened with size_of::<PageFileStatic>().
    let pf: &PageFileStatic = unsafe { seg.as_ref() };

    println!("---------------STATIC INFO---------------");
    println!("SM VERSION {}", decode_wstr(&pf.sm_version));
    println!("AC VERSION {}", decode_wstr(&pf.ac_version));
    print_scalar_i("number of sessions", pf.number_of_sessions);
    print_scalar_i("numCars", pf.num_cars);
    println!("Car model {}", decode_wstr(&pf.car_model));
    println!("Car track {}", decode_wstr(&pf.track));
    println!("Player Name {}", decode_wstr(&pf.player_name));
    print_scalar_i("sectorCount", pf.sector_count);
    print_scalar("maxTorque", pf.max_torque);
    print_scalar("maxPower", pf.max_power);
    print_scalar_i("maxRpm", pf.max_rpm);
    print_scalar("maxFuel", pf.max_fuel);
    print_array_f("suspensionMaxTravel", &pf.suspension_max_travel);
    print_array_f("tyreRadius", &pf.tyre_radius);
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let physics_name = to_wide_null("Local\\acpmf_physics");
    let graphics_name = to_wide_null("Local\\acpmf_graphics");
    let static_name = to_wide_null("Local\\acpmf_static");

    let physics_seg =
        match SharedMemSegment::open(&physics_name, std::mem::size_of::<PageFilePhysics>()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to open physics segment: {e}");
                std::process::exit(1);
            }
        };

    let graphics_seg =
        match SharedMemSegment::open(&graphics_name, std::mem::size_of::<PageFileGraphic>()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to open graphics segment: {e}");
                std::process::exit(1);
            }
        };

    let static_seg =
        match SharedMemSegment::open(&static_name, std::mem::size_of::<PageFileStatic>()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to open static segment: {e}");
                std::process::exit(1);
            }
        };

    println!("ACC Telemetry Reader — press 1 for physics, 2 for graphics, 3 for static");
    println!("Press Escape to exit.");

    // Poll at ~60 Hz (16 ms sleep) instead of busy-looping like the original.
    // Virtual-key codes: 0x31='1', 0x32='2', 0x33='3', 0x1B=Escape
    loop {
        let k1 = unsafe { GetAsyncKeyState(0x31) };
        let k2 = unsafe { GetAsyncKeyState(0x32) };
        let k3 = unsafe { GetAsyncKeyState(0x33) };
        let esc = unsafe { GetAsyncKeyState(0x1B) };

        if k1 != 0 {
            print_physics(&physics_seg);
        }
        if k2 != 0 {
            print_graphics(&graphics_seg);
        }
        if k3 != 0 {
            print_static(&static_seg);
        }
        if esc != 0 {
            println!("Exiting.");
            break;
        }

        // ~60 Hz — avoids 100% CPU usage from the original C++ busy-poll.
        thread::sleep(Duration::from_millis(16));
    }

    // physics_seg, graphics_seg, static_seg dropped here.
    // Drop impl calls UnmapViewOfFile + CloseHandle for each — always reached
    // (unlike the original C++ where dismiss() was unreachable after while(true)).
}
