# KTF forms, modal dialogs and record-list capacity

This follow-up repairs three more recorded failure paths using shared emulator
implementations. The tested packages are 미니고치, 마스터오브소드2-작은화면 and
컴투스포춘골프3D. A bounded boot/input pass is not a full gameplay certification.

## Components and modal dialogs

FormComponent now lays out children using their preferred sizes and configured
orientation/gap. Focus traversal skips non-input children, delivers keys to the
focused child, and scrolls it into view. Container attachment/removal updates focus;
painting and on-screen coordinates account for scrolling offsets. These values
live in guest fields. Label and button preferred sizes include their contents.

KTF GFormComponent preserves explicitly supplied child rectangles through layout,
computes content extents, and uses the shared focus/input behavior. This resolves
미니고치's missing form superclass path. It is not a complete implementation of
KTF's skin, list, text-entry or menu-bar framework.

DialogComponent presents its work component and waits for real user input or its
configured timeout. OK, Cancel and timeout return distinct documented results;
the implementation does not manufacture an OK response. Modal waits dispatch
the existing event queue, and timeout callbacks check guest-backed activation and
generation fields so an old timeout cannot dismiss a newly shown dialog.

EventQueue returns deferred future timers to the queue before entering a guest
timer callback or callSerially batch. Otherwise a nested modal event loop could
not see timers held in the outer, suspended stack frame. Existing timer due-time
checks, cancellation guards and refresh limits remain intact. Dialog reuse also
updates the associated Card's geometry.

The small-screen Master of Sword 2 package now passes its previous modal-dialog
failure. Full text-entry behavior and later gameplay still need validation.

## Fixed-record database capacity

Golf intentionally exits on its first launch after initializing its database;
its own message asks the user to run it again. That exit is preserved.

The second launch exposed a different bug in MC_dbListRecords. Its capacity is
an element count, but our implementation required four times that value. The
observed native caller reserves a 48-byte array, supplies capacity 12, and reads
the returned IDs to select 128-byte records. There are 12 valid records. Rejecting
this request left stale stack values in the ID array, caused the subsequent
record selection to fail, and eventually produced an empty resource name and
a null-pointer dereference. The stored records themselves were valid.

The shared fixed-record API now treats capacity as the number of integer IDs,
checks the entire capacity before writing, excludes private size metadata and
returns the documented bad-descriptor, invalid-argument and short-buffer errors.
This does not change the separate KTF stream-filesystem interface or patch saves.
Golf passes the second-launch boot/input sequence after the correction.

The audit harness can explicitly probe a second launch with the same disposable
save after an intentional stop. It preserves both launches' native logs. Normal
application startup does not gain an automatic restart or bypass.

## Evidence and regression coverage

- [WIPI FormComponent SDK](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/lwc/FormComponent.html)
- [WIPI DialogComponent SDK](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/lwc/DialogComponent.html)
- [KTF GFormComponent SDK](https://nikita36078.github.io/J2ME_Docs/docs/KTF_WIPI_API/com/ktf/kfc/GFormComponent.html)
- [Database specification transcription](https://github.com/mirusu400/wipi-wiki/blob/d65e5a74174851d4cc733bdb2a4830693a7c1a7b/src/content/docs/c-api/database.md)

The database text calls `len` the buffer size without explicitly naming its unit;
the native caller's array bounds and indexing establish the element-count ABI.
No reference emulator implementation was copied.

Tests cover positioned layout, preferred extents, traversal, focus detachment,
scroll coordinates, real modal key press/release and timeout results, stale
timeout rejection, future-timer visibility during guest callbacks, and 12-record
listing with guard bytes, short buffers, invalid arguments and reopening.

Three cases still require separate work: 미니게임패밀리 needs the KTF
GMenubarForm/GMenuBar command and text-input framework; 드래곤로드 calls an
undocumented network slot 34; 액션히어로3D requests the unconfirmed
MXUserMemInterf vendor function table. Missing classes or interfaces are not
replaced with empty classes, invented return values or success stubs.

All game-derived logs, screenshots, databases and Rescue captures remain private
test artifacts. User-excluded titles are not retested. Normal saves and game files
are preserved; existing checkpoint build guards remain enabled.

## Final validation

The final diagnostic APK ran 29 selected boot/input sequences: 16 smoke-only,
nine logged-error-review, one static-screen-review and three native failures.
The failures are the three unresolved cases listed above. The review category
includes eight caught-exception cases and one intentional stop whose diagnostic
backtrace contains an error-type name; that backtrace is not a new exception.
All 19 previously repaired fatal cases avoid a new fatal or uncaught-thread error.
One still stops and one still remains static in the bounded sequence.

The newly improved cases remain running: Master of Sword 2 small-screen records
65 paints, MiniGoChi 60, and Golf 348 after its intentional first-launch exit and
second launch. These are cumulative paint counts during short, different startup
sequences, not gameplay FPS or performance comparisons.

All 24 selected Rescue fixtures match their saved safe-prefix reference and
continue for three seconds without a fatal or uncaught-thread error. Seventeen
have no logged error and seven retain caught-exception warnings. Reference
fixtures were not recaptured or weakened to accommodate the changes.

Relevant Rust suites pass 375 tests; the audit tools pass eight tests. Web/WASM
compilation, targeted Rust formatting and diff whitespace checks pass. Clippy
completes with warnings, including SDK callback argument counts and existing
style warnings; this is not a warnings-as-errors result.

The delivered ordinary APK has compatibility-audit instrumentation disabled and
is installed in the Android environment with all 220 game archives retained.
Older Quick Saves may need recapture because runtime build guards remain enabled.
Thor and public releases were not changed.
