# ACC Telemetry Recorder - Usage Guide

## Overview

The ACC Telemetry Recorder automatically captures lap times and sector splits from Assetto Corsa Competizione and saves them to JSON files for later analysis.

---

## Quick Start

### Prerequisites

1. **Windows 10 or later** (Windows-only application)
2. **Assetto Corsa Competizione** installed and running
3. **Rust toolchain** (if building from source)

### Building the Application

On a Windows machine with Rust installed:

```powershell
# Clone the repository
git clone <repository-url> acc-telemetry-rs
cd acc-telemetry-rs

# Build release binary
cargo build --release

# The executable will be at:
# target\release\acc-telemetry-rs.exe
```

---

## Running the Recorder

### Step 1: Launch ACC

Start Assetto Corsa Competizione and load into any session (Practice, Qualifying, Race, etc.).

**Important:** The game must be running **before** you start the recorder, as it needs to access ACC's shared memory.

### Step 2: Run the Recorder

Open a Command Prompt or PowerShell and run:

```powershell
.\target\release\acc-telemetry-rs.exe
```

### Step 3: Start Driving

You'll see:

```
╔════════════════════════════════════════════════════════════════╗
║          ACC Telemetry Recorder - Lap Time Logger            ║
╚════════════════════════════════════════════════════════════════╝

Waiting for active session (monitoring for completed_laps > 0)...
Press Escape to exit.
═══════════════════════════════════════════════════════════════
```

Once you **complete your first lap**, the recorder will detect the active session:

```
Active session detected!

Session Info:
  Track:   Monza
  Car:     Ferrari 488 GT3
  Player:  Michael Napoleon
  Session: Practice

Recording to: .recordings/Monza_Ferrari488GT3_2026-03-01_143000.json

═══════════════════════════════════════════════════════════════

Lap 1 completed: 2:25.340 [normal]
Lap 2 completed: 2:24.890 [normal]
Lap 3 completed: 3:45.220 [pit]
```

### Step 4: Exit When Done

Press **Escape** to stop recording:

```
═══════════════════════════════════════════════════════════════
Exiting. Finalizing recording...
Recording saved to: .recordings/Monza_Ferrari488GT3_2026-03-01_143000.json
Thank you for using ACC Telemetry Recorder!
```

---

## Output Files

### Location

All recordings are saved to the `.recordings/` directory in the same location as the executable.

### File Naming

Files are automatically named with the pattern:

```
{track}_{car}_{date}_{time}.json
```

Examples:
- `Monza_Ferrari488GT3_2026-03-01_143000.json`
- `Spa-Francorchamps_Porsche911GT3R_2026-03-02_094530.json`

### JSON Structure

Each file contains:

```json
{
  "metadata": {
    "car_model": "Ferrari 488 GT3",
    "track": "Monza",
    "session_type": "Practice",
    "player_name": "Michael Napoleon",
    "recording_start": "2026-03-01T14:30:00Z",
    "recording_end": "2026-03-01T14:45:00Z",
    "total_laps_recorded": 15
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
    }
  ],
  "statistics": {
    "valid_laps": 13,
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

## Understanding Lap Status

Each lap is classified as one of three types:

### Normal
- Regular racing lap with valid timing
- Included in statistics (best lap, average)
- Example: `Lap 1 completed: 2:25.340 [normal]`

### Pit
- Lap where car entered the pit lane
- **Excluded** from average lap time calculations
- Still recorded for completeness
- Example: `Lap 3 completed: 3:45.220 [pit]`

### Invalid
- Lap with no timing recorded (`last_sector_time == 0`)
- Usually caused by crashes, incomplete laps, or session resets
- Excluded from statistics
- Example: `Lap 4 completed: 0:00.000 [invalid]`

---

## Session Changes

The recorder is designed to handle session changes gracefully:

### How It Works

1. Recorder waits for `completed_laps > 0` before capturing metadata
2. This ensures track/car information is from the **current** session, not a previous one
3. If you change tracks or cars in ACC, simply **restart the recorder** to create a new file

### Example Workflow

```
1. Practice at Monza with Ferrari → monza_ferrari_143000.json
2. Change to Spa with Porsche (don't close ACC)
3. Exit recorder (Escape)
4. Restart recorder
5. Drive at Spa → spa_porsche_150000.json (new file!)
```

---

## Troubleshooting

### "Failed to open graphics/static segment"

**Cause:** ACC is not running or hasn't created shared memory segments yet.

**Fix:**
1. Launch ACC
2. Start a session (Practice, Qualifying, Race, etc.)
3. Then run the recorder

### "Waiting for active session..." never completes

**Cause:** You haven't completed a lap yet.

**Fix:** Drive and complete at least one lap. The recorder activates on the first lap completion.

### All lap times show as 0:00.000 [invalid]

**Cause:** ACC is not providing valid timing data, possibly in Replay mode or session hasn't started properly.

**Fix:**
1. Ensure you're in an active session (not Replay)
2. Complete a full lap crossing the start/finish line
3. Check that ACC's own timing shows valid lap times

### File not created / No data in .recordings/

**Cause:** You exited before completing any laps.

**Fix:** The JSON file is only created after detecting `completed_laps > 0`. Drive at least one lap before exiting.

### Antivirus blocking the .exe

**Cause:** Unsigned binary triggers heuristic detection.

**Fix:**
1. Add exception in antivirus settings
2. Or build from source yourself (trusted compilation)

---

## Advanced Usage

### Analyzing JSON Data

The JSON files can be imported into:
- **Python/Pandas** for data analysis
- **Excel** for visualization
- **Custom tools** for telemetry comparison

Example Python snippet:

```python
import json

with open('.recordings/Monza_Ferrari488GT3_2026-03-01_143000.json') as f:
    data = json.load(f)

# Get best lap time
best_lap = data['statistics']['best_lap_formatted']
print(f"Best lap: {best_lap}")

# Analyze sector consistency
for lap in data['laps']:
    if lap['status'] == 'normal':
        print(f"Lap {lap['lap_number']}: {lap['total_time_formatted']}")
        for sector in lap['sectors']:
            print(f"  Sector {sector['index']}: {sector['formatted']}")
```

### Continuous Recording

To record an entire session:

1. Start recorder **after** ACC is running
2. Leave it running during entire session
3. Press Escape when session ends
4. File is automatically finalized with end timestamp

### Multiple Stints

Each time you restart the recorder, a new JSON file is created. This is useful for:
- Different cars on same track
- Different tracks with same car
- Separate practice sessions

---

## Tips for Best Results

✅ **Start recorder after ACC is running** - Shared memory must exist  
✅ **Complete at least one lap** - Recording activates on first lap  
✅ **Leave running during session** - Captures all laps automatically  
✅ **Press Escape to exit** - Ensures proper file finalization  
✅ **Restart for new sessions** - Each session gets its own file  

---

## Technical Details

- **Polling Rate:** ~60 Hz (every 16ms)
- **Memory Usage:** Minimal (only lap data stored)
- **File Format:** Pretty-printed JSON
- **Timestamps:** ISO 8601 UTC
- **Sector Support:** Works with any number of sectors (2, 3, 4+)

---

## Support

For issues, feature requests, or questions:
- Check `IMPLEMENTATION_PLAN.md` for technical details
- Review `README.md` for build instructions
- Check ACC shared memory is enabled (should be by default)

---

## License & Credits

Based on ACC shared memory SDK. Rust implementation for telemetry recording and analysis.
