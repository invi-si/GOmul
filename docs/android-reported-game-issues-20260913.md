# Android user reports — 2026-09-13 follow-up

These user reports override earlier broad compatible labels. Name-only entries are excluded by user preference, not an inferred technical failure. Carrier versions remain distinct: SKT 카오스블레이드 is under investigation; KTF 카우스블레이드 is excluded. 어스토니시아 123 is provisionally tracked against the KTF entry and unavailable SKT catalog entry; individual episodes are not independently verified.

## Removed from support

| Carrier | ID | Title |
|---|---|---|
| KTF | 405 | K-1 WGP 뉴챌린저 |
| SKT | 302 | 나이트메이커 |
| KTF | 382 | 다마고치M |
| SKT | 371 | 닥터K |
| SKT | 372 | 당신은 골프왕 |
| SKT | 375 | 댄스배틀 오디션 |
| KTF | 73 | 미니고치 |
| SKT | 299 | 미니고치 |
| SKT | 446 | 삼국지연의2 |
| KTF | 91 | 삼국축구대전 |
| KTF | 107 | 신시티2 |
| KTF | 108 | 심시티 |
| KTF | 117 | 얼음낚시 |
| KTF | 142 | 장금이의 꿈 |
| KTF | 158 | 카우스블레이드 |
| KTF | 161 | 컴투스사커2006 |
| KTF | 162 | 컴투스포춘골프 3D |
| KTF | 174 | 테트리스2006 |
| KTF | 176 | 트랜스포머 |
| KTF | 182 | 프리스타일 어택 |
| KTF | 190 | 해적왕2007 |

## Reported failures — original reports

| Carrier | ID | Title | User report |
|---|---|---|---|
| KTF | 196 | KBO 프로야구2009 | 오류 / rescue |
| LGT | 206 | 게임빌 2010프로야구 | 검은 부팅 화면 |
| SKT | 341 | 노리맥스 영웅전 | 실행 멈춤 |
| KTF | 19 | 다크사이드 스토리 M | 오류 / rescue |
| KTF | 37 | 드래곤아이즈2 | 추가 다운로드 멈춤 |
| KTF | 45 | 리듬스타 | 실행 불가 |
| KTF | 53 | 만귀토벌전 | 렉 여부 확인 |
| KTF | 57 | 메르헨전기 | 대사 표시 이상 |
| LGT | 226 | 뮤직팩토리 | 튜토리얼 노트/음악 없음 |
| KTF | 93 | 생과일 타이쿤 파이널 | 오류 / rescue |
| KTF | 94 | 생존일기 | 오류 / rescue |
| KTF | 106 | 시네마 타이쿤 | 오류 / rescue |
| KTF | 116 | 어스토니시아 스토리 | 심한 렉 |
| SKT | 447 | 어스토니시아 스토리 | 심한 렉 |
| SKT | 448 | 웰루시아 | 검은 부팅 화면 |
| KTF | 135 | 이노티아연대기1 | 추가 다운로드 |
| KTF | 136 | 이노티아연대기2 | 인증 오류 |
| LGT | 246 | 이터니티-천상의화원 | 실행 오류 |
| KTF | 424 | 지크 | 부팅 멈춤 |
| SKT | 139 | 카오스블레이드 | 인증 오류 |
| KTF | 178 | 판타지포에버2 | 추가 다운로드 |
| KTF | 187 | 하베스트 타이쿤 | 한글 깨짐 / rescue |

## Confirmed fixes and bounded validation

