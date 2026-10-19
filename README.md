# bullsheet

A borderless, always-on-top pop-up calculator for the desktop. Press the global hotkey, type one expression per line, and read the result to the right of each line.

## Hotkey

- `Ctrl+Alt+C` toggles the calculator.
- `Escape` hides it, or closes Settings when Settings is open.
- `Ctrl+Q` quits.
- Click a result to copy it.
- Drag the top strip to move the window.
- The hotkey can be changed from Settings.

## Settings

Open Settings from the header.

- `Start with system` writes or removes the platform autostart entry.
- Autostart launches the app hidden, so only the global hotkey is visible.
- `Shortcut` accepts a combination such as `Ctrl+Alt+C`, `Shift+F8`, or `Super+Space`. At least one modifier is required.
- Only one instance runs at a time. Starting the app again while it is already running exits the new process.

The window position, angle mode, and hotkey are remembered between launches.

## Tray

The app places an icon in the system tray.

- Left-click the icon to toggle the calculator.
- Right-click for `Show`, `Settings`, `Start with system`, and `Quit`.

## Updates

On launch the app checks the latest GitHub release in the background. If a newer version exists it is downloaded and installed silently; the new version runs the next time you launch the app. Nothing is shown or interrupted while you work. The current version and the result of the last check appear in Settings.

## Features

- Multiline expressions with a live result on every line.
- `DEG` / `RAD` toggle in the header.
- Variables: `x = 7`, then `x * 6`.
- `ans` holds the last successful result.
- Comments with `#`.
- Percent, factorial, implicit multiplication, and `**` as power.
- Functions including `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `sqrt`, `cbrt`, `abs`, `ln`, `log`, `log2`, `log10`, `exp`, `floor`, `ceil`, `round`, `trunc`, `sign`, `min`, `max`, `hypot`, and `atan2`.
- Constants `pi`, `tau`, `e`, `phi`, and `inf`.
- Persian and Arabic-Indic digits and common math symbols are normalized.

## Build

```sh
cargo build --release
```

The binary is written to:

- Windows: `target\release\bullsheet.exe`
- Linux and macOS: `target/release/bullsheet`

Run it with:

```sh
cargo run --release
```

## Layout

- `crates/bullsheet` — the pop-up window and everything around it.
- `crates/evaluator` — the expression language: text in, values out.
- `crates/element` — values and how they present.

## Autostart

Autostart is handled by the app itself. No installer or startup script is required.

- Windows: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
- Linux: `~/.config/autostart/bullsheet.desktop`
- macOS: `~/Library/LaunchAgents/bullsheet.plist`

If you move the executable, turn `Start with system` off and on again so the entry points at the new location.

## Releases

GitHub Actions builds and packages release artifacts:

- Windows: portable `.exe` in a zip.
- Linux: binary tarball.
- macOS: universal `.app` bundle in a zip.

## Platforms

- Windows 10/11 is the primary target. The borderless window uses a small Windows-specific style adjustment for the glow renderer.
- Linux is supported through X11. `global-hotkey` does not work on Wayland, and some compositors may ignore window positioning or focus requests.
- macOS builds are produced by CI. Runtime behavior on macOS has not been verified locally.

## Font

The UI uses JetBrains Mono, bundled under the SIL Open Font License. See `crates/bullsheet/assets/OFL.txt`.
