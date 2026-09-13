# KTF boot triage: status-bar geometry and storage

This is an investigation log, including intermediate candidates. See [current results](ktf-boot-fixes-20260912.md) for the final state of this pass.

The unattended audit found 15 persistent blank/mostly blank startup screens and
2 additional fatal failures on nonblank screens. These classifications were
suspects, not proof of infinite loops or full gameplay incompatibility.

## Status-bar geometry

AnnunciatorComponent inherited ShellComponent's full display height. Games that
subtract the annunciator height consequently constructed zero-height Cards or
positioned their drawable area outside the display. A diagnostic run confirmed
다크슬레이어2 attempted to construct a 240×0 Card.

The annunciator constructor now reports a device status-strip height: 14 pixels
on a 120-wide display, 20 on 176/220, and 24 otherwise (bounded by display height).
These are emulated device-profile choices, consistent with existing phone
geometry conventions; the SDK does not prescribe these exact pixel sizes.
Ordinary ShellComponent geometry is unchanged. This patch does not implement
annunciator icon drawing or change its existing show behavior.

The regression checks both transparency constructor variants, geometry before
and after show, creation of the remaining game Card, and unchanged ordinary
shell dimensions. All 53 WIPI Java tests passed.

An identical isolated-save boot/input sequence cleared persistent blank-screen
classification in 10 of 11 affected titles:

- 게임빌 2006프로야구
- 다이어트 타이쿤
- 다크슬레이어2
- 멋지다! 김밥군
- 미니게임파티
- 보글보글
- 보글보글2
- 블레이드마스터2
- 열혈교사전설2
- 판타지포에버2

북천항해기2 was still blank after that isolated geometry fix; the subsequent Card field fix below clears its blank boot. The successes mean the captured boot failure has
cleared; they are not full-game compatibility claims. Some newly visible
screens are ordinary messages, including a server-unavailable dialog.

Contract reference: [WIPI AnnunciatorComponent SDK documentation](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/lwc/AnnunciatorComponent.html).

## Storage and access query

이노티아연대기2's mostly white screen actually reports insufficient storage:
about 2.1 MB required. The diagnostic database trace returned 985,425 available
bytes under the old 1 MiB logical per-application storage profile. The repository
can persist larger data; this advertised capacity caused rejection before
installation could proceed.

The logical KTF save-volume budget is now 32 MiB. This is an emulated storage
profile, not an ARM heap change or a measurement of host free disk. Available
space still subtracts the application's existing database usage, and total
capacity stays constant. Tests cover multi-MiB stored content, per-application
isolation, deletion/reclamation and the existing stream API. All 78 WIPI C tests
passed before the subsequent access-query mapping. LGT retains its separate 1 MiB profile; the broader LGT regression caught the initially shared capacity change.

The storage rerun cleared the space check and reached a separate unimplemented
kernel access-level query. The documented no-argument query returns a bitmask
of optional system/shared storage, network, serial and privileged API grants.
A first zero-grant probe removed the missing-API failure but exposed the guest's
authentication check. Inspection found the package's declared SLvl mask contains
the bits required by that check. The query now reads the installed __adf__ SLvl
value rather than hardcoding grants or confusing access permission with current
network availability. Missing/invalid metadata returns zero. This does not alter
host resource access. The metadata-backed rerun passes the previous authentication gate but then stops at a distinct missing MC_knlGetExecNames API. It is not yet a successful boot.

## Input-dependent findings and unresolved extensions

짜요짜요 타이쿤4 passed its first-run restart and connection gate when the
script explicitly chose the visible No option (Right, OK). It then displayed
a normal popup-settings notice. The original generic sequence had chosen Yes.
This is an automated input-path distinction, not a repaired network service.

미니게임패밀리 still requires the GMenubarForm/GFormBase/GMenuBar/GTextField
framework. Its SDK contracts are being examined; empty class stubs are not a fix.
액션히어로3D still requests MXUserMemInterf through kernel extension slot 36.

