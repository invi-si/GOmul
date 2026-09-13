# GOmul Thor

Separate Android launcher for top-screen-only play on AYN Thor. It shares GOmul's
native emulator core and existing fixes. It does not open a secondary display,
change guest speed/timers, stretch the guest framebuffer, or add a second CPU
implementation. Launch GOmul Thor on the Thor's upper display.

The app locks to landscape, fits the game with its original aspect ratio, and
removes the touch keypad during gameplay. A small Library/Settings header stays
available. Select opens the existing settings, including Quick Save/Load,
dimensions and speed, plus per-game controller mapping. D-pad/left-stick input
supports held directions and diagonals. Button and axis sources are aggregated,
so releasing one source doesn't cancel another source holding the same guest key.
Lost window focus, controller disconnect, pause and settings release held inputs.

| Physical control (Android logical mapping) | Default guest action |
| --- | --- |
| D-pad / left stick | Directions |
| A | OK |
| B | CLR |
| X / Y | 5 / 0 |
| L1 / R1 | Left / right phone softkey |
| L2 / R2 | 1 / 3 |
| Left / right stick click | * / # |
| Start | CALL |
| Select | Settings |

Face, shoulder, trigger, stick-click and Start bindings can be reassigned to any
phone keypad key or disabled, separately for each imported archive. Firmware
button-layout settings may affect Android's logical A/B labels; remap as needed.
Quick Save/Load is selected from the settings menu, avoiding accidental loads
from a stick click. The bottom screen is not used by this app; screen-power
management remains a device setting.

Build with the normal Android/Rust toolchain configured:

```sh
GOMUL_THOR=1 sh wie-android/build.sh
# If the regular native library is already built, rebuild just the wrapper:
wie-android/android/gradlew -p wie-android/android -PgomulThor :app:assembleDebug :app:assembleDebugAndroidTest
```

Package: `local.wie.nativeapp.thor`; launcher label: `GOmul Thor`.
The regular package remains separate. Users import their game archives into
Thor's own library. Existing phone-app saves are not automatically transferred.
This is a debug-signed local candidate APK, not a published store release.

Validation performed: standalone Java controller-state tests (overlapping
sources, repeats, remap/release, analog hysteresis); Android instrumentation for
physical key/joystick events, per-game remapping and menu release; native JNI
startup and synthetic rescue/export tests; launcher cold start. No physical AYN
Thor was available, so actual firmware button mapping, feel and performance
still require device testing. No FPS improvement is claimed.

The audit diagnostic feature is not enabled in the distributed Thor candidate.

Hardware reference: https://www.ayntec.com/products/ayn-thor
