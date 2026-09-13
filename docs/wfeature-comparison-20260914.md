# wfeature comparison and implementation work

Status: completed comparison, selected implementations and validation. Updated local browser assets and handoff builds are available; nothing has been published remotely.

Sections below record the implementation sequence; the final delivery audit supersedes earlier artifact-status notes.

Reference: https://github.com/movingwoo/wfeature at
`a621a6a56dbc6e8f5da967d23218ec750d73001c` (MIT, movingwoo).
Its changelog and architecture/service documents are leads to investigate,
not proof of GOmul bugs or authoritative vendor ABI definitions. Per AGENTS.md,
implementations are independent; reference code/algorithms are not copied.

## Comparison scope

| Area | Decision and evidence |
| --- | --- |
| MIDP RMS | Write validation and persistent monotonic IDs implemented and tested. Store creation, open/close counts, deletion and listing are now implemented; see lifecycle validation below. WIPI-C's different backend allocation contract is unchanged. |
| Packaged saves | KTF P/ data and LGT accompanying records already wired. SKT `.sb`/`.db` import now runs before guest startup and has actual-file and guest-visible tests (details below). |
| Handset identity | SKT MIN/m.MIN now use the provisioned Platform identity unchanged. The old default remains when no identity is configured. Property-construction tests cover both aliases and separation from archive metadata. |
| Graphics/text | Existing clipping, text baseline, framebuffer ownership and LCDUI lifecycle work retained. Added missing key-name/capability queries with guest tests; no blanket alignment or handset key-code changes. |
| Audio | Existing dependency covers handset/mobile/Softbank sequences and PCM/MIDI transport. Added browser initializer cleanup and guest-visible load failures. No oscillator replacement: no fidelity/performance evidence supports it. |
| Archive detection | Android/browser nested KTF roots now match the existing unique-root importer rule; tests cover retained data and ambiguity rejection. Existing KTF ODCF rejection retained. |
| Browser lifecycle and saves | Added per-game local backup/restore; existing safe-area and interrupted-key cleanup retained. No server-hosted sessions adopted. Production download/import and desktop/mobile layout checks pass. |
| CPU and pacing | No new candidate selected: reference architecture does not establish a speedup. Existing transcript gate and rejected register-candidate decision retained (docs/unbanked-register-avd-retest.md). Corrected timers and CPU flags unchanged. |
| GNEX/GVM SGS | No SGS outer entries found in the 229-archive corpus. No new VM selected without a matching workload; nested/encrypted payload coverage is not established and SGS support is not claimed. |

## First implementation: RMS write contract

Authoritative contract:
https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/rms/RecordStore.html

Independent changes in `wie-midp`:

- Null data with zero length represents an empty record in addRecord/setRecord.
- Validate signed offsets/lengths before the unsigned JVM array boundary.
- setRecord rejects unknown/nonpositive IDs rather than silently creating records.
- Failed backend writes surface RecordStoreException.

Regression coverage checks empty creation, slice updates, invalid IDs, signed
bounds/overflow, unchanged data/count after failed writes, empty replacement,
and invalid nonempty null input. `cargo test -p wie-midp --lib` passed all 49
tests, including the new guest-visible regression. Targeted rustfmt and
`git diff --check` also passed.

## Persistent record sequence

Implemented independently in `wie-midp/.../rms/record_data.rs` and the RecordStore
guest adapter. MIDP's next ID and live record contents share one versioned
backend record (ID zero, inaccessible through the MIDP API). A mutation writes
one snapshot, rather than committing the next ID separately from its data.
Deleted guest IDs remain consumed across reopening, including an empty store.
Backend errors do not advance the in-memory counter or change its contents.
The existing host backend's write guarantees still apply; this is not a claim
of power-loss atomicity on every filesystem.

Private static `wieWrite` uses the JVM's guest Class monitor to serialize
read/modify/write across separately opened handles. No runtime-wide host store
registry was introduced. The host database handle is locally wrapped for the
Database Send/Sync boundary; production writes use exclusive `get_mut`, not a
spinlock held over an await.

Legacy positive-number records migrate on their first successful mutation;
original bytes are retained as recovery copies. The initial counter is above
the highest surviving legacy ID. Already-deleted legacy IDs cannot be recovered
because the old format did not record them. Unknown/corrupt metadata produces a
RecordStoreException instead of falling back to stale recovery records.

