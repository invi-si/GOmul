# KTF unattended boot fixes — current results

The normal APK containing the earlier boot fixes plus the form, menu,
deferred-image, MX arena and carrier-endpoint additions is installed on the AVD.
Its installed SHA-256 matches the delivered artifact. The automated pass below is
complete. Game archives and player saves were preserved. Diagnostic output and
game-derived recordings remain private. This is a compatibility update, not a
claim that the complete catalog is playable.

Subsequent graphics correction: [native destination offsets](ktf-native-graphics-offsets.md).
The artifact hashes below identify the earlier automated-boot release.

## Results

- Ten formerly blank cases render after fixing AnnunciatorComponent geometry.
- 북천항해기2 renders its title/description UI after fixing Card field ownership.
- 이노티아연대기2 reaches its main menu after the storage, metadata, private-data
  installation and file-removal fixes. The package's intended first-run process
  still applies: initial authentication failure, restart, then accept the next
  certificate prompt. The game itself grants temporary authentication after a
  failed server connection. No successful server response or certificate is
  fabricated. Full gameplay and future renewal are unverified.
- 짜요짜요 타이쿤4 progresses when the connection offer is declined; this is an
  input-path finding, not a repaired online service.
- All 26 warning-only cases were rerun: 25 Running, one explicit guest exit 0.
  Seven suspicious cases received detailed exception traces: all 229 observed
  throws matched guest catch handlers, and all seven remained Running. This does
  not exclude later hidden failures, but the captured warnings are not proof of
  uncaught background-thread crashes.
- The original 다크슬레이어2 rescue matches its baseline safe-prefix frame/save
  oracle on the final diagnostic build and continues without a logged error.

