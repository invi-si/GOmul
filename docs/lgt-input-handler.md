# LGT input-method startup failure

Grand Chase's rescue reported unknown WIPIC service 304 (0x130), during
`CletWrapper.startApp`. The service follows input mode discovery/get/set and
matches the public SDK's `MC_imHandleInput` declaration, with six arguments.

Implemented the service for the already advertised EN/L and EN/S modes:
English keypad composition, separate committed/composing byte counts, explicit
MH_IMA_FLUSH, mode reset, and capacity checks. Composition resides in guest
memory after the existing input-mode selection. Press is 502 in the LGT bridge;
release/repeat do not duplicate a press. No host timer was introduced.

Reference declarations and contract:
- https://github.com/mirusu400/BetterCNUWipi/blob/main/apps/test/WIPIHeader.h
- https://github.com/mirusu400/wipi-wiki/blob/main/src/content/docs/c-api/graphics.md
- https://github.com/mirusu400/wipi-wiki/blob/main/src/content/docs/hal/input-method.md

Validation: 85 LGT/WIPI-C tests passed, including an ARM SVC integration test
covering stack arguments, composition, release, flush, uppercase/lowercase,
short buffers, and output sentinels. Workspace clippy passed with existing
warnings. A normal APK cold launch on the Mac AVD passed the captured startup
failure and reached the game's usage notice (Running, approximately 20 paints/s).
The original game archive and saved data were not modified by the fix.

Limits: this is English keypad input, not a Korean automaton or a claim of full
input-method conformance. No complete gameplay playthrough was performed.

Follow-up: Korean and numeric modes now use the [shared native input implementation](native-korean-input.md), preserving the existing English mode indices.