Tradeoff: a write serializes the whole RMS store. This is confined to save I/O,
not CPU steps or game timers. It avoids split metadata/data commits but should
be measured before claiming any storage performance improvement. A previously
exported old build cannot interpret the new record-zero snapshot; do not use
the old build to modify an upgraded save.

Validation: `cargo test -p wie-midp -p wie-skt --lib` passed **54 MIDP + 5 SKT**
tests. Added coverage includes deletion of the highest/all IDs and reopening,
two guest handles, a second JVM thread blocked on the shared monitor, failed
host writes, malformed metadata, and retention of legacy source records.
The broader WIPI suites passed **86 WIPI-C + 69 WIPI-Java** tests. The browser
target passed `cargo check -p wie-web --target wasm32-unknown-unknown`, with an
existing unused-import warning in `wie-ktf`. Targeted formatting and diff
whitespace checks passed. These are code/build checks, not newly installed APK
or live-game validation.

## Corpus evidence for the next import step

Read-only inspection of the 229 catalog archives found three `.sb` files, all
in **대해적시대**, paired with `.db` data lengths 300, 350 and 300 bytes. All
three headers name a next ID of 2 and one live record at ID 1, offset 0. Their
declared data lengths match the paired files exactly; header lengths are 46,
47 and 48 bytes. Store names are embedded in the headers (the filenames use
`#` before uppercase letters). No `.sgs` or `.dcf` outer entries were found.
This inventory is not an encrypted-payload scan or proof of every nested format.

## Packaged SKT store import

Implemented `wie-skt/src/rms_import.rs` from the inspected header/data pairs.
It reads big-endian next ID, embedded name, version, record count, declared data
size, timestamp, and ID/offset/length entries. It validates paired files,
metadata length, signed guest ID range, next-ID relation, duplicate names/IDs,
data bounds and nonoverlapping nonempty records before installing anything.
Gaps left by deleted records and zero-length records are retained. Original
archive files are unchanged and no commercial fixture is added to the repo.

The currently supported name encoding is valid UTF-8 (including the observed
ASCII names and Korean BMP text); modified-UTF surrogate encodings are rejected
rather than guessed. Names that would become host paths are also rejected.
Version/timestamp are parsed as part of the header but not exposed by new APIs;
the existing RMS adapter still lacks getVersion/getLastModified.

`SktEmulator::load` validates packaged stores, then installs them in its startup
task **before** creating the guest JVM. `install_packaged_records` writes the
same RMS snapshot used by ordinary guest calls. Existing stores always win,
including empty/deleted-record stores. A failed new installation reports an
error and attempts to remove its newly created directory so it can be retried.
It does not remove an existing user store. Whole-store deletion now leaves a persistent tombstone so startup import does not recreate a deliberately deleted store.

Validation:

- `cargo test -p wie-midp -p wie-skt --lib`: **55 MIDP + 8 SKT passed**;
  the private-file test is explicitly ignored in this ordinary run.
- The separately invoked ignored test
  `actual_handset_store_headers_match_their_data`, with
  `GOMUL_SKT_RMS_FIXTURE_DIR` pointing to the private extracted pairs, passed
  against all three 실제 대해적시대 stores.
- Guest integration reads the imported bytes, sees sparse IDs/next ID, deletes
  every record, repeats installation, and verifies the deleted contents stay
  absent while next-ID history remains. Existing legacy bytes remain unchanged.
- Malformed/truncated headers, missing pairs, duplicate names/IDs, overlap,
  out-of-range offsets, unsafe names, empty data and sparse IDs are tested.

Further RMS APIs (enumeration, version/time, listeners and sharing permissions) were reviewed but not selected; see the final scope disposition below.
No emulator timing or CPU execution changes have been made in these passes.
The previously exported website ZIP and installed APK are unchanged.


## Store lifecycle validation

Repeated opens return the same guest object with balanced open counts. Final
close unlinks it from a guest-backed list; old handles reject subsequent use.
`create=false` rejects missing or deleted stores without silently creating them.
Deleting an open store is rejected. Listing returns live logical store names
(or null when empty), and a persistent tombstone prevents packaged data from
undoing explicit deletion. Tombstones retain recovery bytes, not secure erasure.
Path-like names are reversibly escaped. This does not resolve case-folding or
all platform-specific filename limitations for legacy ordinary names.