- **하베스트 타이쿤 (KTF 187):** source story50 bytes contain intact CP949 Korean, but decoded draw strings contained replacement characters/Hanja. The Java InputStreamReader dropped the second byte of complete Korean pairs at chunk boundaries. Correct prefix handling restores the tutorial question/options in the same rescue replay. No game bytes changed.
- **생존일기 (KTF 94):** recorded panic dereferenced a null byte array in String([B). The runtime now raises a guest NullPointerException, which the game catches. Identical rescue continuation reaches the island gameplay scene (202 paints) instead of stopping (158 paints).
- **KBO 프로야구2009 (KTF 196):** caller 0x104a4c passes the application handle returned by UIC slot 0 directly to create slot 2. Other tested KTF callers pass a pointer to the handle. Guest context-tag validation distinguishes both forms. Rescue continuation passes the former address-12 fault (1150 paints).
- **생과일 타이쿤 파이널 (KTF 93):** implemented documented setInputMethodListener registration/replacement/null removal in guest fields. Replay followed by the same OK input passes the formerly missing-method error (149 paints). This does not assert every InputMethodHandler operation is implemented.
- **웰루시아 (SKT 448):** supplied LBMP font headers establish two grayscale bitplanes plus a vertical LSB-first transparency plane; decoding the supplied numeric glyph mask independently produces recognizable 0/1/2 glyphs. Added decoder support and color-image mask handling. Full-screen XDisplay.refresh now submits the guest LCD buffer instead of requesting redraw of stale host pixels. First replay improves from 1 black paint to 113 paints with opening artwork. The subsequent grayscale-polarity correction restores the opening Korean text on the normal APK at 1×; full playthrough remains unverified.

The RustJava dependency is now vendored at its existing crates.io 0.1.1 version with its MIT notice so the reader/null-array fixes ship reproducibly. Its published integration-test target references an unavailable upstream test_utils package; those upstream integration sources are retained but the target is disabled. GOmul's JVM tests and the new library unit test cover the modified paths.

Contracts: [KTF InputMethodHandler API](https://nikita36078.github.io/J2ME_Docs/docs/KTF_WIPI_API/org/kwis/msp/lcdui/InputMethodHandler.html).

## Remaining findings

| Title | Current evidence / next required investigation |
|---|---|
| 게임빌 2010프로야구 (LGT 206) | Zero-paint black boot is idle rather than sustained CPU spinning. Inputs reach the guest; native paint calls unsupported LGT slots 0x6a/0xee. Their contracts remain unconfirmed. Removing supplied records changes startup but does not fix progression or establish that the records are corrupt. |
| 노리맥스 영웅전 (SKT 341) | Bundled local validator also rejects the supplied key on a standard JVM with GOmul’s identity. Matching identity/key data remains unavailable; see independent validation below. |
| 다크사이드 스토리 M (KTF 19) | Fresh new-game door exit reproduces missing map/field1/object/11.png followed by a null-image property fault. Both recovered distributions omit it. Deferred pending intact data; no fabricated replacement. |
| 드래곤아이즈2 (KTF 37) | Earlier confirmed download/connection retry remains unresolved; supplied data alone does not satisfy it. |
| 리듬스타 (KTF 45) | Repeated fresh startup passes, including Game Over on the monitor-corrected build (460 paints). User's failing launch is not reproduced with the same saved state; not classified as a newly fixed defect. |
| 만귀토벌전 (KTF 53) | Controlled stationary dialogue improves from 5.39 to 6.39 paints/sec after the retained Java metadata comparison change; severe lag remains. Full gameplay performance is not verified. |
| 메르헨전기 (KTF 57) | Quick confirmations reproduce overlap even at 1×; 1.5-second pauses preserve clean text at 1× and 2×. Single-card scene. Line-skip/redraw sequencing investigation pending; waiting is a workaround, not a fix. |
| 뮤직팩토리 (LGT 226) | Missing native playback-start callback identified and queued delivery implemented. Normal APK now shows notes, misses, and Game Over. Exported tutorial MIDI is accepted by Android MediaPlayer (71232 ms duration, playing at 1957 ms after a 2-second wait). Full-song completion and physical audibility remain outside this validation. |
| 시네마 타이쿤 (KTF 106) | Class-header layout fixed: diagnostic rescue passes the former protected-field fault and confirmation proceeds to the next dialogue. Four other KTF rescues pass; Darkside’s distinct missing-asset failure was reproduced separately from a fresh save. |
| 어스토니시아 스토리 (KTF 116) | Controlled stationary dialogue improves from 4.70 to 5.39 paints/sec after the retained Java metadata comparison change; severe lag remains. SKT catalog 447 has no available attachment; three episodes are not individually verified. |
| 이노티아연대기1 (KTF 135) | Recovered matching companion identity: original data now passes download retry and reaches the introduction at 1×. Fresh-install setup wired; current Android save repaired with backup. Gameplay awaits user confirmation. |
| 이노티아연대기2 (KTF 136) | Normal Android first-error/reopen/Yes sequence reaches the title at 1× (348 paints). The certificate prompt defaults to No; blind OK exits. Startup workaround verified, later gameplay/renewal not verified. |
| 이터니티-천상의화원 (LGT 246) | First launch displays a stability shutdown message. Reopening with the same isolated save passes it and reaches the title screen (1077 paints). No code change required for this bounded startup case; later gameplay remains unverified. |
| 지크 (KTF 424) | Fixed monitor starvation: copied-existing-save startup passes the former 112-paint logo freeze, reaches town, and accepts movement at 1×. Bounded validation, not full-game certification. |
| 카오스블레이드 (SKT 139) | Bundled local validator independently rejects GOmul’s identity/key combination. Separate from excluded KTF 카우스블레이드; matching identity data remains unavailable. |
| 판타지포에버2 (KTF 178) | Existing save reproduces the download loop. Resetting its copied common configuration to the unmodified supplied export restores town gameplay at 1×; assets are present and original save is preserved. See existing-save comparison below. |

Private evidence is under the local runs/android-reported-20260913 directory; commercial archives and rescue files are not placed in this repository. Name-only imported folders were moved to the app-private unsupported-games archive; saves and original Mac downloads remain intact.

## Regression validation

The final targeted suite passed: backend 65, KTF 37 plus 1 integration test, SKVM 28, WIPI C 82, WIPI Java 69 (282 total). The vendored runtime chunk-boundary unit test also passed (1). These checks cover the changed paths, not full playthrough compatibility. The normal Android build excludes compatibility-audit instrumentation.

The normal APK was built and installed on emulator-5556 after diagnostic probes, then the app was returned to the library. Existing regular saves were preserved. Native checkpoint build identity changes can make older quick checkpoints incompatible; no identity guard was bypassed.

## Additional-download continuation

Rechecked the current archive instructions: [Inotia 1](https://dubigame.tistory.com/135) requires a first launch without P/ data, closing at the download prompt, then copying the companion data and reopening. [Inotia 2](https://dubigame.tistory.com/136) explicitly expects the first authentication exit, then reopening and selecting Yes. The test harness now supports a staged copy into its isolated cache save and separate post-restart inputs. No production save or ROM is changed by these tests. Blind confirmation previously selected the default No in Inotia 2; that result is not a validated failure of the prescribed Yes sequence.

Inotia 2 normal-APK evidence: restart-select-yes-136 finishes Running at the title screen after the actual Yes selection and connection-notice acknowledgement. The first restart-select experiment pressed Left while the publisher logo was still visible and then selected No; its failed outcome is retained rather than treated as a valid Yes test. The corrected sequence includes a wait for the certificate prompt. No game patch or fabricated server response was needed.

Inotia 1 staged installation is now tested on the normal APK: launch an isolated copy without P/, stop at the 600 KB consent dialog before accepting it, copy all six original P/ files into the isolated database, then reopen the intact game. It still reaches connection retry. All five asset files remain byte-identical to the originals; only the 64-byte prefs file is replaced. Evidence: staged-before-consent-135 screenshots and data-comparison.json. This rules out that documented installation ordering alone as a solution in GOmul; it does not establish missing/lost assets or identify the precise failed preference check. Earlier title-only and already-accepted staging probes are retained separately and are not substituted for this exact test.

### Inotia preference format narrowing

Independent guest disassembly shows that 0x11ef60 both decodes the 62-byte payload using a table and the phone string and checks the sum of the original encoded bytes against byte 62; byte 63 supplies a seed. The supplied file has sum 69 and stored checksum 69. The rewritten file also has a matching checksum. The caller at 0x11eba0 then reads an eight-bit field from the decoded settings and expects 0x4B; otherwise it resets them. This corrects the earlier overly broad “preference validation fails” attribution: actual runtime return values and the decoded header still need inspection. The matching on-disk checksum does not prove the runtime buffer/read path or decode succeeds.

An opt-in Android guest-debugger experiment failed to advance reliably and produced no breakpoint observations. Its new hook was removed, and no runtime-register claim is based on it. Private evidence: staged-inotia/prefs-consumers-disassembly.txt and debug-session logs. No preference bytes, identities or guest branches were patched.

Independent format verification subsequently located the 256-byte table in supplied work.bar at offset 0x87938. The decoder derived from the guest instructions reproduces a valid 0x4B header for the game-generated prefs under the default emulated phone identity; the supplied prefs instead decode to 0x46. The header occupies raw byte 3 because the guest copies native little-endian words before reading the first high-order eight-bit field. Both encoded checksums match. This supports identity/profile incompatibility of the supplied prefs rather than missing assets, without requiring the failed debugger experiment. It does not recover the original identity or prove all decoded settings valid under a substitute identity. Private reproducible script/results: staged-inotia/check-prefs.py and preference-format-analysis.json. The normal APK and test APK were rebuilt/restored, and the diagnostic forwarding port was removed.

## Music Factory media attribution

A focused native media trace records MC_mdaClipCreate with a non-null guest callback (0x14335), successful data submission and MC_mdaPlay at the note-play scene. The existing host adapter discards the callback. Independent ELF disassembly shows event 2 records current time in the guest sound object and forwards the event to its listeners; other handled events record elapsed time. This makes a missing playback notification a concrete candidate, not a confirmed full API contract. Private evidence: music-media/media-callback-disassembly.txt and media-trace-226.

A temporary, explicit experimental-native-media-start feature records the callback and invokes event 2 after successful Play. Its synchronous dispatch is an attribution experiment only: callback ordering, stop/free/replay cancellation, completion and other games still require validation before a production implementation.

The synchronous start-notification experiment makes notes descend, register misses and reach Game Over instead of the empty playfield. It is replaced by a queued implementation: callback address, generation, outstanding notifications and closed state live in the guest clip. Stop, replay and data replacement invalidate stale starts; freeing retains the allocation until outstanding callbacks drain, preventing address-reuse delivery. A callback that closes its own clip is released after return. Freed/replaced audio handles are closed as well. Four new lifecycle tests pass, and the combined WIPI C / KTF / LGT suite passes 165 tests. Completion/pause/resume notifications are not asserted implemented by this start-event fix. Normal APK gameplay verification follows.

Normal queued-callback APK reproduces descending notes and Game Over under the recorded tutorial sequence (media-queued-normal-226). Audio collection yields nonempty MIDI/WAV packets; the tutorial MIDI is 31,285 bytes with a 71,232 ms duration. A separate Android MediaPlayer test prepares and plays that captured file, reporting position 1,957 ms after two seconds and playing=true. This verifies decoder/playback progression, not speaker audibility or audio/visual synchronization. KBO 2009 remains Running in a fresh shared-media regression probe; LGT Gamevil 2010 Baseball remains at zero paints and is unresolved. The temporary synchronous feature/build hook was removed; the shipped implementation queues notifications.

The final normal-speed probe switches to 1× at sample 054, before leaving the tutorial instructions. Samples 078/080 subsequently show notes at different positions; missed notes reduce HP and the run ends at Game Over. This verifies the no-notes fix at 1×, not merely a 1× settling frame after faster play. Evidence: media-normal-speed-226. The normal APK remains installed; no game files or regular saves were modified.

## Baseball bundled-startup-data isolation

Private diagnostic copies retain the original game/JAR bytes while varying only P/ companion entries. Original archive and regular saves are untouched. The original archive and an archive containing only P/game_o.sav both remain black with zero paints. Removing game_o.sav alone still remains black. Removing both game_o.sav and game_rg.sav reaches 237 paints before a guest stop with a green screen; removing all companion entries reaches the opening notice (220 paints in the shorter probe). These differing input sequences/counts are startup observations, not performance comparisons.

The trace reads game_o.sav (2776 bytes) and game_rg.sav (294 bytes) before the first stalled paint. A fresh game-generated options record is also 2776 bytes and has the same leading length field, so there is no evidence to strip a generic four-byte wrapper. The results isolate a bundled-data-dependent failure, but do not establish corrupt source data versus incorrect emulator interpretation. No production data-stripping policy or game-specific runtime branch was added. Further guest decoding/authentication investigation is required. Evidence: only-options-206, without-options-206, without-options-registration-206, without-companion-206.

## Cinema Tycoon protected-field receiver attribution

A diagnostic rescue replay reproduces the same fault and records bounded register-referenced memory without modifying execution. Guest function 0x1547a0 resolves a field, derives the receiver class reference by arithmetic-shifting the object header right five bits and adding the JVM context base, and calls access checker 0x154688. The protected-field path walks class descriptors at +8.

Observed receiver 0x48432240 contains fields pointer 0x4a855900 and real class 0x17a410. Its fields header is 0x1e00, decoding to offset 0xf0. The derived reference 0x401000fc contains adjacent vtable pointers; its +12 entry is the real class's vtable 0x4a856500, but its +8 entry is another vtable, not a descriptor. The access checker therefore follows method-table data as class metadata and faults. This is stronger evidence than a generic stack-pointer corruption hypothesis: the receiver header currently provides the context-relative vtable representation created by get_vtable_index/instantiate, while this guest access path requires class metadata.

No access permission was forced to succeed and no null/error was swallowed. A repair must preserve virtual dispatch and class/parent identity as well as protected access, including runtime-defined classes. A simple global change from vtable indices to shifted class pointers is not justified: the signed compact offset must be representable relative to the JVM context, and native-image and host-generated class allocations occupy widely separated guest regions. Evidence: metadata-trace-106 and game-106/protected-field-check.txt (private artifacts).

## Canonical KTF class-header repair

Runtime-generated class records now live in a guest-backed 1 MiB metadata arena at 0x02000000, with its first page reserved for the native JVM context. Native-image classes retain their original canonical addresses. Both are within the signed 27-bit relative offset represented by an object header shifted left five bits; object fields, descriptors and methods remain in the ordinary heap. This replaces the per-object vtable-table index with a reference to the actual class record. The arena cursor lives in guest support memory, not a host registry. Native images overlapping the reserved arena and class offsets outside the encoding range fail explicitly.

Three new tests cover native/runtime address round-trips, allocation separation/preservation/exhaustion, and independently assembled guest Thumb probes reading a parent descriptor and virtual-method entry through encoded headers. KTF 40 unit tests plus one integration, WIPI C 86, and WIPI Java 69 pass (196 total).

Cinema diagnostic rescue class-layout-audit-106 passes (84 paints) at the cinema-opening confirmation. class-layout-next-106 additionally confirms and reaches the next town dialogue (117 paints). This verifies the reported failure and next action, not a full playthrough. Survival Diary, Harvest Tycoon, KBO2009 and Fruit Tycoon Final diagnostic rescues also pass. Darkside Story's older rescue reports replay divergence before the recorded failure and is not counted as passed or classified as a new regression without a fresh-state comparison. The initial class-layout-106 run used the normal APK, which intentionally excludes diagnostic rescue-capture support; its Unknown checkpoint action result was a harness/build mismatch, not a game result.

Fresh class-layout probes for KTF Rhythm Star (455 paints) and Zik (112 paints) remain Running. Darkside reaches save selection (514 paints); this does not validate its reported room-exit failure. Paint totals are bounded-probe observations rather than FPS or full compatibility claims.

The final normal APK was rebuilt and installed after the class-layout tests. Darkside's normal-build fresh probe also remains at empty save selection under repeated OK input; that sequence does not enter a new game and cannot resolve the old rescue divergence. Next investigation must select the actual new-game path and reproduce the room exit, rather than treating repeated confirmations as sufficient coverage.

## Darkside fresh-state door validation after class-layout repair

The actual new-game menu requires Down after opening the menu; repeated OK selects Continue and cannot start an empty slot. The corrected sequence reaches the introduction, skips with #, and enters the starting room. At 1×, four Down presses, six Right presses and five Down presses reach the bottom-right door from the observed starting position. Earlier movement probes hit a plant/wall and are retained as navigation misses, not transition results.

The current normal APK reproduces the address-zero failure on that door transition (darkside-door-aligned-19). The focused diagnostic build reproduces it too (darkside-door-trace-current-19): map/field1/object/11.png lookup is immediately followed by MC_grpGetImageProperty(0, 4), failing at native caller 0x11186d. This confirms the old missing-resource blocker independently of the divergent historical rescue and shows it remains after the class-layout correction. Both previously independently obtained distributions have the identical inner JAR SHA-256 dc16d96ada90d5a0d1e72a93a998866210e4addfb703731cb3e59b75b5399f20 and omit that member. No intact copy has been recovered; this does not prove the file is lost everywhere.

Per the user's instruction not to spend indefinitely on unavailable additional data, this case is deferred pending an intact package or established target-platform null-image contract. No substitute artwork, guessed dimensions or forced success was introduced. The tested normal APK was preserved from the installed package before diagnostics and restored byte-for-byte afterward; regular saves and source archives were untouched.

## Merhen dialogue overlap reproduction

The normal APK reproduces the reported dialogue defect after skipping the intro with CLR (merhen-skip-57). Speaker name remains readable, but successive body pages visibly retain prior text. Trace merhen-text-trace-57 shows per-character draws at y=170 and y=184, Korean x advances of 13, and normal decoded strings. Thus the observed overlap is not established as a line-spacing defect or corrupt Korean decoding. Space-plus-NUL strings advance one pixel in the game; this separate measurement detail is retained without assuming it causes the retained prior pages.

The broader merhen-background-trace-57 records repeated text-only frames after the last image drawing, with resets/clip restoration but no later fillRect/copyArea/image clear call. CardCanvas currently paints stacked cards onto the same underlying graphics. The [KTF Card reference](https://nikita36078.github.io/J2ME_Docs/docs/KTF_WIPI_API/org/kwis/msp/lcdui/Card.html) describes transparent cards drawing after the component underneath, but the exact application redraw/layer behavior still needs attribution before changing shared compositing. No speculative font shrinking or global framebuffer clearing was introduced. Normal APK restored after diagnosis.

### Merhen redraw attribution correction

merhen-card-trace-57 records exactly one card throughout the dialogue (stack index 0/1, at 0,0), with one initial push/show notification. This refutes stacked-card compositing as the current explanation; no compositor change is justified by this case. The fuller graphics log also contains dialogue-panel fillRect calls at the initial page transition, correcting the overly broad statement that no clearing occurs. Subsequent sampled frames only draw text. An all-1× sequence with longer pauses is being compared against accelerated navigation before assigning the defect to rendering or changing guest timing. Temporary per-card debug logging was removed after collection.

### Merhen input-timing comparison

All-1× dialogue with 1.5-second pauses between confirmations produces clean sampled pages (merhen-one-speed-57, diagnostic build). On the normal APK, fast confirmations at 1× reproduce overlap (merhen-normal-fast-one-57), whereas 2× with 1.5-second pauses produces clean sampled pages (merhen-normal-slow-two-57). The normal captures rule out speed alone as the cause. These sequences advance different numbers of completed text pages because a confirmation can finish typing or advance; the claim is limited to visible overlap versus clean transitions, not identical frame hashes.

The actionable lead is early confirmation during the text-reveal/line-skip sequence and the absence of a subsequent background redraw. It is not yet established whether guest state handling or emulator callback ordering causes that omission. Waiting for text to finish is a temporary workaround; no debounce, forced clear, font scaling or timer alteration was installed. The normal APK remains installed, and per-card diagnostic source logging was removed.

### Merhen callback narrowing

The 1× fast-input diagnostic identifies native paint entry 0x10ed21 and keyNotify entry 0x109499 (merhen-callback-trace-57). Occasional synthesized repeat notifications occur around the 100 ms hold boundary, but independent disassembly of the handler shows type 3 returns without invoking either state-changing press/release branch. Repeats are therefore not a supported explanation for this game's dialogue defect. Press and release invoke distinct encoded method references; resolving their post-startup metadata is the next step before tracing the redraw state. File references are encoded and must not be treated as direct callback pointers. Private evidence: game-57/input-handler-disassembly.txt. No input debounce, repeat-delay, rendering or timer policy was changed.

### Merhen receiver-state trace

The post-initialization method table resolves the press handler to a(I)V at 0x108981 and the release handler to d(I)V at 0x119fa9. The receiver-field trace records the actual instance state at each key boundary (merhen-fields-57); the release handler guards its state change on a mode value of 16. Resolving that field and its relationship to dialogue redraw remains necessary. This narrows the investigation but does not establish a faulty emulator API or justify a redraw patch. Fast-confirmation overlap remains open with the measured wait-for-text workaround. Temporary method/field metadata logging was removed after collection; no gameplay policy changed.

## Welusia grayscale polarity correction

The remaining broken Korean is now attributable to LBMP type-2 grayscale polarity, not text decoding. The supplied white and black bitmap-font pairs have identical visible glyph counts for cho1/cho2/cho3/jung1/jung2/alpha (644/651/254/459/173/1057 pixels). Every visible white glyph pixel encodes level 0; every corresponding black glyph pixel encodes level 3. The prior decoder mapped those levels in the opposite direction. The white final-consonant image uses type 8 instead, explaining why only final consonants were visible on the black dialogue background.

Corrected type-2 decoding to map darkness level 0 to white and level 3 to black, retaining the independently decoded transparency plane. Intermediate levels reverse consistently. A generated white/black glyph regression checks polarity and unchanged transparency; the existing partial-row/plane test now asserts the established polarity. All 66 wie-backend library tests pass. Normal-APK Android validation passes: the identical 1× boot/input sequence now displays complete readable Korean syllables, with the original 107 paints and Running status preserved (welusia-polarity-448/sample-012.png versus welusia-current-448/sample-012.png). Private evidence: welusia-current-448 and game-448/ah-javap.txt. No supplied game artwork was modified or added to source.

The grayscale-corrected normal APK is installed on emulator-5556, temporary Merhen metadata diagnostics are removed, and the environment is returned to the library. Remaining failures above are still open; this is not a full-compatibility declaration.

### Dragon Eyes connection call-site check

Independent inspection of the two native calls into the known Net.connect wrapper (0x103fcc and 0x1042c2, wrapper 0x104f08) finds state/busy/retry gating, not a missing-file error at those call sites. This supports the earlier trace's narrower conclusion: recognized complete data still enters a network state machine. It does not establish the server protocol or justify synthesizing successful authentication/download completion. Alternate recovered data remains identical. Defer replacement-data searching unless a new distribution or setup reference becomes available; the case remains unresolved rather than declared permanently impossible. Private evidence: game-37/connect-callers.txt.

## Stationary-scene performance baseline

Added processCpuMs to the existing test-only compatibility sample alongside monotonic elapsedMs and paint counts. This measures aggregate application CPU (including small harness/image-sampling work), not exclusive CPU-engine time; it is not a production timing/scheduler change. The Android test APK rebuild passes.

Normal APK, 1×, three consecutive ~5-second windows: Astoni KTF 116 at the opening-room dialogue produces 4.80/4.79/4.60 paints/sec and consumes 98.7/98.9/98.9% of one CPU core. Mangu KTF 53 at its illustrated opening dialogue produces 5.39/5.39/5.20 paints/sec and consumes 98.6/98.7/98.5%. These establish CPU-heavy stationary scenes, not full gameplay FPS or original-device timing. Earlier selection-menu and transitioning-intro samples are excluded from this stationary comparison. No performance patch or pacing change is justified until attribution. Private evidence: performance-cpu-53, performance-cpu-116, stationary-performance.json.

### Astoni allocation attribution and candidate

An 8-second simpleperf sample of the same stationary normal-APK scene recorded 1,579 samples with zero lost: 12.86% allocator allocation, 6.21% quarantine/deallocation, 5.00% deallocation, 9.50% mem_read_c_string, and 5.64% Arm32CpuEngine::run. These are process-wide samples, not exclusive callback timings. They motivate a Java metadata lookup candidate rather than another instruction-dispatch change.

The candidate compares each freshly read guest method/field name as borrowed descriptor/name slices instead of allocating two owned substrings for every scanned entry. No metadata cache, game-specific branch, timer change, or guest instruction reduction is introduced. The differential regression compares owned versus borrowed lookup, UTF-8 names, signature mismatches, live guest modifications and a terminator at the actual 64KiB mapped-page boundary; its initial 4KiB-boundary assumption was corrected before passing. All 41 KTF library tests and the integration test pass. The normal candidate APK builds successfully; retention is pending controlled Android comparison.

The unactivated stationary samples above are diagnostic only. Control/candidate acceptance uses the established reversible AVD host activation before each subprocess, retained host-state records, and identical 1× fresh sequences. Do not combine the earlier unactivated numbers with that controlled comparison as a speedup claim.

First activated control/candidate stationary pass: Astoni wait windows 4.80/4.79/4.60 versus 5.60/5.39/5.20 paints/sec; Mangu 5.40/5.40/5.19 versus 6.59/6.20/6.39. Both candidate captures show the same dialogue scene and intact text/artwork. Reverse-order repetition is pending, so these initial gains are not yet the acceptance result.

### Retained lookup candidate: controlled ABBA result

All eight processes completed Running. In control1 → candidate1 → candidate2 → control2 order, per-process median paints/sec were:

| Scene | Control 1 | Candidate 1 | Candidate 2 | Control 2 |
|---|---:|---:|---:|---:|
| Astoni opening-room dialogue | 4.79 | 5.39 | 5.39 | 4.60 |
| Mangu opening dialogue | 5.40 | 6.39 | 6.39 | 5.39 |

Every candidate process median exceeds both controls for its scene. All three preselected 5-second windows per process and host activation records are retained; there are no discarded measured windows. Across both passes, pooled median paint rates improve from 4.70 to 5.39 (+14.8%) for Astoni and 5.39 to 6.39 (+18.5%) for Mangu. These are stationary scene presentation rates, not deterministic callback CPU speedups or full gameplay performance. CPU consumption remains near one core and severe lag is not resolved.

Retain the allocation-reducing name comparison after the 42 passing KTF tests and repeatable two-scene gain. Guest metadata remains read on every lookup; no cache invalidation policy, timers, refresh caps, event scheduling or game data changed. The tested normal candidate APK is installed and the environment is returned to the library. Next performance investigation should attribute remaining metadata-read/allocation costs with the same controlled workload before another candidate. Private evidence: name-match-comparison.json and name-match-{control1,candidate1,candidate2,control2}-{53,116}.

### Post-lookup stack attribution

On the retained lookup build, an 8-second call-stack sample collects 1,576 samples, zero lost. Inclusive stacks include setRGBPixels (65.48%), drawRGB (36.48%), field lookup (21.57%), method lookup (22.21%), and field descriptor decoding (11.80%). These overlap and must not be added together. This is a profiled scene, not a timing comparison. It points to graphics API/JVM bookkeeping rather than assuming all work is ARM interpretation.

A separate small candidate removes the duplicate field-descriptor parsing within JavaClassInstance::get_field: the already-read field_type controls both width and decoding. No writes or asynchronous guest execution occur between those uses. Each subsequent get_field still rereads metadata. A regression changes a live word field from I to F and a wide field from J to D and back, checking bit-exact integer values and floating-point decoding; all 42 KTF library tests plus the integration test pass. Retention is pending measurement against the previously retained name-lookup build, not the older baseline.

### Descriptor-once candidate archived

Initial five-second windows showed a borderline Astoni gain, so a separate confirmation uses one 20-second window per process with the same fresh scene, 1× speed and host activation. The longer window improves count resolution; its results are not pooled with the short-window medians. In candidate1 → control1 → control2 → candidate2 order:

| Scene | Candidate 1 | Control 1 | Control 2 | Candidate 2 |
|---|---:|---:|---:|---:|
| Astoni | 5.50 | 5.25 | 5.30 | 5.45 |
| Mangu | 7.05 | 6.30 | 6.45 | 6.95 |

Mean long-window improvement is 3.8% in Astoni and 9.8% in Mangu. The candidate is correct in the targeted tests but does not clear the conservative ~5% screen in both workloads. Reverted the runtime change and restored the previously retained name-comparison APK. The descriptor-mutation regression remains useful and is retained. All measured runs and the prototype are archived privately; do not re-test this isolated change without new evidence. Severe lag remains unresolved. Remaining attribution points to graphics/JVM metadata work, especially setRGBPixels/drawRGB and field/method lookups, rather than assuming run() is dominant.

The retained descriptor-mutation regression also passes after reverting the runtime candidate (one selected test, no failures), confirming it checks behavior shared by the baseline and candidate.

### Field-wrapper materialization candidate

The next isolated candidate retains the full guest field-pointer-table read, but does not construct an owned JavaField/core-reference wrapper for every entry before scanning names. It reads each candidate's raw metadata and only wraps the returned match. Field enumeration for callers that need all entries retains its original interface. No host cache or changed table-fault policy is introduced. The regression compares lookup with enumeration for static/instance fields, missing names and signature mismatches; live type changes are observed, and a missing table terminator still faults before returning an otherwise matching first entry. All 42 KTF library tests plus the integration test pass. Normal APK measurement is pending.

### Field-wrapper candidate archived

Twenty-second controlled windows show Astoni candidate 5.40 versus control 5.45 paints/sec, and Mangu candidate 6.65 versus control 6.35. It fails the two-workload screen, so the runtime candidate is reverted without a further sweep. The existing accepted name-comparison APK is restored. The lookup/malformed-table regression and all measured artifacts are retained. Do not treat wrapper allocation alone as an established valuable target; the stack profile's inclusive field-lookup cost includes other work.

## SKT authentication attribution: NoriMax and Chaos Blade

Both supplied games contain their own SecureUtil validator. Independent bytecode inspection shows a local comparison between MIDlet-Key and a digest of normalized phone identity, a MIDlet-Jar-URL service identifier, and the validator's constant; this path does not download game assets. The bundled key is present in both MSD files.

Executed the supplied validator classes on the host JDK with a minimal property-providing MIDlet stub and the same system-property values as GOmul. Both reject their supplied keys. An explicitly synthetic, in-memory positive-control key is accepted by both; original archives, metadata and the Android app were not rekeyed. This checks that the standalone validator runs correctly and narrows the rejection to the configured identity/key combination, rather than silently treating a host exception as validation evidence. It does not establish the original matching identity.

A bounded scan of readable original archive members (including inner JAR members) for ASCII/UTF-16 phone-form strings finds no candidates in either package. A directory-header slash mismatch was skipped as a directory; all file members were readable. This does not prove an identity cannot be recovered from another source or encoding. A matching original identity/key pair or a documented installation mechanism is still needed for faithful startup. Private evidence: game-341/secure-javap.txt, game-139/secure-javap.txt, reference-validation.txt and identity-scan.json in each directory, and auth-reference/.

## Existing Android save comparison

Added test-only initialSave copying to the compatibility runner. It boots isolated cache copies, and before/after per-file SHA256 checks verify all original saves remain unchanged: Baseball 206 (3 files), Zik 424 (108), Fantasy Forever 2 178 (66). The current Rhythm Star save folder is absent.

Baseball remains fully black with zero paints. Zik reaches its publisher logo, then remains at 112 paints through the bounded wait/input sequence; this corrects the earlier overly optimistic interpretation of its Running status. An attempted longer wait exceeded the harness's 30-second-per-step limit and is excluded from game results.

Fantasy Forever 2's existing save reaches the additional-download notice and returns to that flow after selecting continue. Its file set matches the previously working fresh-save capture: the sole byte difference is offset 28 of PD004956/db/common/1 (fresh 1, existing 0). The supplied common.db remains intact, and all other saved assets match. Replacing only the copied common record with its unmodified supplied export restores town gameplay/dialogue (211 paints, normal APK, 1×). The assets are available; the existing download-state configuration causes this startup difference. This is a demonstrated local recovery, not an automatic runtime fix or a claim that resetting arbitrary progress is safe. No original saves or authentication metadata were modified, and no automatic overwrite/migration is justified from this observation alone. Private evidence: existing-save-{206,424,178}, existing-save-continue-178, original-save-verification.json in each initial probe.

### Zik publisher-logo freeze confirmed

With the existing-save copy, 30 + 15 + 5 seconds of additional waits and OK/1 presses leave the frame and paint count fixed at 112. Aggregate process CPU rises by about 37.6 seconds across the consecutive 30- and 15-second waits; this is diagnostic-build activity, not a performance benchmark. No fatal exception appears in the bounded log.

The API-boundary trace identifies an unfinished paint call: the last Display.handlePaintEvent / Canvas.paint entry at 07:37:43.944 has no return, and EventQueue.dispatchEvent likewise does not return. Other guest work keeps requesting repaint every ~80 ms and sleeping/yielding, while input/paint dispatch cannot advance. This narrows the freeze to the active guest paint path rather than proving a missing download or dead application process. The exact native loop/await remains to be attributed; no timeout, forced return, event-thread concurrency or guest timing change was introduced. Private evidence: existing-save-diagnostic-424 and zik-boundary-trace-424.

### Zik monitor-starvation candidate

A narrower trace resolves the unfinished native Card.paint entry to 0x10e371. It enters the Card monitor (0x484367e0), then attempts the shared monitor 0x48436890. Another guest thread repeatedly exits and immediately reenters the latter around its 20 ms sleeps. The current monitor wakes a contender but permits the exiting owner to reacquire before that future is polled.

A deterministic direct-poll regression reproduces that barging on the unchanged monitor (expected pending reacquisition is instead ready). The candidate queues contended acquisitions, preserves immediate owner reentrancy, and removes cancelled tickets so they cannot block successors. Existing wait/notify/reentrancy tests plus both new starvation/cancellation tests pass (6 total) in a private harness importing the exact vendored monitor source. Broader KTF/MIDP tests and normal-APK game validation are pending. This changes contended monitor admission, not CPU scheduling, guest timer delays or rendering policy; no game-specific bypass is introduced.

### Zik normal-APK validation

All 42 KTF and 48 MIDP library tests pass with the queued monitor candidate, in addition to the 6 direct monitor tests. The normal APK passes the identical existing-save sequence that previously froze: 279 paints and visible town gameplay versus 112 paints at the publisher logo. A second launch reaches town again and responds to RIGHT/DOWN/LEFT inputs, with visible character/camera movement (286 paints). Original Zik/Fantasy save hashes are rechecked unchanged.

Retain the generic monitor progress fix. It prevents an immediately reacquiring thread from starving a queued contender, with cancelled-ticket cleanup and unchanged reentrant ownership behavior. No game archive, save, timer delay, CPU scheduler or rendering policy was patched. Welusia retains readable Korean at the same 107-paint opening sequence. Music Factory continues note/game progression and reaches Game Over (1,624 paints) using the prior bounded sequence; the final 1× wait continues presenting. These smoke checks pass, without claiming complete gameplay or revalidating physical audio. Normal APK artifact SHA256: e7b8765630a5824810f1899925b76c1fb4cf7e6f2697dfef0090bedb13e3d13b. Private evidence: zik-monitor-candidate-424, zik-monitor-movement-424, candidate-monitor-fairness.apk, monitor-harness, and monitor trace directories.

Post-monitor follow-up: Rhythm Star again passes startup and reaches Game Over (460 paints). Merhen still exhibits overlapping dialogue after the identical fast 1× confirmations; monitor admission does not resolve that defect. Private evidence: post-monitor-open-45 and post-monitor-open-57.

## Baseball idle black-screen attribution

The diagnostic-only cpu-boundary-trace feature records PC/LR at exhausted 10,000-instruction run boundaries. It is enabled by compatibility-audit, filtered under cpu_boundaries, and compiled out of normal builds. It does not change the instruction budget or yielding policy. The diagnostic APK builds; the feature-enabled core tests pass.

Baseball records 366 exhausted-budget boundaries during startup, all before 07:55:16.945. Afterwards process CPU increases only modestly while the screen remains at zero paints. This rules out sustained guest CPU spinning in the sampled black-screen interval. Separate event logging confirms OK/1/RIGHT press/release reaches CletWrapperCard::keyNotify, so event delivery is not globally blocked.

Independent ELF disassembly shows paintClet at 0x11fd delegates through 0x90cc1, whose two external calls correspond to the observed stub unk1 and unk11 calls (LGT slots 0x6a and 0xee), passing event-like value 0xa600. These calls currently return zero. The ELF has no symbol table providing API names. This is a concrete unimplemented path to investigate, not sufficient evidence to assign a full API name, inject a successful event, or declare supplied startup data corrupt. No guessed mapping or game-specific bypass was installed. Private evidence: baseball-cpu-boundaries-206, baseball-event-boundaries-206, baseball-startup/callbacks-disassembly.txt and paint-dispatch.txt. Normal monitor-corrected APK restored after diagnosis.

## Opaque pixel rendering candidate

Code inspection rules out the row-copy allocation candidate: put_pixels currently has no production call sites. That prototype and its test were removed before any timing run. The active draw path instead calls compose_pixel with blend=true for fully opaque RGB pixels, reads/allocates a destination pixel, and performs floating-point blending with factor 1.

The isolated candidate writes a fully opaque source directly after the existing bounds/XOR handling. Fully transparent and partly transparent paths retain their prior behavior, including destination-alpha handling. A counting-buffer regression verifies zero destination reads for opaque draws across all byte values, and retained reads/results for alpha 0/128. All 67 backend tests pass. Normal APK timing against the monitor-corrected control is pending; this is not yet a retained optimization. No guest workload, timers, refresh caps or speed setting changed.

### Opaque pixel candidate rejected

The normal candidate passes 67 backend tests and 48 MIDP + 69 WIPI Java tests. In the activated 20-second comparison, Astoni control and candidate both produce 5.20 paints/sec with ~98.6% single-core process usage. Mangu takes a different path through the timed key sequence: the candidate is in scrolling intro text while control remains in the stationary dialogue, so its apparent 6.3 → 8.7 paint-rate difference is invalid as a performance comparison. Do not treat that as a gain or a rendering regression without a matched scene.

Reverted the opaque shortcut and its candidate-specific read-count test, archived the full prototype and all runs, and restored the monitor-corrected normal APK. Do not repeat this isolated candidate without new evidence or a genuinely improved workload fixture. Private evidence: opaque-pixels-comparison.json, opaque-pixels-prototype.patch, opaque-pixels-{control1,candidate1}-{53,116}.

### Baseball direct-event wrapper comparison

Independent decoding additionally resolves the wrapper at 0x90cad to a direct call of the game's handleCletEvent (0x10b1), followed by a true return. The separate paint wrapper still uses the external 0x6a/0xee pair. This supports investigating an external delivery path but does not establish synchronous versus queued semantics. Public API searches and upstream declarations did not identify the unknown slots; an archived WIPI PDF inventory request timed out. No dispatch mapping was invented.

### Inotia 1 assets-only staged installation

Tested first launch without companion data, generated current-device preferences, stopped at the consent stage, then copied only the supplied assets while retaining generated prefs. The normal APK still reaches the connection-failure/retry dialog (1,588 paints); the retained prefs pass checksum validation and decode to the expected 0x4b header. Thus overwriting generated preferences is not the only barrier: retaining valid current-device prefs does not establish completed-download state. This excludes a simple assets-only staging remedy. No identity or successful-server-response bypass was applied. Original saves and archives remain intact. Private evidence: staged-assets-preserve-prefs-135 and preference-validation.json.

### Public documentation boundary for baseball

The archived [WIPI Java EventQueue documentation](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/lcdui/EventQueue.html) describes queued Java events, but does not establish the native LGT slot identities, signatures, or synchronous/queued delivery behavior. The public J2ME_Docs inventory inspected contains Java API documentation, not the required native C contract. Narrow searches did not recover that contract. This is a documentation gap, not proof that the two calls implement Java EventQueue. No speculative mapping was added; the monitor-corrected normal APK remains installed.

### Merhen initialized field resolution

Used the existing diagnostic single-run CPU capture to retain initialized mapped memory, rather than interpreting pre-initialization references as final metadata addresses. The native helper's runtime field table is 0x19ee40; its file-time value points elsewhere before startup expansion/relocation. This diagnostic capture is not a timing result.

The release handler's mode check resolves to f.ak:B (offset 172). All 24 prior logged input boundaries have mode 2 (1), 3 (3), or 4 (20), never 16. Its guarded release mutation therefore cannot explain the sampled dialogue overlap. The mode-4 press branch instead calls f.g:Li;'s method at 0x16a585. For confirmation, it clears d.q:Z when true; otherwise, if d.h:Z is false, it compares d.o:B to the nested byte-array length and increments d.o while setting d.r:Z and d.h:Z, or advances the enclosing state at the last entry. Their typing/page/redraw roles are inferred from this control flow and still need correlation with the paint path.

This replaces the open release-handler hypothesis with specific confirmation/redraw state to follow; it does not establish an emulator defect or justify changing input timing. No runtime patch was made. Restored the accepted monitor-corrected normal APK and returned to the library. Private evidence: merhen-live-fields-57 CPU capture, game-57/read_capture.py, dialogue-press.txt, dialogue-field-resolution.txt, dialogue-flag-uses.json and resolved-dialogue-state.json.

## Inotia 1 companion identity recovery and Android wiring

The supplied prefs decode under identity 01012349876 with the expected 0x4b header, plausible settings, and five 19-bit asset lengths 45633, 163135, 182483, 76920, 155017. Each exactly matches its supplied char/map/mon/pattern/tile file length minus 44 bytes. This independent five-file consistency check supports the identity; the earlier default identity produces an invalid header and inconsistent lengths.

An isolated normal-APK test at 1×, using all original companion records unchanged and only adding phone-number.txt, passes the download retry and reaches the story introduction. This establishes startup recovery, not full gameplay. The user then requested wiring and will confirm gameplay; no further speed matrix or manual-navigation testing is required.

GameDataImport now recognizes the exact supplied P/prefs SHA256 on fresh installation and creates its per-game identity before native data import. Explicit identities and existing progress are preserved; alternate bundles are not matched by title or application name. No guest API, authentication response, game binary, or asset was changed. The user's current Android save was backed up privately, its supplied prefs restored, and the matching identity written. All five existing assets were verified byte-identical and left intact.

The Java launcher APK rebuild succeeded and was installed. It includes the accepted normal native library, not the diagnostic CPU-capture library. The environment is back at the library for user confirmation. Evidence: matching-identity-readable-135; staged-inotia/matching-identity; inotia-applied-repair save backup and asset-verification.json.

### Inotia 1 directional control profile

User reports that UP opens the skill menu and requests wiring without gameplay checks. Added a launcher input profile for the exact companion-bundle fingerprint: UP/LEFT/RIGHT/DOWN emit numeric 2/4/6/8; other buttons and other bundles retain their mappings. Physical controls routed through the same launcher input function also use the profile. Aliased direction/numeric holds retain the guest key until the final source releases it. APK compilation and installation succeeded. No automated input, screenshot or gameplay verification was performed; user confirmation of the movement mapping remains pending.
