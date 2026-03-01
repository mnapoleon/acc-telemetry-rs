/// Shared memory page layouts for Assetto Corsa Competizione.
///
/// The binary layout of every struct here **must** match the ACC SDK exactly.
/// Field order and sizes must not be changed. New SDK fields must be appended
/// at the end of the relevant struct.
///
/// All structs use `#[repr(C, align(4))]` to match the C `#pragma pack(4)`
/// alignment used in the original C++ header.
///
/// Wide-string fields in the C++ structs are `wchar_t[]` arrays (UTF-16LE on
/// Windows, 2 bytes per code unit).  They are represented here as fixed-size
/// `[u16; N]` arrays with the same element count as the original.
use std::fmt;

// ---------------------------------------------------------------------------
// Enums and status constants
// ---------------------------------------------------------------------------

/// All penalty types that ACC can issue.
#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PenaltyShortcut {
    None = 0,
    DriveThroughCutting = 1,
    StopAndGo10Cutting = 2,
    StopAndGo20Cutting = 3,
    StopAndGo30Cutting = 4,
    DisqualifiedCutting = 5,
    RemoveBestLaptimeCutting = 6,
    DriveThroughPitSpeeding = 7,
    StopAndGo10PitSpeeding = 8,
    StopAndGo20PitSpeeding = 9,
    StopAndGo30PitSpeeding = 10,
    DisqualifiedPitSpeeding = 11,
    RemoveBestLaptimePitSpeeding = 12,
    DisqualifiedIgnoredMandatoryPit = 13,
    PostRaceTime = 14,
    DisqualifiedTrolling = 15,
    DisqualifiedPitEntry = 16,
    DisqualifiedPitExit = 17,
    DisqualifiedWrongWay = 18,
    DriveThroughIgnoredDriverStint = 19,
    DisqualifiedIgnoredDriverStint = 20,
    DisqualifiedExceededDriverStintLimit = 21,
}

impl PenaltyShortcut {
    #[allow(dead_code)]
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::DriveThroughCutting,
            2 => Self::StopAndGo10Cutting,
            3 => Self::StopAndGo20Cutting,
            4 => Self::StopAndGo30Cutting,
            5 => Self::DisqualifiedCutting,
            6 => Self::RemoveBestLaptimeCutting,
            7 => Self::DriveThroughPitSpeeding,
            8 => Self::StopAndGo10PitSpeeding,
            9 => Self::StopAndGo20PitSpeeding,
            10 => Self::StopAndGo30PitSpeeding,
            11 => Self::DisqualifiedPitSpeeding,
            12 => Self::RemoveBestLaptimePitSpeeding,
            13 => Self::DisqualifiedIgnoredMandatoryPit,
            14 => Self::PostRaceTime,
            15 => Self::DisqualifiedTrolling,
            16 => Self::DisqualifiedPitEntry,
            17 => Self::DisqualifiedPitExit,
            18 => Self::DisqualifiedWrongWay,
            19 => Self::DriveThroughIgnoredDriverStint,
            20 => Self::DisqualifiedIgnoredDriverStint,
            21 => Self::DisqualifiedExceededDriverStintLimit,
            _ => Self::None,
        }
    }
}

impl fmt::Display for PenaltyShortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Simulator status (`AC_STATUS` in the C++ header).
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcStatus {
    Off = 0,
    Replay = 1,
    Live = 2,
    Pause = 3,
}

impl AcStatus {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Replay,
            2 => Self::Live,
            3 => Self::Pause,
            _ => Self::Off,
        }
    }
}

impl fmt::Display for AcStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Session type (`AC_SESSION_TYPE` in the C++ header).
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcSessionType {
    Unknown = -1,
    Practice = 0,
    Qualify = 1,
    Race = 2,
    Hotlap = 3,
    TimeAttack = 4,
    Drift = 5,
    Drag = 6,
    HotStint = 7,
    HotlapSuperpole = 8,
}

impl AcSessionType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Self::Practice,
            1 => Self::Qualify,
            2 => Self::Race,
            3 => Self::Hotlap,
            4 => Self::TimeAttack,
            5 => Self::Drift,
            6 => Self::Drag,
            7 => Self::HotStint,
            8 => Self::HotlapSuperpole,
            _ => Self::Unknown,
        }
    }
}

