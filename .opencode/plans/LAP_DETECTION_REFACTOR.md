# ACC Telemetry Recorder - Lap Detection Refactor Plan

**Objective:** Refactor lap and sector detection to use `normalized_car_position` for more reliable start/finish line crossing detection, enabling accurate capture of all sector times including the final sector.

**Status:** Ready for Implementation

---

## Problem Statement

Current implementation has timing issues where:
- **Final sector times are not reliably captured** - The final sector transition happens after the lap completion is detected
- **Lap 1 has no sectors** - First lap isn't tracked because we don't know when recording started relative to lap start
- **Race condition** - The `completed_laps` counter increments before the sector index wraps from last sector back to 0

**Root Cause:** Using `completed_laps` counter to detect lap completion creates a race condition where:
1. ACC increments `completed_laps` when crossing the finish line
2. At that moment, `current_sector_index` is still at the last sector (hasn't wrapped to 0 yet)
3. Our code detects lap completion and returns immediately
4. On the next update cycle, the sector finally transitions and is lost

---

## Solution: `normalized_car_position` Based Detection

### Key Insight

ACC's `PageFileGraphic` struct contains `normalized_car_position: f32` which represents the car's position on the track as a normalized value (0.0 to 1.0).

**Start/Finish Line Indicator:**
- When car crosses from end of track (0.9+) back to start (0.0+), we've crossed the line
- This happens **more reliably and earlier** than waiting for `completed_laps` to increment
- We can capture all sectors **before** the lap counter increments

### Advantages

✅ **Precise timing** - Know exactly when crossing happens
✅ **Complete sector capture** - Can wait for final sector transition to complete
✅ **Works from lap 1** - No need to wait for `completed_laps > 0`
✅ **Matches 50Hz game update rate** - Telemetry is updated every 20ms, our polling should align
✅ **Cleaner state machine** - No race conditions
✅ **Detects session start** - Can record lap 1 from the beginning

---

## Implementation Design

### 1. New Fields in `LapRecorder`

Add to the `LapRecorder` struct in `src/lap_recorder.rs`:

```rust
pub struct LapRecorder {
    // Existing fields (keep all of these)
    previous_completed_laps: i32,
    previous_sector_index: i32,
    previous_last_sector_time: i32,
    current_lap_sectors: Vec<SectorTime>,
    is_in_pit_during_lap: bool,
    current_lap_number: i32,
    
    // NEW: Car position tracking for lap boundary detection
    previous_car_position: f32,      // Last known normalized_car_position
    lap_in_progress: bool,           // Flag: are we actively in a lap?
}
```

### 2. Detection Logic: Start/Finish Line Crossing

Add this helper method to `LapRecorder`:

```rust
impl LapRecorder {
    /// Detect if the car crossed the start/finish line
    /// by checking if normalized_car_position wrapped from high (>0.5) to low (<0.5)
    fn detect_start_finish_crossing(
        current_position: f32,
        previous_position: f32,
    ) -> bool {
        // Wrap-around detection: previous was near end of track, current is near start
        // Example: 0.95 -> 0.05 indicates crossing the line
        if previous_position > 0.5 && current_position < 0.5 {
            return true;
        }
        false
    }
}
```

### 3. Constructor Changes

Update `LapRecorder::new()` to initialize new fields:

```rust
pub fn new() -> Self {
    Self {
        // Existing initializations
        previous_completed_laps: 0,
        previous_sector_index: -1,
        previous_last_sector_time: 0,
        current_lap_sectors: Vec::new(),
        is_in_pit_during_lap: false,
        current_lap_number: 0,
        
        // NEW:
        previous_car_position: 0.0,
        lap_in_progress: false,
    }
}
```

### 4. Main Update Logic Refactor

The `update()` method should follow this order (CRITICAL: ORDER MATTERS):

**STEP 1: Detect Start/Finish Line Crossing (PRIMARY LAP BOUNDARY)**
- Check if `normalized_car_position` wrapped around 0/1 boundary
- If crossed AND lap is in progress:
  - Record the final sector immediately
  - Build lap record with all accumulated sectors
  - Return the lap record
  - Reset state for next lap
  - Mark lap as NOT in progress

**STEP 2: Detect Sector Boundaries (WITHIN LAP)**
- If `current_sector_index` changed:
  - Record the previous sector time
  - Update sector tracking
- This only happens if we didn't return a lap in STEP 1

**STEP 3: Initialize New Lap When Not In Progress**
- If lap is NOT in progress AND we have valid telemetry:
  - Mark lap as in progress
  - Initialize new lap number from `completed_laps + 1`
  - Log lap start

**STEP 4: Update Position Tracking**
- Always update: `previous_car_position = current_position`
- Always update: `previous_completed_laps = completed_laps`

### 5. Polling Rate Alignment

Current polling is ~60Hz (`thread::sleep(Duration::from_millis(16))`), but ACC updates at 50Hz (20ms).

**Change in `main.rs`:**
```rust
// In the main polling loop (around line 240-245):
// OLD:
thread::sleep(Duration::from_millis(16));

// NEW:
thread::sleep(Duration::from_millis(20));  // Match game's 50Hz update rate
```

This ensures we read every telemetry update from ACC without missing any.

---

## Implementation Checklist

### Phase 1: Prepare LapRecorder Struct (5 min)
- [ ] Add `previous_car_position: f32` field
- [ ] Add `lap_in_progress: bool` field
- [ ] Update `new()` to initialize both to `0.0` and `false`

### Phase 2: Add Helper Method (5 min)
- [ ] Add `detect_start_finish_crossing()` method to `LapRecorder` impl

### Phase 3: Refactor update() Method (20-30 min)
- [ ] STEP 1: Add start/finish crossing detection at beginning
- [ ] STEP 1: When crossing detected, record final sector and return lap
- [ ] STEP 2: Move sector boundary detection logic (should already exist, just reorder)
- [ ] STEP 3: Add lap initialization logic
- [ ] STEP 4: Always update position tracking at end
- [ ] Update all debug logging calls

### Phase 4: Polling Rate Optimization (2 min)
- [ ] Change `thread::sleep(Duration::from_millis(16))` to `20` in main.rs

### Phase 5: Compile & Test (5 min)
- [ ] Run `cargo build --release`
- [ ] Verify no compilation errors
- [ ] Test with existing 3-lap recording file
- [ ] Verify debug logs show clean timeline

---

## Expected Results After Implementation

### Before (Current Behavior)
```
Lap 1: 0 sectors       ← First lap has no sectors
Lap 2: 3 sectors       ← Complete
Lap 3: 3 sectors       ← Complete

Debug log shows race conditions and out-of-order events
```

### After (Expected Behavior)
```
Lap 1: 3 sectors       ← First lap NOW has sectors (if lap completed)
Lap 2: 3 sectors       ← Complete
Lap 3: 3 sectors       ← Complete

Debug log shows:
  - Sector transitions in clean order
  - Final sector recorded before lap completion
  - No race conditions
```

---

## Key Changes Summary

| Aspect | Current | New |
|--------|---------|-----|
| **Lap Boundary Detection** | `completed_laps` increment | `normalized_car_position` wrap |
| **Final Sector Capture** | Lost (race condition) | Captured when crossing detected |
| **Lap 1 Recording** | Skipped until lap 1 completes | Started immediately |
| **Polling Rate** | 60Hz (16ms) | 50Hz (20ms) |
| **State Machine** | Linear | Explicit lap state flag |
| **Primary Telemetry** | `completed_laps` counter | `normalized_car_position` value |

---

## Edge Cases Handled

### Edge Case 1: Very Fast Lap Crossing
Position wrapping detection will catch it in one update

### Edge Case 2: Pit Stop During Lap
`is_in_pit` flag still tracked, lap marked as "pit"

### Edge Case 3: Invalid Lap
`lap_time == 0` detection still works

### Edge Case 4: Session Start (Lap 1)
`lap_in_progress` starts false, gets set when we detect active telemetry

### Edge Case 5: Paused Session
Position won't change, so no crossing detected (correct)

---

## Files to Modify

1. **`src/lap_recorder.rs`** - Main refactoring
   - Add struct fields
   - Add helper method
   - Refactor `update()` method
   - Update debug logging

2. **`src/main.rs`** - Polling rate adjustment
   - Change sleep duration from 16ms to 20ms

---

## Verification Checklist

After implementation, verify:

- [ ] Code compiles with `cargo build --release`
- [ ] No warning messages
- [ ] Can run on Windows with ACC
- [ ] First lap now has all sector data
- [ ] All laps have correct sector count
- [ ] Sector times in correct order
- [ ] Debug log shows clean timeline
- [ ] Pit laps still detected
- [ ] Invalid laps still detected
- [ ] Recording files created successfully

---

## Rollback Plan

If issues arise during implementation:
1. Revert `src/lap_recorder.rs` to last known good state
2. Revert `src/main.rs` polling rate change
3. No data loss - JSON files are independent of code changes
4. Can easily switch back and forth for testing

---

## Questions Before Starting

Before beginning implementation, confirm:

1. ✅ Is the approach using `normalized_car_position` acceptable?
2. ✅ Is delaying lap recording by 1-2 frames acceptable?
3. ✅ Can the polling rate be safely changed to 20ms?

**All confirmed - Ready to implement.**

