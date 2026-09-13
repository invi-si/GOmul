# User-reported game issues (Android)

Name-only entries are removed from support by user request. Reported problems remain open until reproduced and verified; prior broad compatible labels do not override these reports. The user subsequently authorized investigation of download-required cases; current results are below. KTF is the initial investigation target; a report is not evidence that another carrier build fails.

## Remaining blockers

The original batch is **not complete**. Darkside still fails at the bottom-right door because the supplied JAR lacks map/field1/object/11.png; the independently recovered copy is identical. Mu's reported tutorial error has not reproduced in the tested opening/NPC/attack sequence, and the requested failing action or rescue has not been supplied. Further undirected boot replays do not establish a fix for either case.

The download follow-up is resolved for three games by corrected initial database import. Dragon Eyes and Arden still require network behavior/data not available in the current offline backend; Inotia's supplied preferences fail identity-dependent validation. Searches found no usable replacement data/setup for these remaining cases. This means not currently resolved with the available evidence, not that recovery is universally impossible.

## Removed

| Carrier | ID | Title |
| --- | --- | --- |
| KTF | 402 | 2006독일축구 |
| SKT | 138 | 2006독일축구 |
| KTF | 403 | 2006영혼낚시 |
| KTF | 11 | 광란의 수족관 |
| KTF | 8 | 귀신사냥2007 |
| KTF | 436 | 데몬헌터 |
| LGT | 214 | 데몬헌터 |
| KTF | 30 | 데빌헌터 |
| KTF | 34 | 두뇌게임Q |
| KTF | 39 | 라이덴3 |
| KTF | 95 | 서든어택 |
| LGT | 236 | 서든어택 포켓 |

## Additional downloads

The user authorized local data recovery and online searches after the original batch. No successful network/authentication response is fabricated.

| Game | Current finding |
| --- | --- |
| 다이어트 타이쿤 (17) | Dense Java database import fixed; unchanged archive reaches opening story on fresh Android save at 1× |
| 블레이드마스터2 (85) | Same shared import fix; opening story verified at 1× |
| 열혈교사전설2 (122) | Same shared import fix; opening story verified at 1×; malformed school0 index was not modified |
| 드래곤아이즈2 (37) | All eight data files recognized; guest still requires a connection and retries offline failure. Alternate archived package is identical |
| 아르덴전기 (110) | Network failure with no bundled companion assets; alternate archived package is identical, no separate data recovered |
| 이노티아연대기1 (135) | Companion assets present. Preferences are read but validation depends on PHONENUMBER and rejects/replaces them. Required original identity not established; alternate ODCF package is not usable by this emulator |

The import correction preserves existing saves and deletion markers. Older installations need a fresh/reset save to receive corrected initial data; no user save was reset during these tests. Opening-story checks are not full-playthrough compatibility claims.

## Open investigation

| Game | Report | Status |
| --- | --- | --- |
| 거상 | Single press behaves like multiple presses | Fresh guest setting defaults to automatic movement. At Android 1×, switching 게임설정 → 이동방식 → 수동 makes the same short taps move short distances and stop. No repeated guest input events; emulator input semantics unchanged |
| 귀혼-무사편 | Unreadable single-color rendering | RGB/stride fix restores title, characters and menu in identical fresh 1× 20-second replay; later gameplay unverified |
| 다마고치M | Cannot enter name | Arrow-based character selection reaches confirmation and gameplay at 1×; shared missing-glyph fallback now restores the four directional instructions on Android |
| 다크사이드 스토리 M | Game error | Door-exit failure attributed to missing map/field1/object/11.png in source JAR, then null image-property query; intact asset/API null-contract evidence needed |
| 더팜1 | Black trails behind character | Fresh 1× Android run reaches free movement after name entry/scene skip; left/right house movement redraws floor without black trails. Outdoor/later movement unverified |
| 던전앤파이터-거너편 | Does not run | ALZ import fixed; title screen at 1×/2×; gameplay unverified |
| 던전앤파이터-격투가편 | Black screen | Native JNI return and C/Java presentation ownership corrected. Reported boot freeze fixed: fresh 1× Android sequence reaches the menu (443 paints), and new-game confirmation enters a skippable intro; later gameplay unverified |
| 던전앤히어로 | Stops during tutorial | Null-font crash fixed; same boot/OK sequence reaches gameplay at 1× for 30 seconds |
| 드래곤로드 | Severe lag | Current fresh Android save at explicit GOmul 1×: Speed Off ≈3.80 versus Speed 5 ≈15.79 guest paints/sec in the same settings screen. Guest setting remedy verified; no emulator timing change. Earlier relaunch test verifies persistence |
| 떡볶이 타이쿤3 | Does not run | ALZ import fixed; dialogue at 1×/2×; gameplay unverified |
| 로스트 아일랜드 | Overlapping dialogue | Zero WIPI baseline fixed; fresh 1× replay now shows readable separated opening dialogue lines |
| 리듬스타 | Does not run | ZIP/cache import fixed; music selection at 1×; gameplay unverified |
| 마스터오브소드2 | Cannot start new game | Corrupt cache repaired and ProgressComponent added; same 1× sequence reaches opening story |
| 메이플스토리-도적편 | Green image trails | Flagged 8-bit BMP palette transparency implemented and tested. Same fresh 1× Android opening dialogue loses the green patches and restores obscured characters; later movement/gameplay still requires verification |
| 메이플스토리-마법사편 | Black trails/backgrounds | encodeImage pixel capture fixed. Same 1× dialogue retains its forest background; follow-up uses 2× opening navigation then restores 1× and verifies left/right village movement without black trails. Later areas unverified |
| 뮤-흑기사편 | Tutorial error | Pending reproduction |
| 미니게임천국4 | Error starting game | KTF DateTimeComponent ABI fixed; fresh 1× replay passes game selection and enters first minigame |
| 어스토니시아 스토리 | Black screen | Guest-string lookup bottleneck reduced: identical fresh 1× Android ABBA sequences yield 45/45 control paints versus 155/166 candidate paints, all Running. Unprofiled 1× follow-up reaches animated room and readable dialogue; later gameplay remains unverified |
| 영웅서기3-대지의 성흔 | Boot error | KTF position-query mapping fixed; fresh 1× boot reaches gameplay |
| 이노티아연대기1 | Boot error | Wrapper and BMP boot issues fixed. Supplied companion files are present; preference validation requests PHONENUMBER and rejects/replaces the supplied preferences before download retry. Required identity/validation compatibility remains unresolved |
| 이노티아연대기2 | Boot error | Wrapper fixed; documented first-error/restart/Yes sequence reaches title and new-game character creation at 1× |