impl fmt::Display for AcSessionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Flag type (`AC_FLAG_TYPE` in the C++ header).
#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcFlagType {
    NoFlag = 0,
    BlueFlag = 1,
    YellowFlag = 2,
    BlackFlag = 3,
    WhiteFlag = 4,
    CheckeredFlag = 5,
    PenaltyFlag = 6,
}

impl AcFlagType {
    #[allow(dead_code)]
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::BlueFlag,
            2 => Self::YellowFlag,
            3 => Self::BlackFlag,
            4 => Self::WhiteFlag,
            5 => Self::CheckeredFlag,
            6 => Self::PenaltyFlag,
            _ => Self::NoFlag,
        }
    }
}

impl fmt::Display for AcFlagType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// ---------------------------------------------------------------------------
// Helper: decode a null-terminated UTF-16LE fixed array to a Rust String
// ---------------------------------------------------------------------------

/// Decode a fixed-size `[u16; N]` null-terminated UTF-16 array to a `String`.
pub fn decode_wstr<const N: usize>(buf: &[u16; N]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(N);
    String::from_utf16_lossy(&buf[..end])
}

// ---------------------------------------------------------------------------
// Shared memory page structs
// ---------------------------------------------------------------------------
//
// IMPORTANT: These structs map byte-for-byte onto the ACC shared memory
// segments.  The layout uses C-compatible representation with 4-byte
// alignment (`#[repr(C, align(4))]`), matching the original
// `#pragma pack(push) / #pragma pack(4)` directives.
//
// Do NOT reorder fields.
// Do NOT change field types or array sizes.
// New SDK fields must be appended at the end.

/// Maps to `Local\acpmf_physics` — real-time vehicle physics.
#[repr(C, align(4))]
pub struct PageFilePhysics {
    pub packet_id: i32,
    pub gas: f32,
    pub brake: f32,
    pub fuel: f32,
    pub gear: i32,
    pub rpms: i32,
    pub steer_angle: f32,
    pub speed_kmh: f32,
    pub velocity: [f32; 3],
    pub acc_g: [f32; 3],
    pub wheel_slip: [f32; 4],
    pub wheel_load: [f32; 4],
    pub wheels_pressure: [f32; 4],
    pub wheel_angular_speed: [f32; 4],
    pub tyre_wear: [f32; 4],
    pub tyre_dirty_level: [f32; 4],
    pub tyre_core_temperature: [f32; 4],
    pub camber_rad: [f32; 4],
    pub suspension_travel: [f32; 4],
    pub drs: f32,
    pub tc: f32,
    pub heading: f32,
    pub pitch: f32,
    pub roll: f32,
    pub cg_height: f32,
    pub car_damage: [f32; 5],
    pub number_of_tyres_out: i32,
    pub pit_limiter_on: i32,
    pub abs: f32,
    pub kers_charge: f32,
    pub kers_input: f32,
    pub auto_shifter_on: i32,
    pub ride_height: [f32; 2],
    pub turbo_boost: f32,
    pub ballast: f32,
    pub air_density: f32,
    pub air_temp: f32,
    pub road_temp: f32,
    pub local_angular_vel: [f32; 3],
    pub final_ff: f32,
    pub performance_meter: f32,
    pub engine_brake: i32,
    pub ers_recovery_level: i32,
    pub ers_power_level: i32,
    pub ers_heat_charging: i32,
    pub ers_is_charging: i32,
    pub kers_current_kj: f32,
    pub drs_available: i32,
    pub drs_enabled: i32,
    pub brake_temp: [f32; 4],
    pub clutch: f32,
    pub tyre_temp_i: [f32; 4],
    pub tyre_temp_m: [f32; 4],
    pub tyre_temp_o: [f32; 4],
    pub is_ai_controlled: i32,
    pub tyre_contact_point: [[f32; 3]; 4],
    pub tyre_contact_normal: [[f32; 3]; 4],
    pub tyre_contact_heading: [[f32; 3]; 4],
    pub brake_bias: f32,
    pub local_velocity: [f32; 3],
    pub p2p_activations: i32,
    pub p2p_status: i32,
    pub current_max_rpm: i32,
    pub mz: [f32; 4],
    pub fx: [f32; 4],
    pub fy: [f32; 4],
    pub slip_ratio: [f32; 4],
    pub slip_angle: [f32; 4],
    pub tcin_action: i32,
    pub abs_in_action: i32,
    pub suspension_damage: [f32; 4],
    pub tyre_temp: [f32; 4],
}

