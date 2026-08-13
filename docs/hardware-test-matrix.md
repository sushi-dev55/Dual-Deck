# Hardware test matrix

This file is the evidence log for physical-controller compatibility. Automated tests cover event
normalization and trigger policy, but they cannot replace tests on a real controller.

## Status definitions

| Status         | Meaning                                                                                  |
| -------------- | ---------------------------------------------------------------------------------------- |
| Pass           | The listed behavior was exercised on the recorded configuration and matched expectations |
| Fail           | The behavior was exercised and did not match expectations                                |
| Blocked        | The test could not complete because of a recorded technical blocker                      |
| Unavailable    | The platform or device did not expose this optional data during the test                 |
| Not run        | No evidence has been collected for this configuration                                    |
| Not applicable | The behavior does not apply to the configuration                                         |

Do not infer a pass from device discovery or successful compilation. Update each row only after the
specific behavior has been exercised.

## Verified configuration

Evidence collected on 2026-08-06:

| Field               | Value                                                      |
| ------------------- | ---------------------------------------------------------- |
| Host                | Windows 11 Pro 64-bit, version `10.0.26200`, build `26200` |
| Controller          | First-party DualSense Wireless Controller                  |
| USB identity        | Sony VID `054C`, PID `0CE6`                                |
| Controller firmware | `1584`                                                     |
| Connection          | USB, reported by SDL as wired                              |
| SDL                 | Vendored SDL `3.4.14` HIDAPI runtime                       |
| SDL gamepad type    | `6`, PlayStation 5                                         |
| Rust target         | `x86_64-pc-windows-gnu`                                    |

The latest five-second controller probe opened the controller, identified it as a real PlayStation
5 gamepad, reported firmware `1584`, observed a live axis event, and finished with zero ignored
devices, zero dropped events, and no stuck buttons. SDL reported the wired controller as charged at
`100%`. An earlier raw query returned unknown battery data, so the application continues to handle
unavailable telemetry without inventing a percentage.

The Rust `controller_probe` example passes `cargo check` for the GNU target. This evidence does not
claim an end-to-end editor mapping or action execution pass.

USB is the only physically verified connection for the current release scope. Bluetooth remains
pending and must not be presented as verified support until its rows pass on real hardware.

## Compatibility summary

| Windows                | Connection | Discovery and open | Control API | Battery | End-to-end mappings |
| ---------------------- | ---------- | ------------------ | ----------- | ------- | ------------------- |
| Windows 11 build 26200 | USB        | Pass               | Pass        | Pass    | Not run             |
| Windows 11             | Bluetooth  | Not run            | Not run     | Not run | Not run             |
| Windows 10 64-bit      | USB        | Not run            | Not run     | Not run | Not run             |
| Windows 10 64-bit      | Bluetooth  | Not run            | Not run     | Not run | Not run             |

## Runtime behavior

| Test                            | USB     | Bluetooth | Notes                                                      |
| ------------------------------- | ------- | --------- | ---------------------------------------------------------- |
| Connect before launch           | Not run | Not run   | Verify one connected event and correct snapshot            |
| Connect after launch            | Not run | Not run   | Verify hot-plug discovery without restart                  |
| Disconnect while idle           | Not run | Not run   | Verify snapshot clears and held triggers reset             |
| Disconnect during hold-repeat   | Not run | Not run   | Verify repeats stop immediately                            |
| Reconnect same controller       | Not run | Not run   | Verify one clean reconnect and no duplicate events         |
| Second DualSense connected      | Not run | Not run   | Verify first stays active and ignored count becomes one    |
| Pause and resume mappings       | Not run | Not run   | Verify input status continues while actions stop           |
| Editor hidden to tray           | Not run | Not run   | Verify mappings continue without visible webview           |
| Windows sleep and resume        | Not run | Not run   | Verify recovery without relaunch                           |
| Foreground game pass-through    | Not run | Not run   | Verify both the game and mapped action receive the input   |
| Startup while controller absent | Not run | Not run   | Verify the app remains usable and later detects the device |

## Input coverage

Exercise a press and release for each control. Axes should also be checked near center, at each
extreme, and across configured trigger thresholds.

| Input group                                  | USB     | Bluetooth |
| -------------------------------------------- | ------- | --------- |
| Triangle, Circle, Cross, Square              | Not run | Not run   |
| D-pad directions                             | Not run | Not run   |
| L1 and R1                                    | Not run | Not run   |
| L2 and R2 digital thresholds                 | Not run | Not run   |
| L3 and R3                                    | Not run | Not run   |
| Create, Options, PlayStation, and microphone | Not run | Not run   |
| Touchpad click                               | Not run | Not run   |
| Left and right stick axes                    | Not run | Not run   |
| L2 and R2 analog axes                        | Not run | Not run   |
| Single-button Press trigger                  | Not run | Not run   |
| Single-button Release trigger                | Not run | Not run   |
| Single-button Long press trigger             | Not run | Not run   |
| Single-button Double press trigger           | Not run | Not run   |
| Single-button Hold trigger                   | Not run | Not run   |

## Action smoke tests

Run each action from a physical mapping, not only from an interface test button.

| Action                    | USB     | Bluetooth |
| ------------------------- | ------- | --------- |
| Open application          | Not run | Not run   |
| Open file and folder      | Not run | Not run   |
| Open HTTP or HTTPS URL    | Not run | Not run   |
| Send hotkey               | Not run | Not run   |
| Type Unicode text         | Not run | Not run   |
| Media and volume controls | Not run | Not run   |
| Play WAV file             | Not run | Not run   |
| Send webhook              | Not run | Not run   |
| Close application         | Not run | Not run   |
| Switch profile            | Not run | Not run   |

Multi-actions and standalone delays are internal backend functionality and are not part of the
current user-facing action smoke test.

## Probe procedure

Put the vendored SDL DLL and MinGW runtime on `PATH`, then run a 30-second probe:

```powershell
$env:PATH = "$(Resolve-Path src-tauri\vendor\sdl3\bin);C:\msys64\mingw64\bin;$env:PATH"
$env:DUALDECK_PROBE_DURATION_MS = "30000"
pnpm cargo run --manifest-path src-tauri/Cargo.toml --locked --target x86_64-pc-windows-gnu --example controller_probe
Remove-Item Env:DUALDECK_PROBE_DURATION_MS
```

During the probe, exercise the controls relevant to the change. Save only non-sensitive output.
Record the date, Windows version and build, controller firmware, connection, SDL version, result,
and any unavailable fields in this document.
