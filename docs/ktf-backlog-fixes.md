# KTF compatibility backlog, 2026-09-11

The earlier boot audit identified unresolved failures. It was not evidence that
the imported collection was compatible. This pass fixes shared causes and keeps
the remaining failures explicit. A successful short boot/input sequence does not
establish gameplay compatibility.

## Implemented contracts

- **Native Java bridge:** host-backed nonabstract methods have both register and
  packed entry points without changing Java access flags. Packed returns and the
  three Java jump adapters preserve both ARM return words, including 64-bit time.
- **Exceptions:** the object-throw entry is distinct from class-name throwing.
  A caught exception is written to the guest handler's catch-variable slot before
  unwinding. Object identity and the saved register context are preserved. This
  does not claim complete support for every nested native exception-handler chain.
- **Clipping:** the KTF null clip argument resets the clip rather than reading a
  rectangle from address zero. Explicit empty rectangles remain empty.
- **Presentation ownership:** actual C framebuffer flushes select C presentation;
  drawing into the associated Java screen graphics selects Java presentation.
  Offscreen drawing and graphics configuration do not change ownership. The
  association is a guest Java reference, not a game-name heuristic.
- **Allocation:** impossible or exhausted C allocations return NULL. Failed
  calloc does not continue with an invalid buffer.
- **KTF stream filesystem:** corrected seek origins, signed offsets, one shared
  read/write position, file-existence convention, resource stat, empty-file
  creation, directory creation/listing and total-capacity query. The KTF table is
  a filesystem interface, not the fixed-record database interface.
- **Fixed-record database:** independently mapped open/close/delete,
  insert/select/update/delete-record, record listing, mode, count and size.
  Record size persists; internal metadata is excluded from guest records. Open
  handles and deletion-in-use checks use a guest-memory linked list. Remaining
  database operations are still explicit unsupported calls.
- **Java database opening:** `create=false` fails when the database is absent;
  empty creation is materialized before returning. This fixes a separate path
  where games treated missing saves as existing empty databases. Other old Java
  database limitations, including full persisted record-size semantics, remain.
- **BMP encoding:** `MC_grpEncodeImage` returns an owned, zero-padded BMP of the
  requested framebuffer rectangle. RGB565 and ARGB sources and padded source
  strides are supported. Invalid geometry/unsupported pixel formats/allocation
  exhaustion return NULL; invalid guest memory remains an emulation fault.
- **Optional APIs:** the documented GrabKeyListener interface is registered.
  KTF OEMDevice reports unavailable address-book/theme capabilities with NULL.
- **Offline networking:** socket readiness registration, connection and reads
  return documented failures when no connection exists. No fake successful
  socket or callback to guest address zero is created.
- **Unsupported package:** an ODCF-wrapped payload named `.jar` is rejected before
  JVM initialization with an explanatory error. This does not decode that format
  or make the affected package compatible.

No title/hash-specific execution branches were added. Guest timing, timer
lifecycle, speed controls, scheduling and refresh caps were not changed. Thor
and public releases were not updated in this pass.

## Contract evidence

Native call sites were inspected for argument order and the distinct exception
entries. Public specification text was used for API behavior; implementation
code from other emulators was not copied.

- [WIPI C graphics specification transcription](https://github.com/mirusu400/wipi-wiki/blob/d65e5a74174851d4cc733bdb2a4830693a7c1a7b/src/content/docs/c-api/graphics.md)
- [Filesystem specification transcription](https://github.com/mirusu400/wipi-wiki/blob/d65e5a74174851d4cc733bdb2a4830693a7c1a7b/src/content/docs/c-api/filesystem.md)
- [Database specification transcription](https://github.com/mirusu400/wipi-wiki/blob/d65e5a74174851d4cc733bdb2a4830693a7c1a7b/src/content/docs/c-api/database.md)
- [Network specification transcription](https://github.com/mirusu400/wipi-wiki/blob/d65e5a74174851d4cc733bdb2a4830693a7c1a7b/src/content/docs/c-api/network.md)
- [WIPI Java DataBase SDK documentation](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/db/DataBase.html)
- [GrabKeyListener SDK documentation](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/lwc/GrabKeyListener.html)
- [KTF OEMDevice SDK documentation](https://nikita36078.github.io/J2ME_Docs/docs/KTF_WIPI_API/wec/OEMDevice.html)

The graphics transcription prints a narrow return type inconsistent with its
memory-ID description; the KTF call site consumes the full R0 memory ID. The
implemented ABI follows the observed caller and memory-ID convention.

## Validation and limits

The baseline covered 92 previously flagged imports, excluding the three titles
the user had already excluded. There were 29 fatal cases. After the shared fixes,
targeted followups and database rechecks, 19 no longer produced a fatal error in
the same bounded boot/input sequence; 10 remained unresolved. The final cohort
contained 23 smoke-only results, 48 logged-exception review cases, seven stopped
cases and four static-screen cases, in addition to those 10 fatal cases. These
categories are evidence classifications, not a list of playable games.

All ten control titles ran without fatal or uncaught background-thread failures
after the database correction. The remaining confirmed background-thread failures
were `ContainerComponent.repaint` and the absent KTF `ChoiceText` component.
The first reported title, 짜요짜요타이쿤3, no longer reproduced its null-clip crash,
but remained static-screen-review in the short automated sequence.

The latest relevant Rust suites passed 242 tests, and the audit harness passed
eight tests. Clippy completed with existing warnings; targeted formatting checks
and the web/WASM compile check passed. All 19 improved cases matched their saved
Rescue safe-prefix oracle and continued three seconds without a native fatal
error: 13 had no logged exception and six retained caught-exception warnings.
No uncaught-thread errors appeared in those Rescue continuations. This verifies
the recorded failure boundary, not later gameplay.

Regression coverage includes both Java entry ABIs, wide returns, thrown/caught
object identity, handler context, clipping reset, allocation failure, filesystem
cursor/error behavior, directory buffer boundaries, record persistence/deletion,
missing Java database creation, and BMP decode round trips. The web adapter
compiles but explicitly declines directory operations that its flat storage
format cannot represent; native Android directory support is not a claim of web
support.

The private device ledger retains every before/after result, APK identity, native
log and rescue outcome. Tests use disposable saves; the user's archives, normal
saves, Quick Saves and exported rescues are preserved. Diagnostic logging is
removed from the ordinary build. Cross-build Rescue verification is diagnostic
only: normal Quick Load build guards remain enabled, and older checkpoints may
require recapture after these runtime changes.

Remaining KTF LWC/KFC UI components need real layout, painting, focus and input
semantics. Registering empty classes would only move the crash or produce a
nonfunctional screen. Unknown vendor extension slots, remaining guest memory
faults, and static/input-gated screens are kept as unresolved work rather than
being reported as successful compatibility fixes.

After this audit, the user classified 강호동맞고2 and 정통맞고2007 as incompatible
and stopped investigation of those packages. Historical measurements above are
unchanged. The active backlog therefore has eight fatal cases and two confirmed
background-thread failures, plus the static/stopped review cases. Exclude these
two packages from further testing unless the user requests otherwise; retain
their archives and saves.

The subsequent [KTF UI and image ABI follow-up](ktf-ui-and-image-abi.md) supersedes
the active-backlog count above. It resolves the recorded sequence for four more
titles and removes the golf cleanup fault, while retaining explicit UI/vendor
failures and a separately reproduced no-input stop. See that report for the
per-title results and the limits of the automated checks.
