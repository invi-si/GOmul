# LGT component setup failure reproduced from Rescue

The Vancouver 2010 report reproduced `Unknown LGT WIPIC SVC id 809`
with identical registers, CPSR and stack on the originating APK. The failure
occurred during name-entry setup, after requesting `TextComponent`.

## Contract and implementation

The WIPI 1.2.1 specification, section 5.1.11, documents component creation,
geometry, enable state and byte-addressed text operations. References:

- [Specification transcription](https://github.com/mirusu400/wipi-wiki/blob/main/src/content/docs/c-api/ui-components.md)
- [Specification provenance and original archive](https://github.com/mirusu400/wipi-wiki/blob/main/README.md)
- [Public SDK header declarations and geometry masks](https://github.com/mirusu400/BetterCNUWipi/blob/main/apps/test/WIPIHeader.h)

Implementation was written independently from these contracts. No emulator
implementation algorithms were copied.

Guest call sites identify 0x320/321/322/323 as application context, class lookup,
create and destroy, and 0x329 as configure. Their previous time-related names
and implementations were incorrect for text components. The LGT adapter now
routes to shared WIPI C component functions. Text and DateTime instances have
separate guest-memory-backed state; text insertion copies input bytes, limits
capacity, and preserves each component's contents independently. Geometry
honors position/size masks and rejects non-positive dimensions. DateTime keeps
its creation timestamp per component and returns the complete nine-field tm.

This is partial component support, not a complete native widget toolkit.
Painting, interactive text composition, callbacks and other component classes
are not implemented by this patch. Unmapped calls still fail explicitly;
unsupported classes return an error. No game-name checks, archive patches,
timer changes, frame caps or CPU optimization are involved.

## Replay validation

The optional `rescue-replay` Cargo feature adds explicit developer commands:
`rescue-capture` replays the safe prefix on a baseline build and exports the
resulting frame/save-data oracle; `rescue-verify` replays that prefix on a
candidate and requires exact frame/save equality. Only these diagnostic
commands permit a different build ID. Normal Quick Load always retains the
build check, including when this feature is compiled in. The regular build
script does not enable this feature.

The instrumentation runner always uses a disposable cache save tree. Supply
`-e mode rescue-capture` with the existing rescue/game arguments, preserve
`cache/rescue-reproduction/checkpoints/case/rescue-reference` outside that cache,
then use `-e mode rescue-verify -e reference <device-directory>` on the candidate.
Optional `-e runMs 5000` continues in live mode and exports `continuation.png`.
Optional comma-separated `-e keys UP,2,OK` sends test input after this wait.
These are test-APK functions, not a user-facing old-checkpoint migration feature.

Baseline and candidate safe-prefix replay matched exactly. The candidate then
continued for five seconds without a fatal error and displayed the name-entry
dialog. This verifies passing the captured failure, not full-game compatibility.
The private report, initial game data, screenshot and game archive are not
included in Git.

An automated `UP,2,RIGHT,2,OK` sequence left the name-entry dialog visible with
an empty field and no fatal error. Text-entry functionality is not validated;
this result must not be described as confirmed progression into gameplay.

Validation: 31 LGT unit tests + 1 LGT integration test, 51 WIPI C tests (including
four new component regression tests), and 15 Android tests passed. Android tests
also pass with the diagnostic feature enabled. Workspace clippy passes with
existing warnings. A regular APK is built with `rescue-replay` disabled.

The installed regular APK passed synthetic Android Rescue instrumentation and
rejected `rescue-verify` with `Unknown checkpoint action`, confirming the
cross-build diagnostic path is absent from the regular build.
