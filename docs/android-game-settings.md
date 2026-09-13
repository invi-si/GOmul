# Android per-game settings

The top Settings button offers the existing Quick Save/Quick Load operations,
display dimensions, and game-clock speed. Preferences are keyed by imported
archive identity, matching the save isolation used by the library.

Display overrides accept 64–1024 pixels per axis and require restart. Auto
removes the override. LGT additionally supports disabling the annunciator crop
with Full framebuffer. Defaults are unchanged. Other platforms may impose their
own display dimensions. Applying a display override writes display-options in
that game's save directory before boot; LGT applies the override at registration,
before framebuffers are allocated. Existing checkpoints made with other display
settings may fail deterministic reconstruction and should be replaced after
changing settings.

Speed runs from 0.25× to 2× in 0.25× steps, default 1×. It changes the live guest
clock while keeping its value continuous at speed changes. Recorded replay clocks
remain authoritative, and changing speed adds no replay events. Speed is
reapplied after launch and successful Quick Load.

Above 1×, host frame pickup is limited to at most 30 updates/sec. Guest drawing,
paint counts, input processing, and timer cancellation are preserved. Supported
RGB565/ARGB images are copied into owned pending buffers; conversion to Android
pixels happens only for presentation or checkpoint/rescue capture. Intermediate
unpresented frames can be replaced, but the latest complete frame is preserved.
Other image formats use the existing conversion. Explicit format metadata avoids
confusing equally sized ARGB and ABGR pixels.

Audio part delays and repeat deadlines follow the selected rate. Supported
MediaPlayer playback speeds and pitches follow that rate too; unsupported rate
changes mute the affected audio. Returning to 1× restores normal volume when the
player supports it. Audio timing calculations are tested; decoder-specific
playback and audible synchronization still need user/device testing.

Achieved gameplay speed depends on the game and available CPU headroom. No guest
instructions or game logic are skipped. This is best-effort fast-forward, not a
guarantee of twice the movement or simulation speed on every game. Host worker
polling remains unchanged: shorter and zero-wait experiments showed no useful
gain in the isolated title-sequence check and were removed.

Validation: 19 Android and 52 backend Rust tests pass. Android tests cover clock scaling, replay independence, exact
checkpoint pixels for eager/deferred conversion, unsupported-format fallback,
latest-frame retention, resize, and switching back to 1×. The Android audio timing
harness checks 0.25–2× deadline conversion. Settings menu and display dialog were
inspected in the Mac-hosted AVD. Workspace clippy and the APK build pass with
existing warnings. An isolated 2× save followed by a speed change and Quick Load
passes deterministic reconstruction on the AVD. No physical-device validation
in this change.

Development measurement (Mac-hosted API 36 AVD, isolated Bio Chronicle initial
saves; 4 seconds, OK held 100 ms/released 300 ms, then 6 seconds):

| Build/rate | Guest paints at end |
| --- | --- |
| Candidate 1× | 93 |
| Existing clock-only 2× | 191, 192 |
| Candidate 2× with temporary shorter polling | 186, 185 |
| Final 2×, original polling restored | 190 |

These are startup/animation progress observations, not gameplay FPS, identical
CPU workloads, or a benchmark of the Java presentation cap/audio path (the native
harness only picks up the final frame). Screenshots show partial title animation
at 1× and the completed title at 2×. The shorter polling candidate provides no
evidence of an improvement and is not retained. Do not claim an additional CPU
speedup or universal 2× gameplay from these results.
