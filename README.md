# bullsheet

A borderless, always-on-top pop-up calculator for the desktop. Press the global hotkey, type one expression per line, and read the result to the right of each line.

## Hotkey

- `Ctrl+Alt+C` toggles the calculator.
- `Escape` hides it, or closes Settings when Settings is open.
- `Ctrl+Q` quits.
- Click a result to copy it; `Ctrl`-click it to copy the whole line, as in `2 * (3 + 4) = 14`.
- `Ctrl`-click any word in the sheet to copy that word.
- Hover a word to see what it is: a number, a function, a parameter, a text, or an error.
- Start typing a name and a list of matches appears; arrow keys pick one, `Enter` or `Tab` inserts it, `Escape` dismisses it.
- Drag the top strip to move the window.
- The hotkey can be changed from Settings.

## Settings

Open Settings from the header.

- `Start with system` writes or removes the platform autostart entry.
- Autostart launches the app hidden, so only the global hotkey is visible.
- `Shortcut` accepts a combination such as `Ctrl+Alt+C`, `Shift+F8`, or `Super+Space`. At least one modifier is required.
- Only one instance runs at a time. Starting the app again while it is already running exits the new process.

The window position and hotkey are remembered between launches.

## Tray

The app places an icon in the system tray.

- Left-click the icon to toggle the calculator.
- Right-click for `Show`, `Settings`, `Start with system`, and `Quit`.

## Updates

On launch the app checks the latest GitHub release in the background. If a newer version exists it is downloaded and installed silently; the new version runs the next time you launch the app. Nothing is shown or interrupted while you work. The current version and the result of the last check appear in Settings.

## Features

- Multiline expressions with a live result on every line.
- Grouped numbers: `1,234,567` is one number, and results come back grouped. Inside `(...)` a comma always separates arguments, so `min(300,100,200)` is three arguments.
- Persian keyboard input: `۱۲۳ + ۴`, `۱٬۲۳۴`, `۲٫۵`, and `min(۱،۲)` are rewritten to `123 + 4`, `1,234`, `2.5`, and `min(1,2)` as you type, because the renderer draws Arabic-script text right to left and would mirror the digits.
- Brackets are coloured by nesting depth; a bracket without a partner is red.
- Autocomplete on every name the sheet knows: functions, constants, and the variables you defined.
- Trigonometry always takes radians, so the angle is part of the expression: `sin(90deg)` is one, `x = 45deg` then `cos(x)`, and `asin(0.5) / deg` is 30. `deg` is an ordinary constant, exactly `pi / 180`.
- Variables: `x = 7`, then `x * 6`.
- Functions: `double(n) = n * 2`, then `double(21)`. Parameters are bound in a child scope that still sees the sheet's variables, so a function can use values assigned elsewhere, and functions can take other functions: `apply(g, a, b) = g(a, b)` then `apply(add, 2, 3)`.
- `ans` holds the last successful result.
- Comments with `#`.
- Percent, factorial, implicit multiplication, and `**` as power.
- Functions including `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `sqrt`, `cbrt`, `abs`, `ln`, `log`, `log2`, `log10`, `exp`, `floor`, `ceil`, `round`, `trunc`, `sign`, `min`, `max`, `hypot`, and `atan2`.
- Every operation is a value: operators have names (`add`, `subtract`, `multiply`, `divide`, `mod`, `pow`, `negate`, `factorial`, `percentage`) and symbols work as values too, so `plus = +` gives you `plus(3, 4)`, and `sine = sin` gives you `sine(1)`.
- Constants `pi`, `tau`, `e`, `phi`, `deg`, and `inf`.
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
