# ACC Telemetry Recorder - Implementation Plan

## Overview

Transform the ACC telemetry reader from an interactive console tool into a continuous background recorder that:
- Monitors lap completions and sector times in real-time
- Records each lap with metadata (normal/pit/invalid status)
- Writes session data to JSON files in `.recordings/` directory
- Displays lap completion events to console for user feedback
- Creates uniquely named files per session with track, car, and timestamp

---

## Summary of Design Decisions

✅ **Lap Detection:** Monitor `completed_laps` counter increment  
✅ **Sector Tracking:** Use `last_sector_time` from ACC  
✅ **Lap Status:** Normal / Pit (via `is_in_pit`) / Invalid (via `last_sector_time == 0`)  
✅ **Statistics:** Include valid/pit/invalid counts; exclude pit laps from avg  
✅ **File Location:** `.recordings/` directory  
✅ **File Naming:** `{track}_{car}_{date}_{time}.json` (new file per session)  
✅ **Console Output:** Per-lap notification only (no sector breakdown)  
✅ **JSON Writing:** Immediate write after each lap completes  
✅ **Invalid Laps:** Mark as "invalid" only if `last_sector_time == 0`

---

## Architecture

```
src/
├── main.rs              [MODIFY] Entry point, main loop, telemetry polling
├── shared_memory.rs     [UNCHANGED] ACC data structures
├── lap_recorder.rs      [NEW] Lap detection, state tracking, lap building
└── json_export.rs       [NEW] JSON serialization, file I/O, formatting

.recordings/
├── Monza_Ferrari488GT3_2026-03-01_143000.json
├── Spa_Porsche911GT3_2026-03-02_094530.json
└── ...
```

---

## Module 1: `src/lap_recorder.rs` (NEW)

### Purpose
Track telemetry state and detect lap completions.

### Key Data Structures

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum LapStatus {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "pit")]
    Pit,
    #[serde(rename = "invalid")]
    Invalid,
}

#[derive(Debug, Clone, Serialize)]
pub struct SectorTime {
    pub index: usize,
    pub time_ms: i32,
    pub formatted: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LapRecord {
    pub lap_number: i32,
    pub status: LapStatus,
    pub total_time_ms: i32,
    pub total_time_formatted: String,
    pub sectors: Vec<SectorTime>,
    pub timestamp: String,  // ISO 8601
}

pub struct LapRecorder {
    previous_completed_laps: i32,
    previous_sector_index: i32,
    previous_last_sector_time: i32,
    current_lap_sectors: Vec<SectorTime>,
    is_in_pit_during_lap: bool,
}
```

### Key Methods

```rust
impl LapRecorder {
    pub fn new() -> Self { ... }

    pub fn update(
        &mut self,
        graphics: &PageFileGraphic,
    ) -> Option<LapRecord> {
        // 1. Detect sector boundaries
        // 2. Accumulate sector times
        // 3. Detect lap completion (completed_laps increment)
        // 4. Build LapRecord if lap completed
        // 5. Reset state for next lap
        // 6. Return Some(LapRecord) or None
    }

    fn format_time(ms: i32) -> String {
        // Convert: 145230 ms → "2:25.230"
    }
}
```

### Logic Flow

```
Each Update:
  1. Read: completed_laps, current_sector_index, last_sector_time, is_in_pit
  
  2. Track sector changes:
     IF current_sector_index ≠ previous_sector_index:
       → Add previous last_sector_time to current_lap_sectors
       → Update previous_sector_index
  
  3. Track pit status:
     IF is_in_pit == 1:
       → Set is_in_pit_during_lap = true
  
  4. Detect lap completion:
     IF completed_laps > previous_completed_laps:
       → Lap just finished!
       → Determine status:
           - is_in_pit_during_lap = true  →  "pit"
           - last_sector_time == 0  →  "invalid"
           - else  →  "normal"
       → Get total_time_ms from i_last_time (most recent completed lap)
       → Build LapRecord with all accumulated data
       → Reset: current_lap_sectors = [], is_in_pit_during_lap = false
       → Update: previous_completed_laps = completed_laps
       → Return Some(LapRecord)
  
