use chrono::Utc;
/// Debug logging for sector and lap tracking issues.
///
/// Writes detailed logs to a debug file for diagnosing sector/lap detection problems.
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// Global debug logger instance
static DEBUG_LOGGER: std::sync::OnceLock<Mutex<DebugLogger>> = std::sync::OnceLock::new();

/// Debug logger that writes to a file
pub struct DebugLogger {
    file: File,
    file_path: PathBuf,
}

impl DebugLogger {
    /// Initialize the debug logger
    pub fn init() -> Result<(), Box<dyn std::error::Error>> {
        // Create .debug_logs directory if it doesn't exist
        let debug_dir = PathBuf::from(".debug_logs");
        std::fs::create_dir_all(&debug_dir)?;

        // Generate filename with timestamp
        let now = Utc::now();
        let filename = format!("debug_{}.log", now.format("%Y%m%d_%H%M%S"));
        let file_path = debug_dir.join(filename);

        // Create/open the debug log file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        // Print before moving file_path into the struct
        println!("Debug log: {}", file_path.display());

        let logger = DebugLogger { file, file_path };

        // Store in global
        DEBUG_LOGGER.get_or_init(|| Mutex::new(logger));

        Ok(())
    }

    /// Log a message to the debug file
    fn log_message(msg: &str) -> Result<(), Box<dyn std::error::Error>> {
        let logger = DEBUG_LOGGER.get_or_init(|| {
            let debug_dir = PathBuf::from(".debug_logs");
            let _ = std::fs::create_dir_all(&debug_dir);
            let now = Utc::now();
            let filename = format!("debug_{}.log", now.format("%Y%m%d_%H%M%S"));
            let file_path = debug_dir.join(filename);
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_path)
                .unwrap_or_else(|_| {
                    File::create("debug_fallback.log").expect("Failed to create fallback log")
                });
            Mutex::new(DebugLogger { file, file_path })
        });

        if let Ok(mut logger) = logger.lock() {
            let timestamp = Utc::now().format("%H:%M:%S%.3f");
            writeln!(logger.file, "[{}] {}", timestamp, msg)?;
            logger.file.flush()?;
        }

        Ok(())
    }

    /// Log a lap start event
    pub fn log_lap_start(
        sample_number: u64,
        lap_number: i32,
        completed_laps: i32,
        current_sector_index: i32,
        last_sector_time: i32,
        i_last_time: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let msg = format!(
            "[{}] LAP START: lap_number={} completed_laps={} sector_index={} last_sector_time={} total_time={}",
            sample_number, lap_number, completed_laps, current_sector_index, last_sector_time, i_last_time
        );
        Self::log_message(&msg)
    }

    /// Log a sector transition
    pub fn log_sector_transition(
        sample_number: u64,
        lap_number: i32,
        from_sector: i32,
        to_sector: i32,
        sector_time: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let msg = format!(
            "[{}] SECTOR TRANSITION: lap={} sector {} -> {} (time={}ms)",
            sample_number, lap_number, from_sector, to_sector, sector_time
        );
        Self::log_message(&msg)
    }

    /// Log a sector being recorded
    pub fn log_sector_recorded(
        sample_number: u64,
        lap_number: i32,
        sector_index: usize,
        time_ms: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let msg = format!(
            "[{}] SECTOR RECORDED: lap={} sector_index={} time={}ms",
            sample_number, lap_number, sector_index, time_ms
        );
        Self::log_message(&msg)
    }

    /// Log final lap record
    pub fn log_lap_completed(
        sample_number: u64,
        lap_number: i32,
        total_time_ms: i32,
        sector_count: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let msg = format!(
            "[{}] LAP COMPLETED: lap={} total_time={}ms sectors_recorded={}",
            sample_number, lap_number, total_time_ms, sector_count
        );
        Self::log_message(&msg)
    }

    /// Log raw telemetry state
    pub fn log_telemetry_state(
        sample_number: u64,
        completed_laps: i32,
        current_sector_index: i32,
        last_sector_time: i32,
        i_last_time: i32,
        previous_sector_index: i32,
        previous_last_sector_time: i32,
        current_lap_sectors_count: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let msg = format!(
            "[{}] TELEMETRY: completed_laps={} cur_sector={} last_sector_time={} total_time={} | prev_sector={} prev_time={} sectors_count={}",
            sample_number, completed_laps, current_sector_index, last_sector_time, i_last_time,
            previous_sector_index, previous_last_sector_time, current_lap_sectors_count
        );
        Self::log_message(&msg)
    }

    /// Log recorder initialization state
    pub fn log_initialization(
        sample_number: u64,
        normalized_car_position: f32,
        current_sector_index: i32,
        last_sector_time: i32,
        completed_laps: i32,
        i_last_time: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let msg = format!(
            "[{}] INITIALIZATION: car_position={:.2} sector_index={} last_sector_time={} completed_laps={} total_time={}",
            sample_number, normalized_car_position, current_sector_index, last_sector_time, completed_laps, i_last_time
        );
        Self::log_message(&msg)
    }

    /// Log when sector 0 is confirmed to be at the start/finish line
    pub fn log_sector_zero_detected(sample_number: u64) -> Result<(), Box<dyn std::error::Error>> {
        let msg = format!(
            "[{}] SECTOR ZERO CONFIRMED: Actual start/finish line crossing detected (not ACC default)",
            sample_number
        );
        Self::log_message(&msg)
    }

    /// Log raw telemetry state for every sample (called every 20ms)
    pub fn log_sample_state(
        sample_number: u64,
        current_position: f32,
        current_sector_index: i32,
        last_sector_time: i32,
        completed_laps: i32,
        number_of_laps: i32,
        i_last_time: i32,
        previous_car_position: f32,
        previous_sector_index: i32,
        previous_last_sector_time: i32,
        current_lap_number: i32,
        lap_in_progress: bool,
        has_seen_sector_zero: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let msg = format!(
            "[{}] SAMPLE: pos={:.3} sector={} time={} laps={}/{} total_time={} | prev_pos={:.3} prev_sector={} prev_time={} our_lap={} lap_prog={} seen_s0={}",
            sample_number, current_position, current_sector_index, last_sector_time,
            completed_laps, number_of_laps, i_last_time, previous_car_position, previous_sector_index,
            previous_last_sector_time, current_lap_number, lap_in_progress, has_seen_sector_zero
        );
        Self::log_message(&msg)
    }
}
