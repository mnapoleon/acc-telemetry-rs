# acc-telemetry-rs

A Windows-only Rust port of the Assetto Corsa Competizione (ACC) shared memory telemetry reader.

This application reads live telemetry data from ACC via the Windows named shared memory API and prints vehicle physics, session/graphics state, and static car/track metadata to the console in real time.

## What it does

- Opens three Win32 named shared memory segments created by ACC:
  - `Local\acpmf_physics` — real-time vehicle dynamics (speed, RPM, tyre temps, forces, etc.)
  - `Local\acpmf_graphics` — session state and UI info (lap times, position, flags, etc.)
  - `Local\acpmf_static` — static metadata (car model, track, player name, specs, etc.)
- Polls for keyboard input at ~60 Hz (avoiding the busy-loop CPU issue in the original C++ version)
- Prints structured telemetry to the console when you press **1**, **2**, or **3**
- Exits cleanly on **Escape** (fixing the unreachable cleanup in the original)

## Improvements over the C++ original

- **RAII cleanup**: Shared memory handles are always properly closed (via `Drop` impl), even if the program exits unexpectedly
- **Better error handling**: Failed Win32 calls return explicit errors instead of silently continuing with null pointers
- **No busy-polling**: Uses `thread::sleep(16ms)` instead of spinning at 100% CPU
- **Type-safe enums**: Status and flag types are Rust enums (`#[repr(i32)]`) instead of raw `#define` constants
- **Proper UTF-16 handling**: Wide-string fields from ACC are decoded to UTF-8 Rust `String`s

## Prerequisites

### Windows machine setup (one-time)

#### 1. Install Rust

Open **PowerShell** or **Command Prompt** and check:

```powershell
rustc --version
cargo --version
```

If either command fails:

1. Download **rustup-init.exe** from https://rustup.rs
2. Run the installer and accept the defaults — it will install:
   - The Rust compiler (`rustc`)
   - The package manager and build tool (`cargo`)
   - The `stable-x86_64-pc-windows-msvc` toolchain
3. Restart your shell so `rustc` and `cargo` are on your `PATH`

#### 2. Install MSVC Build Tools

The `windows-msvc` toolchain requires the Visual C++ compiler and Windows SDK.

**If rustup prompted you during installation:** You may already have them. To verify, open a **Developer Command Prompt for Visual Studio** and run:

```cmd
cl.exe
```

If it prints a version, skip this step. If it fails, install manually:

1. Download **"Build Tools for Visual Studio"** from https://visualstudio.microsoft.com/downloads/
2. Scroll to **"Tools for Visual Studio"** section
3. Download **"Build Tools"** (the installer for standalone tools, not Visual Studio)
4. Run the installer
5. Select the **"Desktop development with C++"** workload (this installs `cl.exe`, the linker, and the Windows SDK)
6. Complete the installation

#### 3. Install Git

Check if Git is already installed:

```powershell
git --version
```

If missing, install from https://git-scm.com/download/win

---

## How to build and run

### Step 1 — Get the code

Clone this repository:

```powershell
git clone <your-remote-url> acc-telemetry-rs
cd acc-telemetry-rs
```

Or if you have it as a ZIP file, extract it:

```powershell
Expand-Archive -Path acc-telemetry-rs.zip -DestinationPath .
cd acc-telemetry-rs
```

### Step 2 — Build

Run Cargo to compile the project. Choose one:

**Debug build** (faster, good for development):
```powershell
cargo build
```
Output: `target\debug\acc-telemetry-rs.exe` (~5 MB)

**Release build** (optimized, recommended for actual use):
```powershell
cargo build --release
```
Output: `target\release\acc-telemetry-rs.exe` (~2 MB)

**First-time note:** Cargo will download and compile the `windows` crate and its dependencies. This requires an internet connection and takes 1–2 minutes.

### Step 3 — Launch ACC

Start Assetto Corsa Competizione and begin a practice session, qualifying, or race. The game must be running **before** you start the telemetry reader, otherwise the shared memory segments will not exist or will be zeroed.

### Step 4 — Run the telemetry reader

In a **standard Command Prompt or PowerShell** window (not Windows Terminal, which can interfere with keyboard input):

```powershell
.\target\release\acc-telemetry-rs.exe
```

You should see:

```
ACC Telemetry Reader — press 1 for physics, 2 for graphics, 3 for static
Press Escape to exit.
```

With the console window **focused**, press:

| Key | Output |
|-----|--------|
| **1** | Physics data (speed, RPM, tyre temps, forces, etc.) |
| **2** | Graphics/session data (lap times, position, flags, etc.) |
| **3** | Static data (car model, track, player name, specs, etc.) |
| **Escape** | Exit the program cleanly |