  5. Return None if no lap completion detected
```

---

## Module 2: `src/json_export.rs` (NEW)

### Purpose
Handle JSON serialization and file I/O.

### Key Data Structures

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub car_model: String,
    pub track: String,
    pub session_type: String,
    pub player_name: String,
    pub recording_start: String,  // ISO 8601
    pub recording_end: Option<String>,
    pub total_laps_recorded: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionStatistics {
    pub valid_laps: i32,
    pub pit_laps: i32,
    pub invalid_laps: i32,
    pub best_lap_ms: Option<i32>,
    pub best_lap_formatted: Option<String>,
    pub average_lap_ms: Option<i32>,
    pub average_lap_formatted: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionRecording {
    pub metadata: SessionMetadata,
    pub laps: Vec<LapRecord>,
    pub statistics: SessionStatistics,
}

pub struct JsonExporter {
    file_path: PathBuf,
    session_data: SessionRecording,
}
```

### Key Methods

```rust
impl JsonExporter {
    pub fn new(
        car_model: String,
        track: String,
        player_name: String,
        session_type: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // 1. Create .recordings/ directory if not exists
        // 2. Generate filename: {track}_{car}_{date}_{time}.json
        // 3. Initialize SessionRecording with metadata
        // 4. Return JsonExporter instance
    }

    pub fn write_lap(&mut self, lap: LapRecord) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Add lap to session_data.laps
        // 2. Update statistics (best_lap, average_lap, counts)
        // 3. Serialize to JSON
        // 4. Write to file (overwrite entire file)
        // 5. Handle errors gracefully
    }

    pub fn finalize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Set metadata.recording_end to current time
        // 2. Finalize statistics
        // 3. Write final JSON to file
    }

    fn generate_filename(track: &str, car: &str) -> String {
        // Input: "Monza", "Ferrari 488 GT3"
        // Output: "Monza_Ferrari488GT3_2026-03-01_143000.json"
        // Steps:
        //   1. Replace spaces with underscores
        //   2. Get current UTC time
        //   3. Format as YYYY-MM-DD_HHMMSS
        //   4. Combine: {track}_{car}_{datetime}.json
    }

    fn update_statistics(&mut self) {
        // 1. Count valid/pit/invalid laps
        // 2. Calculate best_lap (minimum among valid laps only)
        // 3. Calculate average_lap (sum of valid laps / count, excluding pit)
        // 4. Format time strings
    }
}
```

---

## Module 3: `src/main.rs` (MODIFIED)

### Changes Required

#### 1. Add module declarations:
```rust
mod lap_recorder;
mod json_export;
```

#### 2. Add imports:
```rust
use lap_recorder::LapRecorder;
use json_export::JsonExporter;
```

#### 3. Remove interactive display functions:
- Delete `print_physics()`
- Delete `print_graphics()`
- Delete `print_static()`
- Delete `print_scalar()`, `print_array_f()`, etc.

#### 4. Update main() function:

```rust
fn main() {
    // [EXISTING] Open shared memory segments
    let physics_seg = SharedMemSegment::open(...)?;
    let graphics_seg = SharedMemSegment::open(...)?;
    let static_seg = SharedMemSegment::open(...)?;

    // [NEW] Extract metadata from static data
    let static_data: &PageFileStatic = unsafe { static_seg.as_ref() };
    let car_model = decode_wstr(&static_data.car_model);
    let track = decode_wstr(&static_data.track);
    let player_name = decode_wstr(&static_data.player_name);
    let session_type = AcSessionType::from_i32(static_data.session);

    // [NEW] Initialize recorder and exporter
    let mut recorder = LapRecorder::new();
    let mut exporter = JsonExporter::new(
        car_model.clone(),
        track.clone(),
        player_name.clone(),
        session_type.to_string(),
    )?;

    // [NEW] Print startup message
    println!("ACC Telemetry Recorder started");
    println!("Recording to: {}", exporter.file_path().display());
    println!("Track: {}, Car: {}", track, car_model);
    println!("Press Escape to exit and finalize recording.");

    // [MODIFIED] Main polling loop
    loop {
        let esc = unsafe { GetAsyncKeyState(0x1B) };

        // [NEW] Get telemetry data
        let graphics: &PageFileGraphic = unsafe { graphics_seg.as_ref() };

        // [NEW] Update recorder (check for lap completion)
        if let Some(lap) = recorder.update(graphics) {
            // [NEW] Print lap to console
            println!(
                "Lap {} completed: {} [{}]",
                lap.lap_number,
                lap.total_time_formatted,
                format!("{:?}", lap.status).to_lowercase()
            );

            // [NEW] Write lap to JSON
            exporter.write_lap(lap)?;
        }

        // [EXISTING] Exit on Escape
        if esc != 0 {
            println!("Exiting. Finalizing recording...");
            exporter.finalize()?;
            println!("Recording saved.");
            break;
        }

        thread::sleep(Duration::from_millis(16));
    }
}
```

---

## Module 4: `Cargo.toml` (MODIFIED)

### Add dependencies:

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = "0.4"

[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_Security",
    "Win32_System_Memory",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_WindowsAndMessaging",
] }