The two new guest integration tests cover invalid names, repeated opens, closed
handles, deletion/recreation, import after deletion, distinct escaped names,
listing and unlinking every position in the guest list. MIDP: **58 passed**;
SKT: **8 passed**, private-fixture test ignored in this run. WIPI-C: **86 passed**.
The initial WIPI-Java run exposed a test that expected the former empty listing
stub. It now asserts the actual created store name; the full rerun passed
**69 tests**. WASM cargo check passed (existing KTF unused-import warning).

## Browser audio initialization

Audio responsibilities were compared with the reference's audio documentation;
no oscillator substitution or parser algorithm was copied. A concrete local
failure path was fixed independently: soundfont HTTP failures are checked
before constructing the synthesizer, and a synth whose bank initialization
fails is destroyed and removed from the returned state. The PCM gain remains
connected, so unavailable MIDI does not disable sampled sound.

A browser-adapter unit test substitutes the external worklet and exercises the
actual initializer: HTTP 404, rejected bank, successful bank, disposal of partial
state and surviving PCM output. It passes; TypeScript checking also passes.
This is initialization correctness, not a performance or audio-fidelity claim.

These source changes are not in the September 14 static deployment ZIP, which
intentionally snapshots the previously served localhost build. No new APK or
browser gameplay build has been installed for this comparison yet.

## LCDUI input queries and remaining archive gap

The LCDUI comparison confirms that GOmul already has GameCanvas, Form/List,
Ticker, command handling and clipped curve drawing. Existing WIPI regressions
cover baseline text, offsets, source-preserving clipping, null/empty clips and
framebuffer ownership. No replacement drawing implementation was selected.

One additive gap was confirmed in Canvas: getKeyName, hasPointerEvents,
hasPointerMotionEvents, hasRepeatEvents and default pointer callbacks were
undeclared. They are now independently implemented from the Oracle Canvas
contract above. Key names describe the existing SK-VM codes in Korean; pointer
capabilities are false because host touch generates keypad events. Repeat is
true because Android and WASM both deliver Keyrepeat. Raw key values, game-action
mapping, repeat generation and timer pacing are unchanged. getKeyCode and the
remaining GAME_A–D mapping questions are not claimed implemented by this pass.

`cargo test -p wie-midp --lib`: **59 passed**, including the new guest test of
names, invalid-key exceptions, capability reporting and inherited callbacks.
The existing fullscreen/paint/key-delivery regression also passes.

Browser lifecycle inspection found existing pointercancel/lostpointercapture,
visibility key release, worker pause, and gameplay safe-area padding. These
responsibilities do not need duplicate implementations. Browser save export and import were an adapter feature gap and are now implemented below. Native snapshots remain a different format from browser save records.

Archive inspection found a concrete remaining parity gap: Python preparation
recognizes both app_info and __adf__ roots, but Android GameDataImport.normalize
and browser parse_archive only recognize app_info. Nested KTF packages should
use the same unique-root rule, rejecting ambiguous roots and retaining package
data. This gap is now implemented and covered by synthetic wrapped-package regressions (below).


## Carrier-root import parity

Browser `normalize_archive_root` and Android `GameDataImport.normalize` now
recognize both LGT app_info and KTF __adf__ markers. They establish one unique
parent directory, strip that prefix and retain all data below it. Neighboring
folders are not accidentally included. Root-level packages stay unchanged;
multiple application roots fail before Android rewrites the source archive.
Plain JAR behavior is unchanged. This follows GOmul's existing Python import
contract rather than copying reference implementation code.

Validation: **15 browser Rust tests passed**; the Android Java import test
passed including original companion identity, existing-save protection and
backup checks plus new nested KTF, companion-byte preservation, idempotence,
and mixed-root rejection cases. All **38 browser JavaScript tests passed**.
A fresh production WASM build completed successfully in an isolated output directory. The user-facing localhost server and deployment ZIP are unchanged.