---

## Troubleshooting

### Build fails: `link.exe not found`

**Cause:** MSVC Build Tools are not installed or not on PATH.

**Fix:**
1. Install the "Desktop development with C++" workload (see **Prerequisites** section above)
2. Try building from a **Developer Command Prompt for Visual Studio** instead of a regular PowerShell

### Keys don't respond (1, 2, 3, Escape don't print telemetry)

**Cause:** The console window is not focused, or running in Windows Terminal.

**Fix:**
1. Make sure the **console window with the running program is the active window** (click on it)
2. Run the `.exe` in `cmd.exe` or **PowerShell 5** directly, not in Windows Terminal
3. If running from Windows Terminal, try running in the "Legacy Console Mode" or use a separate Command Prompt window

### All values print as `0.0`

**Cause:** ACC shared memory is empty (either ACC is not running, or hasn't started a session yet).

**Fix:**
1. Ensure ACC is **running and you've entered a session** (practice, race, etc.)
2. Stop the telemetry reader (`Escape`)
3. Launch ACC and start a session
4. Run the telemetry reader again

### Error: `Failed to open physics segment: CreateFileMappingW failed`

**Cause:** The ACC shared memory segments don't exist yet.

**Fix:**
1. Launch ACC
2. Start a practice session, qualify, or race (shared memory is created when you enter a session)
3. Then run the telemetry reader

### Antivirus blocks or warns about the `.exe`

**Cause:** The binary is new and unsigned, so heuristic-based antivirus may flag it.

**Fix:**
1. Add an exception for the `target\` directory in your antivirus settings
2. Or, run Windows Defender Exclusions:
   ```powershell
   Add-MpPreference -ExclusionPath "C:\path\to\acc-telemetry-rs\target"
   ```

---

## Optional: Skip Rust installation (pre-compiled binary)

If you don't want to install Rust on the Windows machine, you can cross-compile the binary on your Mac and transfer just the `.exe`:

**On macOS:**
```bash
cargo build --release --target x86_64-pc-windows-gnu
```

Then copy `target/x86_64-pc-windows-gnu/release/acc-telemetry-rs.exe` to your Windows machine and run it directly.

**Note:** This requires `mingw-w64` on macOS (`brew install mingw-w64`). The MSVC (`windows-msvc`) toolchain is preferred and more thoroughly tested; the GNU (`windows-gnu`) toolchain is a fallback if you absolutely want to skip installing Rust on Windows.

---

## Code structure

- **`src/main.rs`** — Entry point, Win32 API calls, shared memory management, polling loop, and console output
- **`src/shared_memory.rs`** — ACC data structures and enums (matches the binary layout of the game's shared memory pages)
- **`Cargo.toml`** — Project manifest with dependencies (`windows` crate) and build settings

All structs use `#[repr(C, align(4))]` to guarantee memory layout matches the game's shared memory pages exactly.

---

## Requirements

- **Windows 10 or later** (uses Win32 API — Windows only)
- **Assetto Corsa Competizione** (game must be running for shared memory to exist)
- **Rust 1.70+** and MSVC toolchain (or a pre-compiled binary)

---

## License & Credit

This is a Rust port of the original C++ `SharedMemoryACCS` project. The shared memory struct definitions and ACC SDK layout are based on the official ACC SDK.

---

## FAQ

**Q: Can I use this on macOS or Linux?**

A: No. The application uses the Windows named shared memory API (`CreateFileMapping`, `MapViewOfFile`) which is Windows-only. Porting to other platforms would require a different IPC mechanism, and ACC itself is Windows-only.

**Q: Can I modify the output or add more fields?**

A: Yes. All the available ACC telemetry fields are defined in `src/shared_memory.rs`. Edit the print functions in `src/main.rs` to display whatever you want, then rebuild with `cargo build --release`.

**Q: Why does it print every frame when I hold down a key?**

A: The program polls at ~60 Hz and checks if the key is currently pressed (via `GetAsyncKeyState`). While the key is held, the condition remains true and data is printed repeatedly. Press and release the key quickly to print once.

**Q: Can I use this with ACC in Replay mode?**

A: Yes. The shared memory data is still available in Replay, though some fields (like real-time physics forces) will reflect the replay playback state.

**Q: Is this faster/better than the C++ version?**

A: Functionally, they are equivalent. The Rust version has some quality-of-life improvements (proper cleanup, no busy-polling, better error handling) but the telemetry data read is identical.