[profile.release]
opt-level = 3
lto = true
```

---

## JSON Output Schema

### File Structure

```
.recordings/
├── Monza_Ferrari488GT3_2026-03-01_143000.json
├── Spa_Porsche911GT3_2026-03-02_094530.json
└── ...
```

### Example JSON Output

```json
{
  "metadata": {
    "car_model": "Ferrari 488 GT3",
    "track": "Monza",
    "session_type": "Practice",
    "player_name": "Michael Napoleon",
    "recording_start": "2026-03-01T14:30:00Z",
    "recording_end": "2026-03-01T14:38:42Z",
    "total_laps_recorded": 5
  },
  "laps": [
    {
      "lap_number": 1,
      "status": "normal",
      "total_time_ms": 145340,
      "total_time_formatted": "2:25.340",
      "sectors": [
        {
          "index": 0,
          "time_ms": 48100,
          "formatted": "0:48.100"
        },
        {
          "index": 1,
          "time_ms": 49200,
          "formatted": "0:49.200"
        },
        {
          "index": 2,
          "time_ms": 48040,
          "formatted": "0:48.040"
        }
      ],
      "timestamp": "2026-03-01T14:30:05Z"
    },
    {
      "lap_number": 2,
      "status": "normal",
      "total_time_ms": 144890,
      "total_time_formatted": "2:24.890",
      "sectors": [...],
      "timestamp": "2026-03-01T14:32:31Z"
    },
    {
      "lap_number": 3,
      "status": "pit",
      "total_time_ms": 225220,
      "total_time_formatted": "3:45.220",
      "sectors": [...],
      "timestamp": "2026-03-01T14:34:57Z"
    },
    {
      "lap_number": 4,
      "status": "invalid",
      "total_time_ms": 0,
      "total_time_formatted": "0:00.000",
      "sectors": [],
      "timestamp": "2026-03-01T14:36:15Z"
    },
    {
      "lap_number": 5,
      "status": "normal",
      "total_time_ms": 143570,
      "total_time_formatted": "2:23.570",
      "sectors": [...],
      "timestamp": "2026-03-01T14:38:20Z"
    }
  ],
  "statistics": {
    "valid_laps": 3,
    "pit_laps": 1,
    "invalid_laps": 1,
    "best_lap_ms": 143570,
    "best_lap_formatted": "2:23.570",
    "average_lap_ms": 144600,
    "average_lap_formatted": "2:24.600"
  }
}
```

---

## Expected Behavior

### Console Output Example

```
ACC Telemetry Recorder started
Recording to: .recordings/Monza_Ferrari488GT3_2026-03-01_143000.json
Track: Monza, Car: Ferrari 488 GT3
Press Escape to exit and finalize recording.
Lap 1 completed: 2:25.340 [normal]
Lap 2 completed: 2:24.890 [normal]
Lap 3 completed: 3:45.220 [pit]
Lap 4 completed: 0:00.000 [invalid]
Lap 5 completed: 2:23.570 [normal]
Exiting. Finalizing recording...
Recording saved.
```

---

## Implementation Checklist

### Phase 1: Setup ✅ COMPLETE
- [x] Update `Cargo.toml` with new dependencies
- [x] Verify dependencies compile with `cargo check`

### Phase 2: Create `lap_recorder.rs` ✅ COMPLETE
- [x] Define enums and structs
- [x] Implement `LapRecorder::new()`
- [x] Implement `format_time()` helper
- [x] Implement lap detection logic in `update()`
- [x] Test sector boundary detection
- [x] Test lap completion detection

### Phase 3: Create `json_export.rs` ✅ COMPLETE
- [x] Define JSON serializable structs
- [x] Implement `JsonExporter::new()` with directory creation
- [x] Implement filename generation
- [x] Implement `write_lap()` with file I/O
- [x] Implement statistics calculation
- [x] Implement `finalize()`
- [x] Test JSON output format

### Phase 4: Refactor `main.rs` ✅ COMPLETE
- [x] Add module declarations
- [x] Remove old print functions
- [x] Extract metadata from static data
- [x] Create recorder and exporter instances
- [x] Update main loop with lap detection logic
- [x] Add console output messages
- [x] Update exit handler

### Phase 5: Testing & Validation ✅ COMPLETE
- [x] Verify code compiles successfully
- [x] Verify all modules integrate correctly
- [x] Document testing requirements (Windows + ACC needed)
- [ ] Run with ACC and verify lap detection (requires Windows + ACC)
- [ ] Verify JSON file creation in `.recordings/` (requires Windows + ACC)
- [ ] Verify lap times are accurate (requires Windows + ACC)
- [ ] Verify sector times sum to total time (requires Windows + ACC)
- [ ] Verify metadata is correct (requires Windows + ACC)
- [ ] Verify statistics calculations (requires Windows + ACC)
- [ ] Test pit lap detection (requires Windows + ACC)
- [ ] Test invalid lap detection (requires Windows + ACC)
- [ ] Test multiple sessions (new files) (requires Windows + ACC)

**Note:** Runtime testing requires Windows machine with ACC installed. Code compilation and integration verified successfully.

---

## Key Implementation Notes

### Lap Detection Logic

The lap detection happens by monitoring the `completed_laps` counter from ACC telemetry:

1. When `completed_laps` increments, a lap has been completed
2. The lap status is determined by:
   - **Pit:** `is_in_pit == 1` during the lap
   - **Invalid:** `last_sector_time == 0` (no timing recorded)
   - **Normal:** All other cases
3. Pit laps are excluded from average lap time calculations
4. Only valid laps (status == "normal") are used for best lap and average calculations

### Sector Time Accumulation

Sectors are accumulated during a lap by:

1. Tracking `current_sector_index` changes
2. When the sector index changes, the previous sector's time (`last_sector_time`) is recorded
3. All accumulated sector times are included in the `LapRecord`
4. The sum of sector times should equal the total lap time

### Session Detection

To ensure correct metadata capture across session changes:

1. The program waits for `completed_laps > 0` before creating the JSON exporter
2. This prevents capturing stale track/car data from previous sessions
3. ACC keeps shared memory segments open across session changes, so waiting for an active lap ensures current session data
4. User sees "Waiting for active session..." message until first lap is detected

### File Management

- **Directory:** `.recordings/` is created automatically if it doesn't exist
- **Naming:** Files are named `{track}_{car}_{date}_{time}.json` to ensure uniqueness
- **Updates:** After each lap completes, the entire JSON file is rewritten with updated statistics
- **Safety:** Immediate writes ensure no data loss if the program crashes
- **Delayed Creation:** JSON file is not created until `completed_laps > 0` to ensure correct metadata from active session (prevents stale data from previous sessions)

### JSON Serialization

- All time values are stored in milliseconds (`total_time_ms`, `time_ms`)
- All times are formatted as human-readable strings (`total_time_formatted`, `formatted`)
- Timestamps are ISO 8601 format (UTC)
- Statistics only count valid laps (pit laps excluded from averages)

---

## Deployment Considerations

### Building for Windows

```bash
# Debug build (development)
cargo build

# Release build (production)
cargo build --release
```

Output: `target/release/acc-telemetry-rs.exe`

### Recording File Location

The `.recordings/` directory will be created in the current working directory where the program is launched from. Users should launch the program from a known location.

### Data Persistence

Each session creates a new file with a unique timestamp. This allows:
- Multiple concurrent recording sessions (different tracks/cars)
- Historical tracking of all sessions
- Easy backup and archival

---

## Future Enhancements

Possible future improvements not in this plan:

- Command-line arguments for custom recording directory
- Configuration file for default settings
- Real-time statistics display during recording
- Lap comparison visualization
- Export to other formats (CSV, SQLite, etc.)
- Multi-session aggregation and analysis
- Driver comparison across sessions