/// Maps to `Local\acpmf_graphics` — session state, timing, and multi-car data.
#[repr(C, align(4))]
pub struct PageFileGraphic {
    pub packet_id: i32,
    pub status: i32,
    pub session: i32,
    pub current_time: [u16; 15],
    pub last_time: [u16; 15],
    pub best_time: [u16; 15],
    pub split: [u16; 15],
    pub completed_laps: i32,
    pub position: i32,
    pub i_current_time: i32,
    pub i_last_time: i32,
    pub i_best_time: i32,
    pub session_time_left: f32,
    pub distance_traveled: f32,
    pub is_in_pit: i32,
    pub current_sector_index: i32,
    pub last_sector_time: i32,
    pub number_of_laps: i32,
    pub tyre_compound: [u16; 33],
    pub replay_time_multiplier: f32,
    pub normalized_car_position: f32,
    pub active_cars: i32,
    pub car_coordinates: [[f32; 3]; 60],
    pub car_id: [i32; 60],
    pub player_car_id: i32,
    pub penalty_time: f32,
    pub flag: i32,
    pub penalty: i32,
    pub ideal_line_on: i32,
    pub is_in_pit_lane: i32,
    pub surface_grip: f32,
    pub mandatory_pit_done: i32,
    pub wind_speed: f32,
    pub wind_direction: f32,
    pub is_setup_menu_visible: i32,
    pub main_display_index: i32,
    pub secondary_display_index: i32,
    pub tc: i32,
    pub tc_cut: i32,
    pub engine_map: i32,
    pub abs: i32,
    pub fuel_x_lap: i32,
    pub rain_lights: i32,
    pub flashing_lights: i32,
    pub lights_stage: i32,
    pub exhaust_temperature: f32,
    pub wiper_lv: i32,
    pub driver_stint_total_time_left: i32,
    pub driver_stint_time_left: i32,
    pub rain_tyres: i32,
}

/// Maps to `Local\acpmf_static` — session/car metadata (constant during a session).
#[repr(C, align(4))]
pub struct PageFileStatic {
    pub sm_version: [u16; 15],
    pub ac_version: [u16; 15],
    pub number_of_sessions: i32,
    pub num_cars: i32,
    pub car_model: [u16; 33],
    pub track: [u16; 33],
    pub player_name: [u16; 33],
    pub player_surname: [u16; 33],
    pub player_nick: [u16; 33],
    pub sector_count: i32,
    pub max_torque: f32,
    pub max_power: f32,
    pub max_rpm: i32,
    pub max_fuel: f32,
    pub suspension_max_travel: [f32; 4],
    pub tyre_radius: [f32; 4],
    pub max_turbo_boost: f32,
    pub deprecated_1: f32,
    pub deprecated_2: f32,
    pub penalties_enabled: i32,
    pub aid_fuel_rate: f32,
    pub aid_tire_rate: f32,
    pub aid_mechanical_damage: f32,
    pub aid_allow_tyre_blankets: i32,
    pub aid_stability: f32,
    pub aid_auto_clutch: i32,
    pub aid_auto_blip: i32,
    pub has_drs: i32,
    pub has_ers: i32,
    pub has_kers: i32,
    pub kers_max_j: f32,
    pub engine_brake_settings_count: i32,
    pub ers_power_controller_count: i32,
    pub track_spline_length: f32,
    pub track_configuration: [u16; 33],
    pub ers_max_j: f32,
    pub is_timed_race: i32,
    pub has_extra_lap: i32,
    pub car_skin: [u16; 33],
    pub reversed_grid_positions: i32,
    pub pit_window_start: i32,
    pub pit_window_end: i32,
    pub is_online: i32,
}