Native call-site inspection established that 드래곤로드 and 삼국쟁패2-전사편
both dereference network-table byte offset 0x88 (slot 34), beyond the populated
standard table. Both pass a carrier endpoint string. The resulting null/invalid
branch reaches PC 4. This identifies a common extension boundary, but does not
establish the exact vendor API contract. No guessed success or error return has
been installed for that slot.

Warnings now include the native exception class using metadata reads only,
without invoking guest Throwable formatting during unwinding. The KTF suite
passed 24 unit tests and its integration test after this diagnostic change.

All game archives, recordings, native disassembly and screenshots remain private
outside the repository. Tests use isolated cache saves. CPU execution, timers,
scheduling and refresh policy are unchanged. Further API work, rescue replay
validation and final normal-APK delivery remain in progress.

## Card field hiding

A separate native KTF test reproduces a base/subclass field collision: Card
geometry uses short names such as x and w, and an obfuscated guest subclass may
declare unrelated fields with identical names/descriptors. Resolving a base
method's fields from the runtime subtype reads the wrong declaration.

Explicit declaring-class JVM field accessors now let the Card adapter address
its own fields, including its display/canvas references. Native regression tests
check same-access-flags numeric fields, private reference-field hiding, getters,
constructor writes, move and resize without touching subclass data. The old
access path fails (getX returns 176 instead of 0); the candidate passes. The identical live boot sequence now reaches a visible title/game-description screen, with no detected error; the baseline remained entirely black.

The bytecode test adapter independently keys its fields by name/descriptor/flags
rather than declaring-class identity. That separate dependency issue prevents
it from serving as an oracle for identically flagged reference-field hiding;
the regression uses the actual KTF guest-memory representation instead. No
bytecode-adapter correctness claim is made by this native fix.

## Latest verification

The combined KTF/WIPI run passed 26 KTF unit tests, 1 integration test,
78 WIPI C tests and 53 WIPI Java tests (158 total). The real native Card
regression passes on the fix and fails on the old lookup path.

The newly observed GetExecNames call has a documented five-argument installed-
application enumeration contract, including nullable name/version/vendor filters
and a bounded NUL-separated output list. It remains unimplemented; a fake
success or empty list has not been substituted for an installation catalog.

The 26 warning-only cases remain unconfirmed: 24 record only thrown exception
addresses, while two include intentional exit stack traces. Exception classes
and actual unhandled/termination state must be distinguished in diagnostic
reruns before calling these hidden crashes. Private warning-review.json groups
the existing messages; it is not a compatibility verdict.

After separating storage profiles, the LGT and WIPI C rerun passed. The normal
pre-audit APK has been restored to the AVD; candidate fixes remain in source
and private diagnostic builds, not promoted as a completed release. Existing
player saves and imported archives were not reset by these isolated tests.

## Installed-application enumeration follow-up

MC_knlGetExecNames now enumerates the loaded package in the single-application
KTF guest environment. It does not enumerate unrelated games in the Android
launcher's library. Version and vendor filters come from Ver/Vdr metadata;
null filters are wildcards. A package without its JAR is not enumerated.

The standard SDK establishes the five-argument query, null-separated result
list, count result and M_E_SHORTBUF (-18). Native caller inspection establishes
the additional carrier convention: prgName is the installation AID, not the
human-readable Name, and its executable identifier is /AID/AID.jar. These values
are derived from metadata, without game names, fixed identities or PC branches.
The first display-title-only candidate reached authentication error 2001 and
was not counted as a successful boot. Live validation of the corrected carrier
identifier follows separately.

The actual native SVC test covers the fifth stack argument, wildcard and exact
filters, Korean vendor text, missing executable, no-match preservation, exact
buffer size, untouched trailing bytes, negative/short capacity, and invalid
input/output pointers. All 26 KTF unit tests and the integration test pass.

Diagnostic exception logs now identify the thrown class, API method/caller when
available, and a matched guest catch handler. No guest Throwable methods are
called during unwinding. A warning alone still does not establish a fatal or
background-thread failure.

## Warning and rescue verification results

