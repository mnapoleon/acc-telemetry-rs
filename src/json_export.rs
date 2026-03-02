/// JSON export functionality for ACC telemetry sessions.
///
/// This module handles serialization of lap records to JSON format,
/// file I/O operations, and statistics calculation.
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::lap_recorder::{LapRecord, LapStatus};

// ---------------------------------------------------------------------------
// JSON Data Structures
// ---------------------------------------------------------------------------

/// Metadata about the recording session.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Car model name (e.g., "Ferrari 488 GT3")
    pub car_model: String,
    /// Track name (e.g., "Monza")
    pub track: String,
    /// Session type (e.g., "Practice", "Race")
    pub session_type: String,
    /// Player name
    pub player_name: String,
    /// ISO 8601 timestamp when recording started
    pub recording_start: String,
    /// ISO 8601 timestamp when recording ended (None until finalized)
    pub recording_end: Option<String>,
    /// Total number of laps recorded
    pub total_laps_recorded: i32,
}

/// Session statistics (best lap, averages, counts).
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionStatistics {
    /// Number of valid (normal) laps
    pub valid_laps: i32,
    /// Number of pit laps
    pub pit_laps: i32,
    /// Number of invalid laps
    pub invalid_laps: i32,
    /// Best lap time in milliseconds (valid laps only)
    pub best_lap_ms: Option<i32>,
    /// Best lap time formatted (valid laps only)
    pub best_lap_formatted: Option<String>,
    /// Average lap time in milliseconds (valid laps only, excluding pit)
    pub average_lap_ms: Option<i32>,
    /// Average lap time formatted (valid laps only, excluding pit)
    pub average_lap_formatted: Option<String>,
}

/// Complete session recording with metadata, laps, and statistics.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionRecording {
    /// Session metadata
    pub metadata: SessionMetadata,
    /// All recorded laps
    pub laps: Vec<LapRecord>,
    /// Computed statistics
    pub statistics: SessionStatistics,
}

// ---------------------------------------------------------------------------
// JSON Exporter
// ---------------------------------------------------------------------------

/// Manages JSON file export for a telemetry session.
pub struct JsonExporter {
    /// Path to the JSON output file
    file_path: PathBuf,
    /// Session data being recorded
    session_data: SessionRecording,
}

impl JsonExporter {
    /// Create a new JSON exporter for a session.
    ///
    /// Creates the `.recordings/` directory if it doesn't exist,
    /// generates a unique filename based on track, car, and timestamp,
    /// and initializes the session metadata.
    pub fn new(
        car_model: String,
        track: String,
        player_name: String,
        session_type: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Create .recordings directory if it doesn't exist
        let recordings_dir = PathBuf::from(".recordings");
        if !recordings_dir.exists() {
            fs::create_dir_all(&recordings_dir)?;
        }

        // Generate unique filename
        let filename = Self::generate_filename(&track, &car_model);
        let file_path = recordings_dir.join(filename);

        // Initialize session metadata
        let metadata = SessionMetadata {
            car_model,
            track,
            session_type,
            player_name,
            recording_start: Utc::now().to_rfc3339(),
            recording_end: None,
            total_laps_recorded: 0,
        };

        // Initialize statistics
        let statistics = SessionStatistics {
            valid_laps: 0,
            pit_laps: 0,
            invalid_laps: 0,
            best_lap_ms: None,
            best_lap_formatted: None,
            average_lap_ms: None,
            average_lap_formatted: None,
        };

        // Initialize session recording
        let session_data = SessionRecording {
            metadata,
            laps: Vec::new(),
            statistics,
        };

        Ok(Self {
            file_path,
            session_data,
        })
    }

    /// Get the file path where data is being written.
    pub fn file_path(&self) -> &PathBuf {
        &self.file_path
    }

    /// Write a completed lap to the JSON file.
    ///
    /// Adds the lap to the session, updates statistics, and writes
    /// the entire updated session to the JSON file.
    pub fn write_lap(&mut self, lap: LapRecord) -> Result<(), Box<dyn std::error::Error>> {
        // Add lap to session
        self.session_data.laps.push(lap);

        // Update metadata
        self.session_data.metadata.total_laps_recorded = self.session_data.laps.len() as i32;

        // Recalculate statistics
        self.update_statistics();

        // Write to file
        self.write_to_file()?;

        Ok(())
    }

