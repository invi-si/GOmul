# Android Rescue v1

Rescue automatically preserves failure evidence when the native emulator returns
an error or its worker unwinds with a Rust panic. It does not navigate away from
the game, reload it, send data over the network, or overwrite Quick Save.

The error panel offers **Export rescue**, **Return to library**, and **Stay here**.
The library also offers **Export last rescue** after a report exists. Export uses
the Android document picker and creates one ZIP containing:

- Last guest framebuffer as `screenshot.png` when a framebuffer exists, plus the
  original binary `frame`. A failure before the first frame has no screenshot.
- `error.txt`, emulator `build-id`, and SHA-256 game `archive-id` (binary digest).
- Initial saved-data tree and existing session recording through the observed
  failure (`trace`). This is the same host-event recording used by Quick Save.
- `safe-trace`: the recording prefix ending after the last successful tick.
  No user quick save is needed to preserve this diagnostic prefix.
- `recording-status.txt`, including recording exhaustion/divergence when present.
- A copy of the latest completed local Quick Save, if its build/game IDs match.
  Missing, incompatible or unreadable checkpoints do not prevent the report.

No game archive, token configuration or device-wide logs are added. Initial
saved data and framebuffer content can still contain private or copyrighted
material. Share reports privately. No automatic upload occurs.

## Recovery semantics and limits

This first version is failure capture/export, **not an automatic time-travel
loader**. `safe-trace` contains no independently captured final memory/filesystem
oracle, so it is explicitly not exposed as a validated Quick Load checkpoint.
It is useful for extending the developer replay runner without manually
reproducing input. The included `checkpoint/`, when present, uses the existing
checkpoint format, whose loader checks build, game, final frame and saved data.
External Mac/AVD snapshot-helper checkpoints are not bundled.

Do not bypass build checks to load an old report into a patched build. First
reproduce with the originating build/game, then explicitly validate replay
compatibility with the candidate. A pre-crash boundary may already contain
corruption, and a completed tick does not prove a healthy guest state.

Intentional library exits, Android pauses and the guest's normal exit API do not
produce reports. Hard process kills, native aborts, OS kills and hangs cannot be
caught by this mechanism. A disk/storage failure may prevent capture; the UI
then says no report was captured. Existing recording limits still apply (64 MiB).

Reports live outside saves and checkpoints at `files/rescues/<game-id>/latest`.
One previous report is retained. A temporary directory and rename/rollback keep
an interrupted write from replacing the previous completed report. Nothing is
persisted per tick: the existing recording continues, with a small bookmark
(length and clock repetition count). This does not change clock values, tape
compression, guest scheduling, timers or refresh caps. On failure, recording
and file copies can take time; the panel appears after capture finishes.

## Validation

- Rust tests: safe prefix survives compressed-clock mutation and a failed tick;
  bookmarks leave recording bytes/replay unchanged; poisoned lock capture;
  original saves and quick slot unchanged; duplicate capture suppressed;
  broken checkpoint does not discard diagnostic report; publication rollback.
- Android instrumentation uses only synthetic files: verifies exported PNG
  dimensions/colors, triggers a real native worker error with a malformed JAR,
  checks automatic report creation, and verifies stop does not duplicate it.
- No game-specific gameplay compatibility or performance claim is made.

Next stage: add a developer report-import/replay runner and verify representative
reports end to end; then measure rolling validated checkpoints before enabling
any automatic recovery. Rescue v1 alone does not promise resuming through a fix.

## Developer reproduction on the originating build

The Android test APK now accepts an extracted, trusted Rescue report and the
matching local game. Install the original app build and its Android test APK,
then run (paths are on the Android device):

```sh
adb shell am instrument -w \
  -e rescue /data/local/tmp/rescue-case \
  -e game /data/local/tmp/game.zip \
  local.wie.nativeapp.test/local.wie.nativeapp.RescueInstrumentation
```

The runner checks the archive digest and uses the existing loader's build-ID
check. It copies initial saves and the full recording into an isolated app-cache
workspace. The supplied game is read-only; regular saves and Quick Save slots
are not touched. It compares the complete reproduced native error with the
report, including registers and stack when present. Exit code zero means the
recorded failure was reproduced, **not that the game was fixed**. The outcome
is also retained in `cache/rescue-reproduction/outcome.txt`.

This is a failure-reproduction harness, not a valid successful checkpoint: the
scratch slot has no expected final saved-data oracle. A replay that completes or
produces a different error fails this test. It cannot certify a patched build,
and it does not bypass normal build or checkpoint validation. Run without
arguments for the synthetic Rescue checks.

### First real report: LGT Vancouver 2010

The original exported recording reproduced `Unknown LGT WIPIC SVC id 809`
(`0x329`) on the original APK without manual navigation. The native error,
registers, CPSR and stack matched the report. Game bytes and user saves were
unchanged. The game and report remain private fixtures, excluded from Git.

Guest call-site inspection places the failure in a component setup sequence:
`0x321` receives the string `TextComponent`, followed by `0x322`; `0x329`
receives a handle and arguments `90, 248, 60, 17, 3`. Geometry/configuration is a
strong hypothesis, not a verified native ABI contract. Nearby `0x320–0x323`
currently use time-related placeholders, so implementing `0x329` as a no-op
would conceal a larger missing component implementation. The exact native API
contracts and lifecycle still need confirmation before a safe runtime fix.

The first component failure now has a candidate fix and explicit developer
pre-crash comparison workflow; see [the component report](lgt-native-components-rescue.md).