All 26 warning-only reruns completed: 25 remained Running; Real Soccer 2009
explicitly called guest exit with code 0. The seven null/bounds/generic-exception
cases received a second diagnostic pass recording catch-handler targets. All
229 observed throws matched guest catch handlers and all seven remained Running.
This does not establish full gameplay correctness, but these observations are
not evidence of uncaught background-thread crashes. The repeated bounds throws
in Caus Blade originate in guest code, not a failing host Java method; do not
silence them or skip guest instructions to improve a counter.

Dark Slayer 2's original rescue was replayed using its exact originating audit
APK. The candidate then matched the safe-prefix frame and saved-data oracle,
continued for the harness interval (194 paints), and reported no logged error.
Classification: prefix-oracle-matched. This is a genuine rescue regression pass,
not merely the fresh-boot improvement reported above.

## Accompanying package data

The corrected executable identifier advances Inotia 2 to its missing-certificate
prompt. Its archive already includes the companion data. KTF loading previously
stripped only uppercase P/, while this archive uses lowercase p/; native resource
lookup also searched the JAR without consulting installed companion files.

The first candidate exposed private data through immutable resource lookup. That
advanced to error 3001 but would incorrectly resurrect a certificate deleted by
the guest, so that candidate was removed. The package supplier explicitly
requires an initial authentication failure, a restart and acceptance of the next
certificate prompt ([installation notes](https://dubigame.tistory.com/136)).

The replacement imports P/ and p/ private data once into the application's
mutable database repository before startup. An installation marker is persisted
with the save filesystem and is covered by save/reset/replay operations; it is
not an in-memory metadata registry. Existing databases are preserved, and later
guest deletion or modification does not cause the original seed to reappear on
restart. Ordinary JAR resources retain their previous lookup behavior.

Synthetic tests cover initial import, pre-existing progress, case-sensitive data
names, nested names, modification and deletion across subsequent initialization.
The KTF suite passes (27 unit tests and 1 integration test). No certificate or
identity is fabricated; live validation of the one-time installation follows.

## KTF stream file removal

The first one-time-install rerun still repeated authentication error 3001. Its
trace showed the guest calls the stream table's name-keyed slot 6 to delete the
rejected certificate. The existing adapter returned success without deleting.
The documented MC_fsRemove(name, accessMode) contract matches that call shape
and the stream table's surrounding MC_fs operations; this is not an inferred
request to delete record 1 through a string pointer.

The name form now removes the private file/database, returns M_E_NOENT when
absent, rejects unsupported shared/system access and malformed/long names, and
refuses removal while any stream for that name is open. The existing open-handle
record-deletion form remains supported. Stream membership and links reside in
guest memory with a System adapter root, consistent with existing record/timer
roots. Closing any position in the list unlinks and invalidates the handle.

Regression coverage checks head/middle/last closes, multiple opens of one file,
contents preserved after refused removal, successful deletion after close,
missing files, access restrictions, bad names, and the legacy record form.
KTF: 27 unit + 1 integration; LGT: 40 unit + 1 integration; WIPI C: 79 tests pass.
Live validation of removal and restart follows separately.

## Inotia startup result

The removal fix makes the rejected certificate absent in the next startup's
saved state (verified against the preceding candidate, which retained its
23-byte record). The package's documented restart then reaches the certificate
prompt. An explicit timed input sequence chooses Yes. The game reports a failed
server connection and grants its own temporary authentication, then reaches its
title screen. No successful server response, fabricated certificate, phone-number
guess or unconditional authentication bypass was introduced. Full gameplay and
future temporary-certificate renewal remain unverified.

The final file-removal diagnostic APK also passes the original Dark Slayer 2
safe-prefix oracle again. Ordinary user saves remain separate from these tests.

The warning audit's original RSOFT token was ignored by NativeBridge; it expected
R. The runner now normalizes LSOFT/RSOFT and rejects unknown keys, with tests.
Existing evidence is not relabeled as right-soft-key coverage. Diagnostic rescue
continuations gained bounded WAIT steps to make first-run navigation repeatable.
