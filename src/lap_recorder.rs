/// Lap recording and sector time tracking for ACC telemetry.
///
/// This module detects lap completions and sector boundaries by monitoring
/// ACC's shared memory telemetry data and builds structured lap records.
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::shared_memory::PageFileGraphic;

// ---------------------------------------------------------------------------
// Data Types
// ---------------------------------------------------------------------------

/// Lap status classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LapStatus {
    /// Normal racing lap (valid for statistics)
    Normal,
    /// Lap with pit stop (excluded from averages)
    Pit,
    /// Invalid lap (no timing recorded)
    Invalid,
}

impl std::fmt::Display for LapStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LapStatus::Normal => write!(f, "normal"),
            LapStatus::Pit => write!(f, "pit"),
            LapStatus::Invalid => write!(f, "invalid"),
        }
    }
}

/// Sector timing information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorTime {
    /// Sector index (0-based)
    pub index: usize,
    /// Sector time in milliseconds
    pub time_ms: i32,
    /// Human-readable formatted time (e.g., "0:48.100")
    pub formatted: String,
}

/// Complete lap record with timing and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LapRecord {
    /// Lap number (1-based)
    pub lap_number: i32,
    /// Lap status (normal/pit/invalid)
    pub status: LapStatus,
    /// Total lap time in milliseconds
    pub total_time_ms: i32,
    /// Human-readable formatted time (e.g., "2:25.340")
    pub total_time_formatted: String,
    /// Sector times for this lap
    pub sectors: Vec<SectorTime>,
    /// ISO 8601 timestamp when lap was completed
    pub timestamp: String,
}

// ---------------------------------------------------------------------------
// Lap Recorder
// ---------------------------------------------------------------------------

/// Tracks telemetry state and detects lap completions.
pub struct LapRecorder {
    /// Previous value of completed_laps counter
    previous_completed_laps: i32,
    /// Previous sector index
    previous_sector_index: i32,
    /// Previous last_sector_time value
    previous_last_sector_time: i32,
    /// Accumulates sector times during current lap
    current_lap_sectors: Vec<SectorTime>,
    /// Tracks if car was in pit during this lap
    is_in_pit_during_lap: bool,
    /// Lap number counter for tracking (helps identify lap boundaries)
    current_lap_number: i32,
}

impl LapRecorder {
    /// Create a new lap recorder.
    pub fn new() -> Self {
        Self {
            previous_completed_laps: 0,
            previous_sector_index: -1,
            previous_last_sector_time: 0,
            current_lap_sectors: Vec::new(),
            is_in_pit_during_lap: false,
            current_lap_number: 0,
        }
    }

    /// Update the recorder with latest telemetry data.
    ///
    /// Returns `Some(LapRecord)` if a lap was just completed, `None` otherwise.
    pub fn update(&mut self, graphics: &PageFileGraphic) -> Option<LapRecord> {
        // Read current telemetry values
        let completed_laps = graphics.completed_laps;
        let current_sector_index = graphics.current_sector_index;
        let last_sector_time = graphics.last_sector_time;
        let is_in_pit = graphics.is_in_pit;
        let i_last_time = graphics.i_last_time;

        // Track pit status during this lap
        if is_in_pit == 1 {
            self.is_in_pit_during_lap = true;
        }

        // Detect lap completion (completed_laps counter incremented)
        if completed_laps > self.previous_completed_laps {
            // A lap just finished! Record the final sector before resetting
            if self.previous_last_sector_time > 0 && self.previous_sector_index != -1 {
                let sector = SectorTime {
                    index: self.previous_sector_index as usize,
                    time_ms: self.previous_last_sector_time,
                    formatted: Self::format_time(self.previous_last_sector_time),
                };
                self.current_lap_sectors.push(sector);
            }

            // Determine lap status
            let status = if self.is_in_pit_during_lap {
                LapStatus::Pit
            } else if i_last_time == 0 {
                LapStatus::Invalid
            } else {
                LapStatus::Normal
            };

            // Build lap record
            let lap_record = LapRecord {
                lap_number: completed_laps,
                status,
                total_time_ms: i_last_time,
                total_time_formatted: Self::format_time(i_last_time),
                sectors: self.current_lap_sectors.clone(),
                timestamp: Utc::now().to_rfc3339(),
            };

            // Reset state for next lap
            self.previous_completed_laps = completed_laps;
            self.current_lap_number = completed_laps;
            self.current_lap_sectors.clear();
            self.is_in_pit_during_lap = false;
            // Reset sector tracking for next lap
            self.previous_sector_index = -1;
            self.previous_last_sector_time = 0;

            return Some(lap_record);
        }

        // Detect sector boundary (sector index changed within same lap)
        if current_sector_index != self.previous_sector_index {
            // Record the previous sector's time when transitioning to a new sector
            if self.previous_last_sector_time > 0 && self.previous_sector_index != -1 {
                let sector = SectorTime {
                    index: self.previous_sector_index as usize,
                    time_ms: self.previous_last_sector_time,
                    formatted: Self::format_time(self.previous_last_sector_time),
                };
                self.current_lap_sectors.push(sector);
            }

            // Update tracking
            self.previous_sector_index = current_sector_index;
            self.previous_last_sector_time = last_sector_time;
        } else if last_sector_time != self.previous_last_sector_time {
            // Sector time updated without index change (can happen on lap completion)
            self.previous_last_sector_time = last_sector_time;
        }

        None
    }

    /// Format milliseconds to "M:SS.sss" string.
    ///
    /// Examples:
    /// - 145230 ms → "2:25.230"
    /// - 48100 ms → "0:48.100"
    /// - 0 ms → "0:00.000"
    fn format_time(ms: i32) -> String {
        if ms <= 0 {
            return "0:00.000".to_string();
        }

        let total_seconds = ms / 1000;
        let milliseconds = ms % 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;

        format!("{}:{:02}.{:03}", minutes, seconds, milliseconds)
    }
}

impl Default for LapRecorder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time() {
        assert_eq!(LapRecorder::format_time(0), "0:00.000");
        assert_eq!(LapRecorder::format_time(1000), "0:01.000");
        assert_eq!(LapRecorder::format_time(48100), "0:48.100");
        assert_eq!(LapRecorder::format_time(60000), "1:00.000");
        assert_eq!(LapRecorder::format_time(145230), "2:25.230");
        assert_eq!(LapRecorder::format_time(225220), "3:45.220");
    }

    #[test]
    fn test_lap_status_display() {
        assert_eq!(LapStatus::Normal.to_string(), "normal");
        assert_eq!(LapStatus::Pit.to_string(), "pit");
        assert_eq!(LapStatus::Invalid.to_string(), "invalid");
    }
}