## First automated pass

Fresh private KTF runtimes were exercised for all 21 reported entries with a bounded boot/OK sequence at 2×. This establishes reproducible failures, not full gameplay compatibility. No browser saves were used or reset.

- Shared importer corrections: strip a single enclosing KTF `__adf__` folder, canonicalize valid ZIPs with prepended data, invalidate broken cached archives, and reject incomplete HTTP bodies instead of preserving a bad cache hit. These retain the source archive identity and do not modify guest code.
- 리듬스타: rejected attachment now boots and paints at 1× after ZIP/caching corrections; gameplay remains to be checked.
- 이노티아1/2: both pass the prior `Unknown archive format` failure at 1×. The first reaches a connection prompt; the second later reports a guest stop during the bounded input sequence. Neither is declared fully fixed yet.
- 영웅서기3: confirmed missing KTF database slot 15 (`MC_dbUnk15`); the established seek/tell/rewind ABI and fix are documented below.
- 던전앤히어로: confirmed host panic in `vendor/jvm/src/class_instance.rs:108` (`Option::unwrap`). The null-font adapter fix below removes this reproduced panic.
- 귀혼-무사편: single-color output reproduced. 던파 격투가 remains on a two-paint boot screen.
- 거너편 and 떡볶이 타이쿤3 use ALZ attachments. Both now import and boot at 1× and 2× after the shared format adapter fix. These packaging failures are distinct from the deferred in-game additional-download prompts.
- 드래곤로드: prior evidence points to the game's own speed setting (`docs/dragon-road-speed-setting.md`); verify with the current web runtime before changing emulation speed policy.

Private results/screenshots/rescues are kept outside the repository under the existing local run-artifact directory. This document records conclusions without including game binaries, user saves or signed download URLs. The prior handoff ZIP predates these changes and must be regenerated before another handoff.

### Tutorial crash attribution

A full native backtrace for 던전앤히어로 identifies the WIPI `Graphics.setFont` adapter dereferencing a null font. The adapter now forwards null to the existing MIDP default-font handling instead of dereferencing it. A regression test changes a non-default font to null, verifies default attributes and draws text afterward. Fresh native boot/input replays pass the former crash point at both 2× and 1× (30 seconds, gameplay visible); the JVM null reference implementation itself was not weakened or patched globally.

### ALZ and cache follow-up

Both 거너편 and 떡볶이 타이쿤3 use ALZ attachments. The website now recognizes that format, with bounded `lsar`/`unar` decoding and validation of member names, sizes and links. Output is streamed to memory rather than extracted to archive-selected host paths. The host needs the `unar` package. Private supplied ALZ data decoded reproducibly in repeated conversions; both affected games also pass bounded live boots at 1× and 2×.

마스터 오브 소드2 also had a corrupted cached archive. Refetching removes that packaging error, but the first 1× boot/input check ends on a flat-color frame. Its new-game report remains open.

Canonical ZIP output now uses stable timestamps/order so restoring browser-owned checkpoints into fresh temporary runtimes does not change the normalized archive hash. No commercial archives or extracted members were added to the source tree.

### KTF stream position attribution

The Hero3 KTF call site opens a packaged save, calls stream slot 4 with `(handle, 0, 2)` (seek to end), calls slot 15 with only the handle, then calls slot 4 with `(handle, 0, 0)` before reading. Native instruction offsets `0x94dc0`, `0x94dee`, and `0x94e0c` establish the seek/tell/rewind sequence. This is a position query on the existing shared guest-backed KTF file cursor, not an authentication bypass or a success-returning stub. All 13 database tests pass, including cursor/read/write/closed-handle coverage. Fresh 1× boot/input validation reaches gameplay (20 seconds); the former slot-15 boot error is resolved. Later gameplay remains unverified.

Diagnostic-only replays also exposed a background-thread `java.lang.Error` resolving the missing WIPI `ProgressComponent` in 마스터 오브 소드2. Its blank display is not yet treated as a proven color-conversion bug. Diagnostic builds remain separate from the normal web runtime.

### Progress component implementation

The missing `ProgressComponent` and its `ChangeListener` interface are implemented from the [published WIPI API contract](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/lwc/ProgressComponent.html). State is guest-backed. Value limits, stepping, passive/interactive input, margin preservation and repainting after a decreasing value are tested. All 64 WIPI Java library tests pass. The same fresh 1× boot/OK sequence now reaches the new-game opening story (20 seconds, 250 paints) instead of the flat-color failure. This clears the reproduced new-game blocker; later gameplay remains unverified.

A separate Mac launcher issue was observed during rebuild: overwriting a running Mach-O inode produced immediate SIGKILL on subsequent launch despite on-disk signature verification. Atomic replacement restored launch. The native build script now installs through a temporary executable and atomic rename.