Production integration: optimized WASM/webpack build passed, with the existing
KTF unused import and webpack asset-size/performance warnings. Fresh isolated
Chromium and desktop WebKit contexts ran 미니게임천국1, produced frames, handled
scripted directional input and returned to the library without page/console
errors. These are startup/transport smoke checks, not full gameplay certification
or physical iPhone results. The original localhost:8791 assets were not replaced.

The same isolated production Chromium check also ran SKT 대해적시대 (the game
containing the inspected .sb/.db bundles): 88 paints in the scripted observation
window, no page/console errors, private IndexedDB stores created, and clean
return to the library. This confirms startup integration; it does not certify
full gameplay or prove every imported record is used by a particular scene.

## Audio loading failures and optional content type

Reference audio documentation prompted inspection of the load boundary, not a
parser port. GOmul already uses smaf-player 0.1.2 for handset/mobile/Softbank
sequences and PCM/MIDI conversion. Its parse_smaf function returns an empty list
for a rejected container, so load_smaf previously reported success even when
container parsing failed.

The adapter now distinguishes a valid silent sequence from rejected data. Only
when the event list is empty does it ask the existing Smaf parser whether the
container was accepted; normal nonempty sound sequences are not parsed twice.
No stricter CRC, chunk-length, trailing-padding or dialect policy was added.
Rejected input does not allocate a handle or alter active playback. MIDP maps
the error to MediaException instead of unwrapping it. WIPI-C uses its existing
failure return and leaves the previous clip/registration intact.

MIDP Manager also accepts null content type as a detection request (SMAF is the
currently available decoder), and rejects a null stream with
IllegalArgumentException. Contract source:
https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/media/Manager.html

Validation: **67 backend, 61 MIDP, 87 WIPI-C and 69 WIPI-Java tests pass**.
Tests cover invalid/truncated containers, preserved handle allocation and active
playback, guest MediaException, detection with omitted type, null stream, and
failed WIPI replacement retaining the prior clip. Existing playback/lifecycle
tests that used empty or arbitrary bytes now use a synthetic valid silent SMAF
container; their playback assertions are retained. Clip-free tests begin their
allocation observations after freeing the fixture's input buffer.

This pass changes source after the previous isolated production smoke build;
that build, the user-facing localhost, APK and deployment ZIP do not yet include
these latest audio-loading changes. No CPU or scheduling changes were made.


## Browser-private save backup and restore

Added a Korean `저장 백업` action to the catalog. Users select a game/carrier,
export its current browser save records and filesystem data, or import a matching
GOmul JSON backup. The file includes that game's display/phone settings, not ROMs,
other games, global speed preferences, or emulator memory snapshots. This is
in-game persistent-save backup, not quick-save/rescue parity with Android.

Both operations acquire the existing per-game Web Lock, so an active emulator
cannot race a backup or restore. Restore validates identity/schema/keys/base64
and size before writing. It installs into fresh databases, waits for each
transaction commit, and only then changes the launch namespace. Old progress is
retained for the existing previous-save action. A failed write cannot select a
partially restored namespace. Game launch now reads its selected namespace
inside the same lock, avoiding stale selection during an import race.

Validation: **40 browser unit tests pass**; TypeScript checking passes. Real
IndexedDB tests in isolated Chromium and desktop WebKit contexts pass binary and
empty records, filesystem bytes, UTF-8 namespace lengths, settings, preservation
of the previous store and an unrelated game, wrong-game rejection, active-lock
rejection, and a forced actual read/write transaction abort. No user's browser
storage was touched. The checks import transpiled source modules; production UI integration is recorded below.

Backup limits: one catalog game/version per file, 64 MiB decoded record data,
128 MiB input file ceiling; requires IndexedDB database enumeration and Web Locks.
It does not import Android checkpoint/rescue ZIPs or legacy unnamespaced browser
stores. No uploads or server-side emulator sessions were introduced.

Production backup UI integration now passes: the actual catalog button opens the
selector, downloads a valid backup, imports it, activates the new namespace and
retains the old namespace. Desktop/mobile screenshots were inspected. The first
mobile run exposed native file-input horizontal overflow; a Korean import button
and constrained selector fixed it, and the repeated layout assertion passes.
The production browser build passed with the existing size warnings.