The first-run sequence is described by the [package supplier](https://dubigame.tistory.com/136).

## Shared implementation

AnnunciatorComponent now occupies a device-profile status strip instead of the
whole display. Card's own fields are resolved against their declaring class,
so obfuscated subclass fields cannot replace its geometry or display reference.

KTF advertises a 32 MiB logical save volume; LGT retains its existing 1 MiB
profile. Available space still subtracts repository usage. GetAccessLevel reads
the installed SLvl mask. GetExecNames honors AID/version/vendor filters and
returns the carrier's executable identifier with proper buffer checks.

P/ and p/ companion files seed mutable per-application databases once. Existing
records are preserved, and subsequent modification/deletion survives restart.
The installation marker is persisted with saves and included in replay state.
The prior filesystem view remains available; ordinary JAR resource lookup is
unchanged. Native MC_fsRemove now deletes closed private files, rejects removal
while open, and reports missing/invalid/inaccessible files accurately. Open
stream links and names live in guest memory, with a System root reference.

Diagnostic tools report exception classes and catch locations. Rescue input
sequences support bounded host waits. The boot runner translates LSOFT/RSOFT to
native L/R and rejects invalid keys. Earlier RSOFT tokens were ignored, so those
captures do not establish right-soft-key coverage; no results were relabeled.

## Validation and limitations

- Latest full workspace run: 498 passing tests; audit tooling: 17 passing tests.
  Separate production CPU-feature coverage: 86 passing tests; Android audit-feature
  coverage: 23 passing tests. These overlap the workspace suites and are not added
  together as unique tests. Final KTF rerun also passes after physical IME
  press/release/typed and focus-loss coverage was added.
- Clippy completed with warnings; no lint-clean claim is made.
- Native Card regression fails on the old field lookup and passes on the fix.
- File tests cover multiple opens, closing different list positions, preservation
  after refused deletion, successful closed-file removal, and legacy record calls.
- Companion-data tests cover existing saves, changes and deletions across
  initialization, case-sensitive names and nested paths.
- Native enumeration tests cover the five-argument SVC, Korean vendor strings,
  filters, exact/short buffers, untouched trailing bytes and invalid pointers.
- The new normal APK has compatibility diagnostics compiled out; its embedded
  build ID was checked against final Rust source. Installation verification is
  recorded in the final delivery section below.
- CPU timing, refresh caps and guest scheduling were not adjusted. This was not
  a performance benchmark. Existing replay Quick Saves retain build checks.

## Latest focused startup results

| Game | Automated observation | Remaining limit |
|---|---|---|
| 미니게임패밀리 | Longer automated sequence reaches a minigame START board without a logged error | Full play and scoring unverified |
| 액션히어로3D | Reaches a rendered 3D gameplay scene, accepts scripted numeric input; no logged error | Longer play and unidentified MX slots remain unverified |
| 드래곤로드 | Reaches the main menu and new-game story opening after a longer boot trace | Full gameplay unverified |
| 삼국쟁패2-전사편 | Reaches the opening story using a fresh rescue and paced confirmation inputs | Full gameplay unverified |

The network slot is an evidence-based endpoint-configuration interpretation, not
a recovered official API name or a successful online service. Actual connections
retain the existing offline failure behavior. No downloaded data or successful
authentication response is fabricated.

The detailed investigation, intermediate candidates and their limitations are in
[the investigation log](ktf-annunciator-and-boot-triage.md).

## Form framework investigation history

Current source supports CommandBarComponent, deferred static-image loading and
GMenubarForm/GMenuBar/GTextField integration. Command-event and ImageObserver
constants were verified from archived SDK declarations. The chronological notes
below include superseded intermediate states; current outcomes are above.

ShellComponent now implements title component/string setters, title/work getters,
and title-above-work layout. Removing a child by reference, index, or remove-all
clears the matching shell slots, so detached components are not laid out later.
The work component retains input focus when a title is installed. Replacement
uses ContainerComponent's existing ownership/cycle checks before detaching the
old title. Runtime state remains in guest Java fields.

This follows the [WIPI ShellComponent API](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/lwc/ShellComponent.html).
The profile uses a LabelComponent for string titles and its preferred height;
this is a rendering choice, not a claim of pixel-exact KTF device chrome.
All 54 WIPI Java tests pass, including the title layout/removal/focus regression
and existing full-screen/dialog tests. This is preparatory source work:
command bars, GForm/GFormBase/GMenubarForm and text-field integration remain.
미니게임패밀리 has not been declared fixed or rerun on a new APK for this change.

The shared `org.kwis.msp.lwc.Command` class is now registered, with all five SDK
constructors and four getters. Text, payload, icons and lazy resource paths live
in guest fields. Single-resource commands share their normal/active image;
separate-resource commands load each on demand and retain the returned object.
Tests cover nullable values, object identity, distinct icons, and loader call
counts/path selection using an isolated loader double. WIPI Java: 56 tests pass;
KTF: 27 unit tests plus its integration test pass. Clippy completes with warnings.

Contracts: [Command](https://nikita36078.github.io/J2ME_Docs/docs/KTF_WIPI_API/org/kwis/msp/lwc/Command.html),
[CommandBarComponent](https://nikita36078.github.io/J2ME_Docs/docs/KTF_WIPI_API/org/kwis/msp/lwc/CommandBarComponent.html),
[CommandListener](https://nikita36078.github.io/J2ME_Docs/docs/KTF_WIPI_API/org/kwis/msp/lwc/CommandListener.html).
The listener documentation names FOCUS_CHANGE/SELECT but does not provide their
numeric values; these must be established before emitting guest events.

An additional dependency is now explicit: the existing Image.loadImage returns
null. Its SDK contract requires a non-null, initially empty image and deferred
completion/observer notification, with cancellation via stopImage. Command
correctly delegates to that API, but production resource icons remain incomplete
until the asynchronous loader is implemented. The isolated loader test does not
prove that backend works. No synchronous load substitution was introduced.

GForm now provides the documented ID and explicit-geometry constructors,
final form-ID accessors, device-profile annunciator-height query, and the
makeContents extension point. Form ID is stored against its declaring guest
class. The geometry/ID/virtual-dispatch regression passes: KTF now has 28 unit
tests plus the integration test passing. This remains incomplete framework work:
GForm(boolean inflate), initializeUI/design lifecycle and the derived forms are
not implemented. Constructor-driven hook order is not inferred from the mere
presence of makeContents in a game binary. This change is not in the delivered APK.

Private native inspection located the actual game's commandAction and
makeContents method records using its relocation base. initializeUI is an external
symbol reference, not a local body. The commandAction entry overwrites the incoming
r3 argument before using it, so it cannot establish command event constants.
Locations and the binary digest are recorded privately in
mini-family-form-locations.json; no guest code is included in this repository.

GFormBase's seven constructors, title setters, background state/painting,
focus/cancel hooks, and first-paint notification are now implemented. The default
visual profile uses LabelComponent titles; changing title text preserves its
icon. A supplied custom title component survives construction and initializeUI.
Background coordinates and the first-paint flag are guest-owned. The first-paint
flag is set before calling the subclass hook to prevent reentrant duplicate hooks.
The focused child gets key input before an unhandled CLEAR reaches onCancelKey.

Two new tests check title/icon identity, custom-title retention, background pixels
inside and outside a positioned image, image removal, first-paint count, focus
notifications and CLEAR forwarding. The full KTF run passed 30 unit tests plus the
integration test. A subsequent native KTF regression also passes: the form's
String/Image/int constructor preserves the stack-passed form ID and guest
references through the actual ARM/SVC Java runtime. Clippy completes with warnings.

This does not complete GFormBase: popup/message helpers, deprecated bounds/show
hooks and menu refresh remain. The default title styling, custom-title replacement
when using string setters, child-first key routing and initializeUI behavior are
emulator profile choices awaiting live form validation, not recovered pixel-exact
vendor behavior. GMenubarForm/GMenuBar/GTextField, command event constants and the
asynchronous image loader are still pending. The installed APK is unchanged.

ShellComponent now has getCommand/setCommand, a bottom command area, and grab-key
routing. The WIPI 2.0 UI manual's setCommand description explicitly gives the
command first access to all keys when bGrab is true (not just soft keys). The
existing 1.1 API has the same signature, but its mirrored page is truncated before
these details. The newer manual supplies the routing contract used here:
https://github.com/mirusu400/wipi-wiki/blob/main/src/content/docs/v20/java-api/ui.md

Title/work/command references and the grab flag remain guest fields. The work area
fills the remaining space between preferred-height bars. Installing a command
does not steal focus. A handled grabbed event stops propagation; an unhandled one
continues to the focused child, with no duplicate delivery if the command itself
is focused. GFormBase delegates to this shell routing before its CLEAR fallback.
Removal clears the command slot as well as existing title/work slots.

The new regression checks all three areas' geometry, preserved focus, numeric-key
interception, fallback, changing grab policy on the same bar, focused-command
single delivery, and removal. All 57 WIPI Java tests and 31 KTF unit tests plus its
integration test pass. Clippy completes with warnings. The normal APK is unchanged.

Additional source research: libwipi's generated native network header/profile and
wipi-wiki's 1.x/2.0/2.2 CommandListener declarations do not establish the unresolved
carrier slot 34, MXUserMemInterf, or the numeric command-event constants. No values
were borrowed from a different native ABI and no emulator implementation was copied.

### SDK declaration recovery

The public AromaSoft WIPI 1.1.1 emulator/SDK archive linked from
[this archived SDK post](https://rahxephon.tistory.com/183) was downloaded privately.
Sevenzip extracted its classes.zip and documentation from the NSIS archive without
executing the Windows installer. `javap -constants` verified ConstantValue entries:
CommandListener FOCUS_CHANGE=1, SELECT=2; ImageObserver FRAME_END=0, IMAGE_END=1,
NOT_EXIST=-1, DECODE_ERROR=-2, OUT_OF_MEMORY=-3. No method bytecode or implementation
algorithm was copied. The SDK archive and extracted files are not redistributable
GOmul assets; they remain outside the repository. Digests, URLs and declaration
output are preserved privately in aroma-sdk-provenance.json and accompanying files.

CommandListener is now registered with its abstract callback signature and public
static final fields. ImageObserver exposes its five verified fields. A guest
static-field regression checks all seven values. All 58 WIPI Java tests, 31 KTF
unit tests and its integration test pass. Clippy completes with warnings. This
removes the constant-value blocker; it does not yet implement command widgets or
asynchronous image loading, and the installed APK is unchanged.

The archive also contains the complete 62,794-byte ShellComponent documentation,
unlike the previously truncated online copy. Use that original SDK help for the
remaining shell contracts. Installation/execution of the proprietary SDK was not
needed or performed.

### Command-bar source validation

CommandBarComponent now stores its commands, selection, listener and held-key
state in guest fields. It implements command insertion/removal, active selection,
listener notifications using the verified SDK constants, keyboard selection and
basic rendering. Regression tests cover object identity, selection after removal,
one SELECT per press/release/typed cycle, reentrant removal by a listener, callback
errors and restoration of the graphics clip. The native KTF constructor regression
also attaches a command bar and checks guest references and listener constants.

All 60 WIPI Java tests, 31 KTF unit tests and its integration test pass (92 total).
Clippy completes with warnings; the new command-bar test's redundant conversion
was subsequently removed. Appearance, invalid-index normalization and the
non-pointing keypad profile are emulator choices, not exact vendor skin fidelity.
Pointer interaction and the real asynchronous Image.loadImage backend remain
unimplemented. GMenuBar/GMenubarForm and text input dependencies are still pending;
these tests do not establish that Mini Game Family boots. A diagnostic APK rerun
is the next check. No production APK promotion is implied by these source tests.

The diagnostic rerun completed on emulator-5556 with two boot samples and
OK,DOWN,OK,R,1 input, using disposable audit saves. Verified native logs identify
the current first failure as `No such class: com/ktf/kfc/GMenubarForm`, followed by
native initialization returning 0xffffffff; all five captured frames had zero
paints. Private evidence is in command-bar-boot/result data under the 20260912
fixes directory. This is an unresolved vendor framework dependency, not evidence
of a successfully booting game or a new graphics regression. Restore the prior
normal APK after this diagnostic run; keep the new framework code source-only
until the remaining form/menu dependencies can be validated together.

### Deferred WIPI image loading (source only)

The original SDK Image.loadImage contract requires returning a non-null empty
image (0x0), decoding later and notifying ImageObserver after completion. The old
implementation returned null. It now schedules an internal Runnable on the
existing guest event queue, decodes through the existing MIDP resource loader,
installs the resulting pixels into the same immutable WIPI image, then reports
IMAGE_END. Missing resources, decoder/IO errors and guest out-of-memory exceptions
map to the verified observer codes; unrelated runtime failures still propagate.

All pending references and cancellation links live in guest fields. stopImage
cancels matching pending jobs and releases their references; queued canceled jobs
become inert and never notify. Null observers can load images, while stopImage(null)
does nothing. Graphics.drawImage treats an undecoded empty image as having no
pixels rather than passing a null backing image into MIDP. This adds static-image
loading, not animated GIF playback: play/animation support remains unresolved.

The lifecycle regression substitutes only the resource decoder, keeping real guest
images, event-queue storage and observer dispatch. It verifies deferred dimensions,
image identity, immutable state, unchanged pixels when drawn before completion,
all three error statuses, no duplicate callback, cancellation among unrelated jobs,
anonymous loads, cleanup after decoder/observer exceptions and null-path rejection.
It does not constitute a real-resource APK or animated-image test. All 61 WIPI Java
tests, 31 KTF unit tests and its integration test pass (93 total). Clippy completes
with existing warnings and no warning in the new loader. The installed normal APK
is unchanged; GMenubarForm/GMenuBar/GTextField remain pending.

The KTF GMenubarForm mirror was checked against its raw GitHub source: both end
mid-description at 32,754 bytes. This is source truncation, not an incomplete
download. Its constant-values.html is absent (404), so vendor menu-position values
have not been inferred from similarly named WIPI constants.

A separate native KTF regression now passes through the actual ARM-backed JVM:
loadImage creates a guest image and queues one Runnable, the real resource loader
reports a missing file, the job completes without an uncaught error, dimensions
remain zero, and both the pending root and task-held image reference are cleared.
This uses the real resource loader rather than the host lifecycle test's decoder
double. It verifies the native missing-resource path, not successful native pixel
decoding or APK gameplay. Log: gomul-native-image-test.log (private temporary log).

### Binary-guided menu investigation resumed

The user's authorization to infer missing contracts from the game binary produced
new evidence beyond the truncated SDK mirror. Applying the guest's two relocation
passes and resolving its compact method-reference table identifies makeContents
calling getGMenuBar, then setItem(int,String) with 0="모드", 1="확인", 2="취소".
It subsequently calls setIMEButtonPos(0). Private mini-family-menu-contract.json
records the binary digest, call PCs, method descriptors and string-table evidence.
These establish the slot values and IME role; left/centre/right physical ordering
remains an inferred keypad profile, not recovered SDK ConstantValue metadata.

GMenuBar now has a source implementation for three persistent slots, string/Command
setters, item lookup, removal, listener lookup and physical-key selection. Empty
slots retain their positions. Its parent renderer/width calculation tolerates empty
slots. Physical activation uses SELECT without emitting a focus-navigation callback;
press/repeat/release/typed delivers one selection. The regression checks all three
keys, listener payload identity, removal without moving the right slot, painting
with gaps and removal during a held key. All 33 KTF unit tests, its integration
test and 61 WIPI Java tests pass (95 total); clippy completes with existing warnings.

The three-slot capacity, invalid-position exception and physical-key policy are
explicit emulator-profile choices requiring live validation. Vendor skin images,
public constant declarations, GMenubarForm initialization and GTextField/IME
integration are not complete. No game-specific code branch or archive modification
was introduced. The installed APK is unchanged; this is progress toward the failed
boot, not a claim that Mini Game Family now boots.

## Integrated forms and vendor calls

GMenubarForm now creates and attaches its command bar, preserves an existing
listener, supports the documented constructor signatures, and routes the IME key
to a focused GTextField. GTextField stores its mode, caret and focus association
in guest Java fields. Per-field Korean/uppercase/lowercase/numeric mode selection
does not mutate the user's global input setting. Focus reset and numeric
constraints are handled. These are functional profile implementations, not
pixel-exact KTF chrome or a complete implementation of every optional form API.
The native ARM/SVC regression covers constructor arguments and reference identity,
mode changes, text entry, menu label changes and global-setting preservation.
Physical soft-key press/release/typed routing changes mode once; loss of focus
clears the form association and prevents further mutation of the old text field.

MXUserMemInterf is implemented independently for the two observed operations:
initialize an arena supplied by the guest, then allocate bytes from it. The
returned pointers are used by real callers as writable object/buffer storage.
Metadata stays inside the guest arena and uses the existing ListAllocator; no
host allocation registry is introduced. Separate arenas, reset, overflow requests
and preservation of existing data are tested. Slots 2 and 3 remain explicit
unsupported operations: a pointer-shaped argument alone does not prove free().
Arena header format and edge-case policy are emulator profile choices.

The network wrapper selects a constant carrier endpoint, calls table slot 34,
ignores its result and returns 1 itself. The implementation copies the endpoint
into guest-backed support state and preserves the return registers. Native SVC
tests cover input-buffer reuse, replacement, aliasing the stored string and
invalid input addresses. Slots 30–33 remain explicitly unsupported. This fixes
an out-of-table call without pretending the carrier server is available.

Private binary evidence records file hashes, wrapper addresses, observed call
shapes and inference limits. Game bytes, endpoint strings, rescues and recordings
are not added to the repository. No application-name or guest-PC branches were
added to shared runtime dispatch. No CPU execution, timer, pacing or scheduling
policy was changed.

## Final automated regression evidence

The refreshed symbol scan covers all 124 installed archives. One malformed nested
archive is in the already-excluded 정통맞고2007 package. Symbol-only unmatched
members include guest-defined methods and unresolved owners; they are not treated
as proof of missing APIs or used to guess numeric slots.

Twenty-one active games referencing the changed UI classes were rerun with
isolated saves and normalized physical keys. All 21 ended Running. Fourteen had
no error-pattern log matches; seven needed exception review. All 19 observed
throws paired with guest catch handlers in trace order, with no unmatched throw.
This is bounded startup/input regression evidence, not complete playability.

DarkSlayer2's original baseline safe-prefix frame/save oracle still matches and
continuation logs no error. Action Hero 3D and Mini Game Family's old captures
can capture and repeat a safe prefix on this candidate. Those latter repeats
use a candidate-generated oracle, not an old-build equivalence comparison.
The old Dragon Road and Samguk tapes diverge and are explicitly not passes.
New recordings of both games capture and repeat successfully, including a
three-second continuation without a logged error. Cross-build tape checks were
not weakened. Fresh-boot outcomes are reported separately from replay outcomes.

The initial Samguk new-game probes accidentally selected Continue after the first
key opened its title menu. The guest deliberately exited with code 0. This was
not an uncaught emulator exception; the subsequent probe explicitly navigates
from the title screen to New Game before confirming.

The paced Samguk continuation subsequently reaches its opening-story screen and
remains Running without a logged error. Dragon Road reaches its new-game story
opening. Mini Game Family reaches a minigame START board; Action Hero 3D reaches
a 3D gameplay scene. These observed endpoints define the current automated scope;
none is presented as an end-to-end game completion test.

## Delivered normal APK

All four focused games pass the normal-APK startup/input smoke test and remain
Running with rendered frames. This check has native compatibility diagnostics
compiled out; it is not substituted for the richer diagnostic log review above.
Automatic rescue capture/export passes, including screenshot contents, game-archive
exclusion and no duplicate rescue on an intentional stop. Manual rescue passes:
paused game unchanged, per-game report, Quick Save preserved, repeat capture and
valid screenshot ZIP.

The long-lived AVD exited during the first manual-rescue invocation; no host crash
report established its cause. It was cold-restarted with the same CPU/RAM/GPU
profile and existing userdata, without wiping games or saves. The manual-rescue
rerun passed. This interruption is retained in the private test logs rather than
counted as a pass. The normal APK was verified by reading the installed APK back
and comparing its SHA-256, then the app was reopened.

- APK SHA-256: `d19c1b2ef778851429d993d35966bf38192112950479ec2b6b4b26eae377f80f`
- Rust build ID: `f3d5e4856c833b67f0ec690fce8511c950629047aa341c108b1e6c790351fc45`
- Original normal APK retained separately for rollback.
- Repository changes and reports remain local; no publication was performed.

Remaining manual scope: longer gameplay, progression, scoring, controls, audio
and visual accuracy beyond the observed scenes. The completed automation does
not establish those properties for every game in the catalog.