### RGB pixel contract and row stride

Diagnostic calls from 귀혼-무사편 repeatedly reach WIPI `setRGBPixels`, which was passing RGB-only source data to MIDP with alpha processing enabled. WIPI specifies `0x00RRGGBB`; the adapter now treats it as opaque RGB. The regression test also exposed MIDP `drawRGB` ignoring the supplied scan length. It now reads padded, negative, overlapping and zero-stride rows according to the [MIDP contract](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Graphics.html), validating source bounds before painting. Tests cover unchanged source data, padding, black pixels, ignored high bytes, signed/zero stride, null input and rejection without destination mutation. All 47 MIDP and 65 WIPI Java library tests pass. The rebuilt normal local runtime restores 귀혼-무사편’s title, characters and menu in the same fresh 1× 20-second sequence (91 paints) that previously produced a flat blue screen. Other reported graphics cases remain unverified.

### 다마고치M name controls

Fresh 1× input testing reaches the name-confirmation dialog and gameplay using directional character selection, then `1` to accept. Numeric keys and the emulator Korean-mode toggle are not the controls for this guest-owned picker. Left/right selects a position and up/down cycles characters. Its arrow-symbol glyphs appear as boxes; that presentation issue remains open. No shared input semantics were changed to force numeric input into this screen.

### Fighter boot wait attribution

A fresh normal 1× launch still remains black at two paints despite OK/1/5 input. Separate diagnostic runs show the guest worker requesting 11,031, 11,029, 11,031 and 11,027 ms sleeps while a second Java thread requests 500 ms repeatedly. The monitor is exited between cycles. This rules out a permanently blocked monitor in this capture, but does not establish why the guest computed that wait or whether it is intended. No delay clamp or fabricated success was introduced. A one-pixel WIPI byte `getPixels` call also reaches an unimplemented adapter; causality is not established.

### Dialogue baseline attribution