The Android native/Gradle build also succeeded. A debug-signed candidate APK is
stored privately under runs/wfeature-20260914/GOmul-wfeature-candidate.apk;
apksigner verification passed, SHA-256:
`4f07fb1fef3d9c84b1929f1c20e207d1f387653781d3fa05c867a8bf75134711`.
It contains the shared runtime and Android importer fixes, not browser-specific
UI. It has not been installed over the user's existing app in this pass.
The original localhost assets and previously delivered ZIP remain unchanged.


## Final scope disposition

The requested comparison covered storage, packaged data/identity, graphics/input,
audio, archive detection, browser lifecycle/saves, CPU/timers, and the additional
SGS interpreter. The selected changes and their validations are detailed above.
No reference source or algorithms were copied; the report retains the reference
revision and authoritative API links. Existing software/dependency notices remain
in the browser and Android artifacts.

- Further RMS methods and getKeyCode/GAME_A–D mapping: a classfile constant-pool
  scan of all **10 SKT archives / 243 classes** completed without parse errors.
  Direct RecordStore references use open/close, add/set/get, and getNumRecords.
  No references by name to the missing RMS methods or getKeyCode were found;
  the only getVersion reference belongs to gxg/DataPacket, not RMS. These are
  not selected for speculative implementation. Reflection, native numeric slots
  and future games are outside this negative-reference evidence. Reference RMS
  documentation itself says its version/time state is not retained; importing
  that behavior would not establish a correct implementation here.
- Graphics and audio rendering: existing clipping/text ownership, SMAF dialect
  support and PCM/MIDI separation overlap the reference's responsibilities.
  Do not replace them merely for implementation similarity. The concrete local
  input-query and audio-failure gaps were addressed instead.
- CPU/cache/timer changes: not selected without a new measured candidate. The
  frozen transcript gate and previous rejection remain authoritative. No speed
  or FPS improvement is claimed for these compatibility changes.
- SGS/GVM: not selected without a matching supported archive/workload. Building
  an additional interpreter is not justified by the inspected corpus. This is
  not a claim that arbitrary SGS or encrypted packages now work.
- Native checkpoints/rescue in the browser: not ported or represented as JSON
  save backups. The adopted backup feature concerns persistent game data; it is
  verified independently of native process snapshots. The comparison does not
  claim complete Android/browser feature parity.

Latest production smoke checks after the audio-loading changes repeat clean
SKT 대해적시대 startup/input/exit in Chromium and KTF 미니게임천국1 in desktop
WebKit. The separately invoked actual handset .sb/.db test passes again. The
scoped source/build checks do not certify the entire game library or an iPhone.


## Final delivery audit

All selected improvements are implemented and validated. The scope table and
final disposition cover each comparison area; unproven VM/CPU changes and unused
API extensions are explicitly not claimed implemented. No timing/CPU optimization
was adopted, and no reference implementation code was copied.

Final combined run: **307 Rust tests passed** (backend 67, MIDP 61, SKT 8,
browser adapter 15, WIPI-C 87, WIPI-Java 69). The private handset fixture test is
ignored in that regular suite and passed separately against all three real
.sb/.db pairs. Browser unit tests: **40 passed**. TypeScript, diff whitespace,
Android native/Gradle build and APK signature verification passed. Optimized WASM
build passed with existing KTF unused-import and webpack size warnings.

All **26 files** in the clean browser release match the candidate used in the
production smoke/UI checks byte for byte. The updated assets now serve at
localhost:8791; already-running workers are not interrupted and older hashed
assets are retained for open tabs. Reloading the library activates this build.
Browser saves stay at the existing origin; no personal save migration/copy was
performed. The Android candidate remains uninstalled.

Handoff outputs:
- GOmul-Web-Publish-20260914-updated.zip: prebuilt static site, selected catalog
  and thumbnails, deployment instructions, comparison report, licenses and file
  hashes. This supersedes the earlier ZIP. It contains no personal saves or
  private configuration; commercial game redistribution rights remain outside
  GOmul's license and have not been established by this task.
- GOmul-wfeature-candidate.apk: debug-signed Android build, hash recorded above.

ZIP integrity, complete file-hash validation, catalog count and local asset
references are checked during final packaging. Nothing was uploaded, committed
or published remotely. The tests do not establish physical iPhone performance,
full-library gameplay compatibility, Android checkpoint compatibility with WASM,
or hardware-accurate timing beyond the preserved existing contracts.