    /// Finalize the recording session.
    ///
    /// Sets the recording_end timestamp and writes the final JSON file.
    pub fn finalize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Set end timestamp
        self.session_data.metadata.recording_end = Some(Utc::now().to_rfc3339());

        // Write final version to file
        self.write_to_file()?;

        Ok(())
    }

    /// Generate a filename based on track, car, and current timestamp.
    ///
    /// Format: `{track}_{car}_{date}_{time}.json`
    /// Example: `Monza_Ferrari488GT3_2026-03-01_143000.json`
    fn generate_filename(track: &str, car: &str) -> String {
        // Clean track and car names (remove spaces, special chars)
        let clean_track = track.replace(' ', "").replace('/', "_");
        let clean_car = car.replace(' ', "").replace('/', "_");

        // Get current timestamp
        let now = Utc::now();
        let date = now.format("%Y-%m-%d").to_string();
        let time = now.format("%H%M%S").to_string();

        format!("{}_{}_{}_{}. json", clean_track, clean_car, date, time)
    }

    /// Update session statistics based on current lap data.
    fn update_statistics(&mut self) {
        let mut valid_laps = 0;
        let mut pit_laps = 0;
        let mut invalid_laps = 0;
        let mut valid_lap_times: Vec<i32> = Vec::new();

        // Count laps by status and collect valid lap times
        for lap in &self.session_data.laps {
            match lap.status {
                LapStatus::Normal => {
                    valid_laps += 1;
                    if lap.total_time_ms > 0 {
                        valid_lap_times.push(lap.total_time_ms);
                    }
                }
                LapStatus::Pit => {
                    pit_laps += 1;
                }
                LapStatus::Invalid => {
                    invalid_laps += 1;
                }
            }
        }

        // Calculate best lap (minimum time among valid laps)
        let best_lap_ms = valid_lap_times.iter().min().copied();
        let best_lap_formatted = best_lap_ms.map(Self::format_time);

        // Calculate average lap time (valid laps only, excluding pit)
        let (average_lap_ms, average_lap_formatted) = if !valid_lap_times.is_empty() {
            let sum: i32 = valid_lap_times.iter().sum();
            let avg = sum / valid_lap_times.len() as i32;
            (Some(avg), Some(Self::format_time(avg)))
        } else {
            (None, None)
        };

        // Update statistics
        self.session_data.statistics = SessionStatistics {
            valid_laps,
            pit_laps,
            invalid_laps,
            best_lap_ms,
            best_lap_formatted,
            average_lap_ms,
            average_lap_formatted,
        };
    }

    /// Write the current session data to the JSON file.
    fn write_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Serialize to JSON with pretty formatting
        let json = serde_json::to_string_pretty(&self.session_data)?;

        // Write to file
        fs::write(&self.file_path, json)?;

        Ok(())
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time() {
        assert_eq!(JsonExporter::format_time(0), "0:00.000");
        assert_eq!(JsonExporter::format_time(1000), "0:01.000");
        assert_eq!(JsonExporter::format_time(48100), "0:48.100");
        assert_eq!(JsonExporter::format_time(60000), "1:00.000");
        assert_eq!(JsonExporter::format_time(145230), "2:25.230");
        assert_eq!(JsonExporter::format_time(225220), "3:45.220");
    }

    #[test]
    fn test_generate_filename() {
        let filename = JsonExporter::generate_filename("Monza", "Ferrari 488 GT3");
        assert!(filename.starts_with("Monza_Ferrari488GT3_"));
        assert!(filename.ends_with(".json"));
    }

    #[test]
    fn test_clean_filename() {
        let filename = JsonExporter::generate_filename("Spa-Francorchamps", "Porsche 911 GT3");
        assert!(filename.contains("Spa-Francorchamps"));
        assert!(filename.contains("Porsche911GT3"));
        assert!(!filename.contains(' '));
    }
}