로스트 아일랜드’s opening dialogue reproduces as overlapping lines. The diagnostic trace draws successive outlined strings at y=233 and y=237 and calls WIPI `Font.getBaselinePosition`, whose previous stub returned zero. The [KTF WIPI Font contract](https://nikita36078.github.io/J2ME_Docs/docs/KTF_WIPI_API/org/kwis/msp/lcdui/Font.html) requires the font baseline. The adapter now forwards to MIDP’s implemented query, which reports the existing fixed renderer’s 10-pixel baseline; baseline anchoring uses the same constant. No font size, guest coordinates or game-specific layout override was introduced. All 47 MIDP and 65 WIPI Java tests pass, including pixel equality between top-anchored text and baseline-anchored text positioned through the query, with translation/clipping and four drawing APIs.

Fresh normal-runtime validation at 1× now shows the opening dialogue as two separated readable lines after the same new-game sequence (14-second wait, 265 paints). The former overlap is resolved in that scene; broader gameplay remains unverified.

### KTF DateTimeComponent startup crash

미니게임천국4 reaches the menu, then game start faults at `0xffffffff` in KTF UI slot 3 (destroy). Native call sequence at offsets `0x36692–0x366bc` stores the application-context handle at SP, passes SP to slot 2 (create), passes the returned component and a stack output buffer to slot 26, then destroys the component. It copies nine 32-bit words from that output. The guest imports `DateTimeComponent`.

KTF incorrectly treated the context pointer as a context handle and slot 26 as `getMenuItem`, so creation failed before the later invalid-handle fault. The KTF adapter now dereferences the caller-owned context slot once and maps the observed slot 26 to shared `get_time`, which writes the nine-word `struct tm`. This follows the already-tested shared guest-backed date component; no error suppression, constant date or game-name branch was added. Shared by-value creation and LGT routing remain unchanged. A real KTF SVC regression passes for creation, independent components, caller-slot reuse/null rejection, complete time output and sentinels, and destruction.

For the separate 거상 input investigation, the [KTF WIPI EventQueue constants](https://nikita36078.github.io/J2ME_Docs/docs/KTF_WIPI_API/org/kwis/msp/lcdui/EventQueue.html) confirm pressed=1, released=2, repeated=3. The current Java event bridge matches that ordering; swapping release/repeat types is not a supported fix. Gameplay travel distance remains under investigation.

The normal rebuilt web runtime passes the formerly fatal menu → game-start input at 1×, reaches game selection, character selection and instructions, and runs the first minigame through its end animation (1,507 paints in the private manual replay). All 35 KTF library tests pass. This validates the reported startup crash, not every minigame or long-term progression. The private replay does not modify browser saves.

### Inotia 2 current web restart verification

With a fresh private save, initial authentication error 3001 reproduces after the logos. Reopening the same saved installation (without resetting it) reaches the missing-certificate prompt. Selecting Yes produces the game’s own connection-failure/temporary-authentication notice; acknowledgement reaches the title, new-game slots and character creation at 1× on the current normal runtime. This independently confirms the existing mutable companion-data/removal implementation described in `ktf-annunciator-and-boot-triage.md`. A blind OK sequence on the first error is not evidence of a remaining emulator crash. Long-term certificate renewal and full gameplay remain unverified; no authentication response was fabricated.

### Inotia 1 required download

After the wrapper-import correction, the current normal 1× runtime reaches the title. Starting requests a 600 KB data transfer explicitly needed to play. Failed connection opens a retry prompt; selecting No returns to title. This is now classified with the user-deferred additional-download cases rather than pursued as an unexplained boot crash. No download bypass or fabricated content was added.

Darkside follow-up: the user identified the bottom-right door. Navigating around the obstructing plant to that door reproduces a loading screen followed by `Invalid memory access; address: 0` from `EventQueue.getNextEvent` at 1×. The earlier black scene was not this exit. Private rescue and navigation captures are retained in the darkside-door run directory; root cause remains under investigation.

### Darkside door-exit attribution

A targeted native trace reproduces the same fault: field1 objects 0–10 load, `MC_knlGetResourceID` for `map/field1/object/11.png` fails, then the guest invokes `MC_grpGetImageProperty(NULL, 4)`. The host dereferences the null image. The source downloaded JAR and imported JAR are byte-identical (SHA-256 `dc16d96ada90d5a0d1e72a93a998866210e4addfb703731cb3e59b75b5399f20`), and the ZIP member CRC check passes. Both omit object/11.png. This establishes the missing asset is not caused by canonicalization or import.

The consulted WIPI image-property contract does not define a successful property value for null images. No fabricated image, guessed width or null-error suppression was added. Repair needs an intact matching asset/package or stronger evidence of the target platform’s null-image semantics. The case remains unresolved; it is not claimed compatible or silently removed from the requested work.

### Android-only continuation: background capture

The user discontinued website development; subsequent runtime validation and delivery target Android. Maple Wizard reproduces loss of the cave and village backgrounds after dialogue at 1×. Targeted Java graphics attribution shows `Graphics.encodeImage` calls; its existing BMP encoder wrote a valid header but left all pixel data zero. The implementation now reads the guest-backed graphics image, applies the graphics origin and emits BGR bottom-up rows with padding. A regression decodes the BMP and checks distinct colors, black, cropping/translation, row order, padding and unchanged source pixels. All 66 WIPI Java tests pass. Android before/after uses the same 20-second boot wait and OK/#/30×OK sequence at 1× with isolated fresh saves. Both final frames show the same 아르웬 dialogue: before the scenery is black, after the forest remains visible. Status is Running in both captures. This verifies the background-loss case; movement trails and later scenes remain unverified. Contract: [KTF WIPI Graphics reference](https://nikita36078.github.io/J2ME_Docs/docs/KTF_WIPI_API/org/kwis/msp/lcdui/Graphics.html).

Updated Android APK SHA-256: `d7cae7e0f89c02bdc2bf1947c28c06c036e267a5a79b02d5ab27f4f3a35a4b13`. Installed successfully on the existing emulator without clearing user saves. Private before/after evidence is retained in the maple-android-before-long and maple-android-after run folders.

### Maple Thief: Android reproduction and BMP attribution lead

The current Android APK reproduces solid green foreground patches in the opening 로이 dialogue, with normal Running status. The case uses an isolated cache save and the existing local normalized archive (SHA-256 db8ef04a6504d9b011a89b032bbfcb585ab17eaa6a34a275603bc64e67f516c5). No user library saves were reset.

Assets with `.png` names are actually 8-bit BMPs. `map/tile/Henesys.png` has reserved header value 1 and biClrImportant=255; palette entry 255 is RGB #209020, matching the suspect green. Many sprite assets use flag 1 with differing palette indices, while ordinary backgrounds use 0/0. The standard image decoder currently ignores these nonstandard fields. This is a specific transparency-extension hypothesis, not yet a verified contract; do not make every green pixel transparent or modify the game archive. Private screenshots and a BMP-header inventory are retained under thief-android-current. Next: establish the carrier BMP extension semantics and add synthetic format tests before a runtime change.

### Encoding failure contract and discarded mixed Android run

The complete KTF SDK Graphics documentation specifies null when encodeImage fails. Nonpositive and unrepresentable BMP/Java-array dimensions now return null before allocation; checked wide size arithmetic avoids integer wrapping, and allocation failure is handled. Regression checks cover zero, negative and maximum signed dimensions. All 67 WIPI Java tests pass.

A subsequent 더팜1 Android instrumentation run is invalid for attribution: the paint counter reset during the key sequence, and the environment's active-game identity changed to 귀혼-무사편. Do not attribute its final magenta/black frame to 더팜1 or call the trail report fixed. Gameplay automation was paused to avoid sharing the native process with manual play. The original valid wizard before/after comparison remains separate.

After the user released the Android environment, a clean 더팜1 run at 1× reached the beach and opening dialogue (sample-039), with no mixed-process counter reset. A separate numeric-5 sequence remained at the splash screen; this is a control-path observation, not a new boot failure. The corrected encodeImage failure contract also built and installed successfully.

### 더팜1 movement check

Fresh Android sequence: 15-second boot observation, 65 OK taps, private test-name entry with numeric keys, 20 OK taps, the guest's displayed `*` skip, 8 OK taps, then 8 Right and 8 Left taps. The final house scene is in free movement. Samples 102/110/118/121 show player movement with intact wooden flooring and no observed black trail at 1×. The game remains Running. This is a bounded indoor observation after the shared encodeImage fix, not a before/after attribution or full-game pass. Its archive references encodeImage and contains 107 PNG resources (no modified BMP files), separating it from the Maple Thief BMP-extension lead. Private evidence: farm-android-skip.

### 어스토니시아 Android follow-up

Current 1× APK reaches the furnished opening room after a fresh 20-second boot observation and 20 OK taps. A second run adds Down/Right/5/OK: the paint counter advances only from 41 to 43 across those inputs and settling, with an intermediate fully black frame followed by the room. The original baseline capture was black. Rendering a room is not sufficient evidence that the reported progression problem is fixed; diagnostics are pending. Evidence is retained under astonishia-android-current and astonishia-android-movement.

Targeted Android diagnostics for 어스토니시아 do not show a missing-method exception or terminated game thread in the captured opening sequence. The active thread continues making 10–100 ms Thread.sleep calls, with roughly three-second gaps between some groups. Those gaps are observations from an instrumented build, not a CPU-cost measurement or proof of the cause. The current conclusion is slow/intermittent progression, with black transitional frames, rather than a demonstrated permanent boot crash. Next attribution should distinguish guest execution, host API work and scheduling during those gaps. The normal APK and normal packaged native library were restored after the capture.

The Android compatibility test runner now accepts an optional diagnosticFilter argument and sets it before native startup. This permits targeted diagnostic traces without recompiling the Rust runtime or changing production behavior.

A follow-up trace with WIPI Graphics logging accounts for the previously unobserved gaps: 555,946 `setRGBPixels` calls out of 559,336 log lines, predominantly 1×1 writes. The inspected implementation writes directly into guest-backed image storage; it does not copy a complete destination framebuffer for each call. Each call does, however, cross WIPI/MIDP Java adapters and reconstruct drawing state. This is an attribution lead, not a measured performance win or a reason to change guest timers. A normal-build native sampling run follows; verbose-log timings are excluded from performance conclusions. Private diagnostic log: `gomul-astonishia-graphics-frames.log`.

Normal-build native CPU sampling (30 seconds, `cpu-clock:u`, 500 Hz; 14,438 samples, zero lost) identifies a host-side lookup bottleneck: ArmCore `read_bytes` 17.48%, engine `mem_read` 15.20%, null-terminated guest-string reading 13.04%, memcpy 8.92%, and CPU `run` 2.41% self samples. The three read/string symbols together account for 45.72% of samples; this is not an inclusive call-stack attribution to a particular API. KTF field lookup reconstructs guest-backed member names by reading strings byte by byte and parsing them repeatedly, consistent with the high-volume drawing trace. No CPU/timer/render behavior was changed. Next candidate should address measured string/member lookup overhead, preserve guest mutations and exact memory-fault boundaries, and be validated on the same normal Android sequence. Private evidence directory: `astonishia-native-attribution`.

A guest-RAM string-reader candidate now searches each mapped page directly under the existing core lock. The generic ByteRead fallback remains bytewise for other reader types; no member-name cache or guest-code change is introduced. Regression tests compare the original bytewise reader across page boundaries, binary byte values, repeated guest writes, empty strings and exact unmapped-page faults. Targeted suites pass: core 88, KTF 35, MIDP 47, utility 3, WIPI Java 67 (240 total). Android measurement is pending; this is not yet a verified game fix.

Android ABBA validation completed with the normal (non-diagnostic) APKs, identical fresh temporary saves, 20 seconds boot sampling, twenty OK taps and six seconds settling. All runs remained Running; control produced 45 paints in both rounds, candidate 155 and 166. Sampling used the same external 500 Hz native sampler in every round, with zero lost samples. Candidate rendering advanced further into the room animation; therefore these are timed-sequence progression counts, not an identical-frame FPS benchmark or an exact-work speedup. Native string/byte-read self samples fell substantially (candidate `mem_read_c_string` 9.46%, `read_bytes` 4.69%, `mem_read` 4.23% in the first candidate round, versus 46.84% combined byte/string-reader samples in first control). The tested generic implementation is retained; unprofiled navigation follows. Guest timers, event scheduling, input mappings, drawing semantics and speed settings were not edited.

Unprofiled follow-up with the retained APK, fresh temporary save, the same opening taps followed by the game's star/skip input, further confirmation taps and directions remained Running with 208 paints. Retained screenshots show an animated room and readable dialogue; transitional black frames still occur. This establishes improved opening progression, not complete gameplay compatibility or a guarantee that every black frame was erroneous. Evidence: `astonishia-string-reader-navigation`. The retained normal APK is installed in the AVD; ordinary user saves were not reset.

The profiling-enabled core build also passes all 92 library tests after the reader change. The installed APK remains the normal build; profiling support was tested separately.

### 던전앤파이터 격투가: native return convention

The updated normal Android build still stops on the white loading-symbol frame (two paints), independently of the string-lookup performance change. A call/return diagnostic trace and native guest disassembly isolate a JNI return mismatch. Guest update function `0x1058bd` publishes kind 2 at thread-context +0x24 and value 0 at +0x28, but leaves R0 as `0x2b1c` (11,036), a structure offset loaded near its epilogue. GOmul's `CallNative` bridge previously copied R0 into the result buffer. The main loop near `0x108dd2` subtracts elapsed callback time from that supposed interval, explaining observed sleeps around 11,031 ms. Clock API results themselves are plausible.

The candidate allocates the complete observed return area in the guest thread context, consumes explicitly published context results, and retains the register-return path for adapters that do not publish one. It restores enclosing return state across nested calls and failures. Both the guest CallNative trampoline and Rust JVM native-method entry share this behavior. Tests exercise an actual Thumb fixture that publishes 33 while leaving 199 in R0, both entry paths, nesting, fault cleanup, exact layout, existing exceptions and existing wide register returns; all 35 KTF library tests pass. Normal Android validation is pending. Private evidence: `fighter-native-return`.

After the native-return fix, the normal fresh Android sequence still shows only two visible paints, so the game is not marked fixed. The updated diagnostic build records hundreds of requested sleeps around 65–73 ms (plus the separate 500 ms worker), rather than the prior ~11,031 ms sleeps. In the captured window it also records 10,165 native FillRect calls, 2,737 DrawString calls and 458 PutPixel calls, but only one FlushLcd and one native Repaint. Drawing targets the same framebuffer used for the initial flush. The remaining lead is mixed C/Java presentation ownership: the game continues drawing native pixels and invoking Java serviceRepaints, while Display.paintDisabled persists after a native flush unless another explicit native repaint or Java drawing reclaims it. Investigate that lifecycle with regression tests; do not synthesize periodic flushes or alter timers. The normal native-return APK is restored after diagnostics.

### Native presentation ownership follow-up

A new regression reproduced the remaining display problem: after native presentation, a later Java repaint with no Java screen drawing failed to present updated native pixels. The Display now retains native screen ownership in a guest-backed field while keeping per-cycle flush suppression separate. Actual Java screen drawing clears that ownership; offscreen drawing and graphics-state changes do not. A native-owned cycle is activated only by a pending repaint, so late notifications for serviced requests do not cause duplicate presentation. No new periodic redraw, timer or pacing mechanism was added. The KTF integration test changes real guest framebuffer pixels and verifies Java repaint presents them exactly once. All 35 KTF, 48 MIDP and 67 WIPI Java tests pass (150 total). Android validation follows.

Normal Android validation of the ownership fix now reaches the illustrated Continue/New Game menu in the identical fresh 1× sequence that previously remained on the loading symbol. Paint counts at 5/10/15/20 seconds are 68/138/207/277; final count is 443, status Running. The previous normal build, already containing the JNI-return fix, remained at two paints. The final Continue slot correctly reports no saved data because these tests use fresh temporary saves. New-game navigation is being checked separately. Private evidence: `fighter-native-ownership`.

Follow-up at normal 1× accepts New Game and its numeric slot-initialization confirmation, then reaches a skippable intro (Running, 766 paints); this is not a full gameplay pass. Cross-checks on the same normal APK retain Maple Wizard's forest background/dialogue (Running, 2,275 paints over the scripted opening) and Astonishia's animated opening room (Running, 151 paints). These are bounded functional checks, not matched-workload FPS comparisons. Private evidence: `display-ownership-crosscheck/{26,58,116}`. The normal ownership-fix APK remains installed in the Android environment.

### 거상: short-tap event isolation on Android

The updated normal Android APK reaches free movement after the guest opening-scene skip. Both 100 ms and 50 ms directional taps produce a large displacement. The 50 ms run's rescue transcript records RIGHT down/up 49 ms apart and LEFT down/up 78 ms apart, with no directional repeat events. This rules out host-generated repeat delivery for this capture, but does not yet distinguish delayed guest consumption from movement performed within a single guest handler. No production input, timer or game-speed policy was changed. The test-only compatibility harness now accepts a bounded keyHoldMs argument (default remains 100 ms). Private normal screenshots, transcript and parsed directional events are in geosang-short.

A separate filtered diagnostic capture confirms exactly one guest dequeue and callback for each directional down/up, with no repeat callbacks. The four callback lifetimes are 0.139–0.172 ms. RIGHT release guest entry follows recorded host delivery by about 49 ms; LEFT release is within timestamp precision. Both still show large travel, so a long-running input callback and host repetition are not the explanation. This does not establish original-device movement correctness: update-loop/key-state handling remains to be attributed. Diagnostic timing is not a production performance benchmark. The normal APK was rebuilt and restored after capture; no production behavior was changed in this follow-up.

### Geosang automatic movement setting

Guest handler tracing identifies keyNotify at 0x114f71 with exactly one press/release pair. Independent disassembly finds a direction-key path that sets movement flags and can clear a flag on a second press of the same direction. The guest settings confirm automatic movement is the fresh-save default.

In the normal Android APK at 1x, changing the game's movement option from automatic to manual makes the same 50 ms RIGHT/LEFT taps move short distances and stop, with no further travel during the six-second settling period. This resolves the reproduced symptom through a guest setting, without changing emulator input semantics. It does not establish original-hardware timing or full-game compatibility.

Route: left softkey/menu → 게임설정 → 이동방식 → left/right to 수동 → 취소 twice. Isolated test saves were used; existing user preferences were not overwritten. Private evidence: geosang-short, geosang-game-options, geosang-manual, and geosang-handler. Method names/arguments were added to the existing trace-level KTF call log for attribution. The normal APK was rebuilt and restored after diagnostics.

Mu Black Knight Android follow-up: the current normal APK passes a fresh 1x opening replay into the skippable story (324 paints, Running). This is not the reported tutorial failure; tutorial traversal remains in progress. Private evidence: mu-android-current.

A second current Android sequence passes the story into an illustrated area/monster-level selection screen (354 paints, Running). The story skip is STAR, not HASH; the HASH input did not skip it. This capture is retained as navigation evidence in mu-android-tutorial, not as proof of tutorial completion or a fix for the reported crash.

### Flagged BMP palette transparency

The KTF package contains 1,301 BMP assets: 1,259 use reserved DWORD 1, a 40-byte DIB, 8-bit BI_RGB, and a valid palette index in biClrImportant. The corresponding LGT package has 1,073 flagged images out of 1,123 BMPs, with the same format and no invalid indices; 1,112 BMP assets are byte-identical across carriers. The identified indices select the unwanted background pixels. This is an empirically inferred phone-format extension, not a claim that standard BMP defines transparency in these fields; no public extension specification was found.

The shared decoder now applies alpha only for that explicit header combination and palette index. It preserves other palette entries even when they contain identical RGB colors; ordinary BMPs and other reserved values retain standard decoding. Unsupported variants are not guessed. Tests cover top-down/bottom-up rows, padding, duplicate palette colors, unchanged encoded input, ordinary BMP opacity, invalid indices, and truncated data. All 62 backend and 82 WIPI-C tests pass. No archive, timer, scheduling, or key behavior was modified.

Normal Android before/after uses identical fresh test saves, archive, 20-second boot and 30 OK/two RIGHT/two LEFT inputs at 1x. Both reach the same opening Roy dialogue. Before: solid green foreground patches obscure part of the scene. After: background and both characters are visible through those areas, status Running. This verifies the reproduced opening defect; later gameplay remains unverified. Private artifacts: thief-mask-before, thief-mask-after (APK hashes), and thief-cross-carrier/header-comparison.json.

The normal candidate also boots the supplied LGT Maple Thief archive into a clean opening scene (1,351 paints, Running) using the same bounded 1x inputs. This is a second-carrier compatibility check, not an identical-scene timing comparison. Private evidence: thief-mask-lgt.

Darkside package recheck confirms object/11.png is absent even allowing filename case variants; other field1 object entries 0-31 are present. A targeted web search for the mobile title/JAR/download found the existing [DubiGame distribution](https://dubigame.tistory.com/19), but no alternate intact package in those results. This does not prove that no recoverable copy exists.

Mu tutorial navigation on the normal Android APK now reaches the first NPC quest dialogue using alternating UP/LEFT movement in the isometric scene; it remains Running. Earlier straight-UP probes moved diagonally past the NPC and were not tutorial-dialogue validation. Private evidence: mu-npc-diagonal. The reported tutorial error remains open until its failing action is reproduced or specifically rechecked.

Mu follow-up passes additional first-NPC dialogue and returns to the starting area with attacks and the portal animation active (263 paints, Running). A manual rescue of this non-failing state is retained privately in mu-quest-dialogue. The user has been asked for the specific tutorial action/error while other cases continue.

Wizard extended 1x control retains the forest background after 60 further confirm inputs, but remains in dialogue; directional inputs there do not establish a movement-trail pass. Private evidence: wizard-movement. A numeric-confirm continuation is being checked.

Wizard numeric-confirm follow-up advances the opening conversation to a later speaker while keeping scenery intact (3,313 paints, Running), but the final directional probes still occur inside dialogue. It is deliberately not recorded as a movement-trail pass. Private evidence: wizard-numeric-movement.

Live Android removal audit: 245 library entries were checked by sourceId, and five alternate-build metadata entries lacking sourceId were reviewed by title. None of the twelve requested removed carrier IDs/titles remain in the active library. The audit uses the live app metadata, not just the earlier restore manifest; private result: android-grid-restore/current-removal-audit.json.

Wizard navigation correction: a mixed-key probe advances to a different location and speaker, so the earlier END prompt is not evidence of a fixed freeze. The embedded help text confirms numeric 5 is confirm/action. The test-only harness now supports bounded WAIT and SPEED steps, allowing 2x story navigation followed by an explicit 1x restoration before movement probes; production input behavior is unchanged.

### Wizard movement verification

The longer opening sequence reaches a scene without the dialogue overlay. Test-only SPEED:2000 was used for the opening, followed by SPEED:1000 and a one-second wait before five RIGHT and five LEFT taps. Frames 128/133/138 show the blue-haired character in different positions with the forest/ground redrawn cleanly; the final settling frame also has no black background trail. The existing encodeImage pixel-capture fix is therefore verified for both the earlier dialogue-background failure and this village movement case. No production change was needed in this follow-up, and later areas are not claimed tested. Private evidence: wizard-opening-long.


### Archive recovery and additional-data inventory

A separately preserved Darkside package from the Internet Archive 100+ Korean WIPI collection is byte-identical to the supplied ZIP and inner JAR, including the missing field1 object 11. Four further collection ZIP listings produced no named Darkside match. This search has not recovered an intact replacement; it does not prove none exists. Private metadata and package audit: darkside-recovery.

The same collection has an encrypted Inotia companion ZIP containing char.dat, map.dat, mon.dat, pattern.dat, prefs and tile.dat. All six match the size and CRC of unencrypted files already present in the supplied Inotia package. No password recovery or substitute data is necessary. The current Android import also retains all six P/ members. Diet Tycoon, Dragon Eyes 2, Blade Master 2 and Teacher Legend 2 likewise have external P/ assets; Arden has none in its supplied package. Their presence alone does not establish successful installation or version compatibility. Private inventory: additional-downloads/package-inventory.json.

A fresh Inotia Android check caught a regression in the recent BMP-mask change before reaching its prior download prompt: Annunciator.bmp is a standard-decodable 8-bit image with reserved flag 1, 256 palette entries and biClrImportant 304. An out-of-palette hint is outside the empirically supported extension; rejecting the whole decoded image was too strict. The decoder now preserves standard pixels for such hints, with regression coverage for boundary, 304 and maximum unsigned values. Valid supported masks and malformed-row rejection remain covered. Android validation follows.

BMP fallback validation: all 63 backend and 82 WIPI-C tests pass (145 total); the normal Android APK builds successfully and is installed. No guest assets or saves were changed. Fresh Inotia replay is being checked.

Fresh normal Android Inotia validation now passes boot into the original connection-retry prompt (Running, 2,187 paints), using the same archive and isolated input sequence that failed before the fallback. This closes the BMP regression, not the additional-download issue. Evidence: additional-downloads/inotia-bmp-fallback. A focused data-access trace follows because all six companion files are already packaged.


### Additional-download Android attribution

Six bounded fresh diagnostic sequences are retained under additional-downloads/diagnostic-ID. These are prompt/data-access checks, not gameplay or performance passes.

- Inotia reads the supplied 64-byte prefs stream, then writes replacement preferences and reaches connection retry. Companion assets are present; guest acceptance/identity remains to be explained.
- Dragon Eyes 2 stats all eight supplied data files successfully, with exact sizes, then remains at CONNECTING (872KB/872KB) in the sampled window. The zero-missing-data situation is reproduced, not solved.
- Diet Tycoon, Blade Master 2 and Teacher Legend 2 remain at their numeric download-consent screens. Their Java databases are packaged as .db/.idx pairs, while the importer previously only seeded the physical filenames as single records.
- Arden reaches a connection-failed/restart screen; no P/ files are present in this package. Online recovery remains open.

A narrow KTF import candidate recognizes the observed 45-byte qtpdb dense fixed-record variant: big-endian width/count, zero field at offset 13, equal dense counts, and an exact payload-length match. It seeds the logical database name with consecutive records, preserving existing logical databases and the existing one-time installation/deletion policy. Physical archive members remain intact. Unknown index forms are not inferred. Teacher Legend school0.idx has different bytes in the signature/width region and is deliberately not interpreted as the dense variant. This is an empirical export-format inference, not a published-format claim. All 37 KTF tests pass, including record splitting, preserving progress/deletions and rejecting unsupported metadata. Fresh normal Android validation follows. Existing installed saves are not automatically replaced or migrated.


Normal Android dense-database candidate verification passes the previously reproduced download-consent screens for Diet Tycoon (158 paints), Blade Master 2 (171) and Teacher Legend 2 (276). All three reach illustrated/text opening stories with unchanged archives and fresh isolated saves at 1x. This verifies the reported startup/data-recognition issue, not full gameplay. Teacher uses its valid schoolComm database for this path; no repair of school0.idx was necessary. A private copy replacing school0.idx with its included backup was prepared but never run or installed and is not evidence for the fix. The generic import is retained. Existing install markers preserve prior saves/deletions; older installations need a fresh/reset save to receive the corrected initial import, and user saves have not been reset here. Private captures: additional-downloads/java-db-candidate-{17,85,122}.

The remaining Dragon Eyes trace repeatedly hits the unimplemented KTF group-12 slot-1 during the completed-data screen. This is a concrete ABI attribution lead; the slot contract is not yet established and must not be guessed. Inotia's preference validation and Arden's missing external assets/network dependency remain open.


Dragon Eyes 2 completion-loop attribution: an additional 60-second normal wait retains the 872KB/872KB screen while paints increase to 4,990. A separate diagnostic SVC trace records 683 Net.connect calls at caller 0x104f49, each returning accepted (0), with callback 0x104de9. Independent guest disassembly shows nonzero callback result clears a busy byte and returns; the update caller can retry next frame. A success path would proceed to socket creation. All eight bundled data sizes are recognized, but this path still requests network access. Interface12 slot 1 is in the paint path followed by FlushLcd; its return value is unused here, so it is not established as the loop cause. No success was fabricated and no unknown slot remapped. Evidence: completion-wait-37 and completion-svc-37 (trace and caller disassembly). The normal APK was restored afterward. Shared trace-level SVC diagnostics now include ID, caller, arguments and results; normal release logging compiles them out.


Inotia preference attribution: SVC replay confirms a successful 64-byte read, followed by validation and a 64-byte rewrite. Guest reader 0x10cc6c ignores the read count and closes normally; its caller 0x11ef10 invokes validator 0x11ef60. That routine calls a helper requesting the PHONENUMBER system property (native GetSystemProperty, caller 0x101ba3) before checking the decoded preference bytes. This identifies emulated identity as a concrete validation dependency; it does not establish the original required phone number or prove it is the only mismatch. No number was guessed, no preference bytes patched and no online success fabricated. Evidence: prefs-svc-135 trace and independent disassembly. Targeted searches for Inotia naming/prefs instructions and Dragon Eyes companion data found no additional results; Arden search found the existing Dubi package, not a separate data bundle. These search results do not prove recovery impossible. Normal APK and packaged native library restored after diagnostics.


### Tamagotchi name-instruction glyphs

A targeted Android text trace confirms the game emits standard Unicode arrows U+2190–U+2193 correctly. The bundled NeoDGM font's Unicode cmap maps all four to missing glyph ID 0. This is a font-coverage failure, not corrupt string decoding or a reason to change name-entry controls. Shared canvas text rendering now draws a small directional fallback only when those glyphs are missing, preserving the font advance and honoring the existing clip/alignment. Fonts that supply the glyphs continue through normal rendering. Five root font tests pass, including distinct directional shapes, spacing, clip boundaries and existing text rendering; the normal APK builds and is installed. Android picker verification follows. Diagnostic guest_text logging was added at trace level and is compiled out of ordinary release logging.

Normal Android verification at 1x using OK then numeric 1 shows readable left/right and up/down arrows in the name picker, where the earlier capture showed boxes. Status Running, 181 paints. The ordinary updated APK remains installed. Private evidence: additional-downloads/name-arrow-fixed-382/sample-007.png. No name-entry or key-dispatch behavior was changed.

Final collection search: the remaining 3,160-member ZIP listing contains no named Darkside package. Retrieved Arden and Dragon Eyes packages are byte-identical to the supplied archives. The alternate Inotia archive contains an ODCF-wrapped JAR with the same PID but a different AID and no P/ data; it does not supply a usable replacement or identity instructions. Private evidence: darkside-recovery/collection-package-audit.json. Search coverage is bounded and does not prove no other copy exists.

Completion audit: requested catalog removals are verified against the live library, retained code fixes have targeted tests and scoped runtime evidence, and the current installed normal APK matches the packaged artifact (SHA-256 c7a6da2193030b0d8b950b2cd817285d7f4c64621fe9f906a2d23ef80bd19af1). The goal remains unmet because the missing Darkside asset and unprovided Mu failure reproduction persist. Those same blockers have been recorded across multiple consecutive goal turns. Recovery searches and the remaining independent Android verification are now complete for the available evidence; no live diagnostic/test process remains. New failing-scene evidence or usable replacement data is needed before another justified repair can be verified.
