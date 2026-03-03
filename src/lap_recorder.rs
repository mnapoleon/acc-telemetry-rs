/// Lap recording and sector time tracking for ACC telemetry.
///
/// This module detects lap completions and sector boundaries by monitoring
/// ACC's shared memory telemetry data and builds structured lap records.
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::debug_logger::DebugLogger;
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
    /// Previous normalized car position (for detecting start/finish crossing)
    previous_car_position: f32,
    /// Flag indicating if we're actively tracking a lap in progress
    lap_in_progress: bool,
    /// Flag indicating if we've ever seen sector 0 (required for complete lap recording)
    has_seen_sector_zero: bool,
    /// Flag to prevent duplicate lap completion on same line crossing event
    /// Set when we complete a lap via line crossing, cleared on next non-line-crossing update
    just_completed_via_crossing: bool,
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
            previous_car_position: 0.0,
            lap_in_progress: false,
            has_seen_sector_zero: false,
            just_completed_via_crossing: false,
        }
    }

    /// Detect if the car crossed the start/finish line by checking if normalized_car_position
    /// wrapped from high (>0.5) to low (<0.5).
    ///
    /// This indicates the car crossed from near the end of the track back to the start.
    fn detect_start_finish_crossing(current_position: f32, previous_position: f32) -> bool {
        // Wrap-around detection: previous was near end of track, current is near start
        // Example: 0.95 -> 0.05 indicates crossing the line
        previous_position > 0.5 && current_position < 0.5
    }

    /// Update the recorder with latest telemetry data.
    ///
    /// Returns `Some(LapRecord)` if a lap was just completed, `None` otherwise.
    pub fn update(&mut self, graphics: &PageFileGraphic) -> Option<LapRecord> {
        // Read current telemetry values
        let current_position = graphics.normalized_car_position;
        let completed_laps = graphics.completed_laps;
        let current_sector_index = graphics.current_sector_index;
        let last_sector_time = graphics.last_sector_time;
        let is_in_pit = graphics.is_in_pit;
        let i_last_time = graphics.i_last_time;

        // Track pit status during this lap
        if is_in_pit == 1 {
            self.is_in_pit_during_lap = true;
        }

        // ========================================================================
        // STEP 1: INITIALIZE ON FIRST UPDATE
        // ========================================================================
        // On first update, capture current sector state but don't start lap yet.
        // We wait for sector 0 to ensure we only record complete laps.
        if self.previous_sector_index == -1 && !self.lap_in_progress {
            self.previous_sector_index = current_sector_index;
            self.previous_last_sector_time = last_sector_time;
            self.previous_car_position = current_position;
            self.previous_completed_laps = completed_laps;

            // Log initialization state from shared memory
            let _ = DebugLogger::log_initialization(
                current_position,
                current_sector_index,
                last_sector_time,
                completed_laps,
                i_last_time,
            );

            return None;
        }

        // ========================================================================
        // STEP 2: DETECT LINE CROSSING
        // ========================================================================
        let crossed_line =
            Self::detect_start_finish_crossing(current_position, self.previous_car_position);

        if crossed_line && self.lap_in_progress {
            // We have a complete lap to record

            // Log the completed lap with its telemetry state
            let _ = DebugLogger::log_telemetry_state(
                completed_laps,
                current_sector_index,
                last_sector_time,
                i_last_time,
                self.previous_sector_index,
                self.previous_last_sector_time,
                self.current_lap_sectors.len(),
            );

            // Record final sector (sector 2) if we have one
            if self.previous_sector_index == 2 && self.previous_last_sector_time > 0 {
                let final_sector = SectorTime {
                    index: 2,
                    time_ms: self.previous_last_sector_time,
                    formatted: Self::format_time(self.previous_last_sector_time),
                };
                let _ = DebugLogger::log_sector_recorded(
                    self.current_lap_number,
                    2,
                    self.previous_last_sector_time,
                );
                self.current_lap_sectors.push(final_sector);
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
                lap_number: self.current_lap_number,
                status,
                total_time_ms: i_last_time,
                total_time_formatted: Self::format_time(i_last_time),
                sectors: self.current_lap_sectors.clone(),
                timestamp: Utc::now().to_rfc3339(),
            };

            // Log the completed lap
            let _ = DebugLogger::log_lap_completed(
                self.current_lap_number,
                i_last_time,
                lap_record.sectors.len(),
            );

            // Reset for next lap
            self.current_lap_sectors.clear();
            self.is_in_pit_during_lap = false;
            self.previous_last_sector_time = 0;

            // Start next lap
            self.current_lap_number += 1;

            // CRITICAL: Set previous_sector_index to current to prevent stale sector state
            // from triggering another lap completion in the next update cycle.
            // When we detect line crossing (2->0), the next update might still see the same
            // sector transition because telemetry updates don't guarantee state has changed.
            // By setting previous = current, we ensure the next sector change won't immediately
            // match the "sector 2->0" completion pattern for the newly started lap.
            self.previous_sector_index = current_sector_index;

            let _ = DebugLogger::log_lap_start(
                self.current_lap_number,
                completed_laps,
                current_sector_index,
                last_sector_time,
                i_last_time,
            );

            // Update tracking
            self.previous_car_position = current_position;
            self.previous_completed_laps = completed_laps;

            // Mark that we just completed via line crossing to prevent sector-based
            // completion from triggering on the same event in the next update
            self.just_completed_via_crossing = true;

            return Some(lap_record);
        }

        // Clear the flag if we didn't cross a line (normal update)
        self.just_completed_via_crossing = false;

        // ========================================================================
        // STEP 3: DETECT SECTOR INDEX CHANGES
        // ========================================================================
        if current_sector_index != self.previous_sector_index {
            // Sector index changed - we have a sector time to record

            // Log the transition
            let _ = DebugLogger::log_sector_transition(
                self.current_lap_number,
                self.previous_sector_index,
                current_sector_index,
                last_sector_time,
            );

            // Track if we've ever seen sector 0
            if current_sector_index == 0 {
                self.has_seen_sector_zero = true;
            }

            // Only process sectors if we've seen sector 0 at least once
            if self.has_seen_sector_zero {
                if current_sector_index == 0
                    && self.previous_sector_index == 2
                    && self.lap_in_progress
                    && !self.just_completed_via_crossing
                {
                    // Lap boundary: moving from sector 2 to sector 0
                    // (but not if we just completed via line crossing to avoid duplicates)
                    // Record final sector of current lap
                    let final_sector = SectorTime {
                        index: 2,
                        time_ms: last_sector_time,
                        formatted: Self::format_time(last_sector_time),
                    };
                    let _ = DebugLogger::log_sector_recorded(
                        self.current_lap_number,
                        2,
                        last_sector_time,
                    );
                    self.current_lap_sectors.push(final_sector);

                    // Log the completed lap with its telemetry state
                    let _ = DebugLogger::log_telemetry_state(
                        completed_laps,
                        current_sector_index,
                        last_sector_time,
                        i_last_time,
                        self.previous_sector_index,
                        last_sector_time,
                        self.current_lap_sectors.len(),
                    );

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
                        lap_number: self.current_lap_number,
                        status,
                        total_time_ms: i_last_time,
                        total_time_formatted: Self::format_time(i_last_time),
                        sectors: self.current_lap_sectors.clone(),
                        timestamp: Utc::now().to_rfc3339(),
                    };

                    // Log the completed lap
                    let _ = DebugLogger::log_lap_completed(
                        self.current_lap_number,
                        i_last_time,
                        lap_record.sectors.len(),
                    );

                    // Reset for next lap
                    self.current_lap_number += 1;
                    self.current_lap_sectors.clear();
                    self.is_in_pit_during_lap = false;
                    self.lap_in_progress = true;

                    let _ = DebugLogger::log_lap_start(
                        self.current_lap_number,
                        completed_laps,
                        current_sector_index,
                        last_sector_time,
                        i_last_time,
                    );

                    // Update tracking
                    self.previous_sector_index = current_sector_index;
                    self.previous_last_sector_time = last_sector_time;
                    self.previous_car_position = current_position;
                    self.previous_completed_laps = completed_laps;

                    return Some(lap_record);
                } else if current_sector_index == 0 && !self.lap_in_progress {
                    // First time entering sector 0 - start lap 1
                    self.lap_in_progress = true;
                    self.current_lap_number = 1;

                    let _ = DebugLogger::log_lap_start(
                        self.current_lap_number,
                        completed_laps,
                        current_sector_index,
                        last_sector_time,
                        i_last_time,
                    );
                } else if self.lap_in_progress && current_sector_index > 0 {
                    // Normal sector transition within a lap (0→1 or 1→2)
                    // Record the sector we just left
                    let sector_just_left = current_sector_index - 1;
                    if self.previous_last_sector_time > 0 {
                        let sector = SectorTime {
                            index: sector_just_left as usize,
                            time_ms: self.previous_last_sector_time,
                            formatted: Self::format_time(self.previous_last_sector_time),
                        };
                        let _ = DebugLogger::log_sector_recorded(
                            self.current_lap_number,
                            sector_just_left as usize,
                            self.previous_last_sector_time,
                        );
                        self.current_lap_sectors.push(sector);
                    }
                }
            }

            // Update tracking
            self.previous_sector_index = current_sector_index;
            self.previous_last_sector_time = last_sector_time;
        } else if last_sector_time != self.previous_last_sector_time {
            // Sector time updated without index change
            self.previous_last_sector_time = last_sector_time;
        }

        // Update tracking
        self.previous_car_position = current_position;
        self.previous_completed_laps = completed_laps;

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
