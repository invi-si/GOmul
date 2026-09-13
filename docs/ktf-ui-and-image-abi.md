# KTF native image ABI and lightweight UI follow-up

Subsequent work implements forms/modal dialogs and resolves Golf's second-launch
record-list failure. See [the later follow-up](ktf-forms-dialogs-and-record-list.md).
The results below describe this earlier stage.

This follow-up investigates the eight fatal cases and two background-thread failures
left active after the previous KTF audit. 강호동맞고2 and 정통맞고2007 remain
user-excluded. These are emulator changes, with no game archive or save patches.

## Confirmed native image fault

폴라폴리2007 dereferenced an embedded framebuffer width as a pointer. Its KTF
native image prefix expects framebuffer memory IDs, whereas the shared image
implementation supplied inline framebuffer descriptors. KTF now explicitly selects
the indirect image ABI; other adapters retain their existing representation.
The descriptors and ownership information remain in guest memory. Reads follow the
live guest descriptor, so guest changes are not hidden by a host cache.

A second fault became visible after correcting the pointer: native rendering used
the source pixel depth to calculate destination LCD strides. Supplying ARGB32
images with an RGB565 LCD caused writes beyond the LCD allocation and corrupted
the guest heap. KTF native images now use RGB565 pixels, matching that LCD, with
an independent eight-bit alpha mask where needed. Java/MIDP image storage remains
unchanged. Tests cover alpha rendering, live descriptor changes, borrowed encoded
source ownership, and cleanup of owned allocations.

Offscreen cleanup now releases its pixel allocation as well as the descriptor.
A null handle is harmless and the LCD framebuffer is preserved. This removes the
original cleanup fault in 컴투스포춘골프3D, but its later stop is a separate issue.

## Lightweight components

Component geometry, parent/card lookup, invalidation, repaint forwarding, colors
and background painting now have guest-backed implementations. Containers manage
children, reject invalid attachment/index operations, and paint with restored
clip/translation state. Shells attach their work component and bridge painting
and input through a WIPI Card. Component's handled-key result is inverted when
returned as Card's propagate-key result.

LabelComponent renders nullable text/images. KTF ChoiceText stores and navigates
its guest string array. ButtonComponent supports paired Select press/release and
ActionListener callbacks, including argument identity, listener replacement and
focus cancellation. These are partial SDK implementations, not claims of complete
LWC/KFC layout, focus traversal, form, ticker, command-bar or dialog support.

DialogComponent now attaches its work component instead of merely storing it.
Modal presentation remains explicitly unsupported: inventing an OK response or
blocking the UI event thread would be incorrect. The missing GMenubarForm family
also remains unresolved.

## Error reporting and audit

Java exception formatting failures preserve the original exception instead of
panicking while attempting to print its stack. KTF class-loader initialization
errors propagate to the emulator's fatal-error/rescue path. Native CPU faults log
registers and a bounded stack before unwinding, without adding per-step work.

The diagnostic boot harness records a framebuffer image before input and skips
empty key entries. A missing/blank key sequence therefore sends no input. Screenshots
and game-derived captures remain private test artifacts, outside this repository.
Boot/input success is only a smoke result, never a claim of playable compatibility.

## Evidence and unresolved contracts

The public WIPI Java SDK documentation establishes component inheritance and
button callback semantics. WIPI C graphics documentation establishes framebuffer
cleanup responsibilities. The KTF image representation and stride fault are
confirmed by native guest call sites and reproduced memory faults.

- WIPI SDK: https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/lwc/ButtonComponent.html
- KTF SDK: https://nikita36078.github.io/J2ME_Docs/docs/KTF_WIPI_API/com/ktf/kfc/ChoiceText.html
- WIPI C transcription: https://github.com/mirusu400/wipi-wiki/blob/d65e5a74174851d4cc733bdb2a4830693a7c1a7b/src/content/docs/c-api/graphics.md

드래곤로드 calls network slot 34, beyond the documented table ending at slot 29.
액션히어로3D requests MXUserMemInterf through a reserved kernel entry and then
calls the returned function table. The contracts of those extensions are not
established. Neither has been replaced by a guessed null/success implementation.

## Latest automated results

| Title | Result after this change |
| --- | --- |
| 폴라폴리2007 | Original pointer/heap faults gone; running, 232 paints in the short sequence. |
| 뮤_흑기사편 | Missing shell geometry failure gone; running, 61 paints. |
| 해적왕2007 | Original layout/validation failure gone; running, 66 paints; caught-exception warnings remain. |
| 질풍노도17대1 | Original repaint background-thread failure gone; running, 99 paints; caught warnings remain. |
| 마스터오브소드2-작은화면 | Component construction/attachment progresses; modal dialog presentation remains unsupported. |
| 드래곤로드 | Undocumented network slot 34 remains unresolved. |
| 컴투스포춘골프3D | Original framebuffer cleanup fault gone; one black frame, then stops even with no input. |
| 액션히어로3D | MXUserMemInterf vendor extension remains unsupported. |
| 미니게임패밀리 | Missing GMenubarForm family; now reported as a guest initialization failure instead of a host panic. |
| 미니고치 | ChoiceText and ButtonComponent load; background thread now reaches missing GFormComponent/FormComponent support. |

The active failure count is four native fatal cases, one confirmed background-thread
failure, and one stopped-screen case. Four other titles have cleared their recorded
failure sequence. None of these results constitutes a full gameplay pass.

The 19 previously repaired fatal cases were rerun: no native fatal or uncaught
background-thread failure. Eleven were smoke-only, six retained caught warnings,
one stopped and one remained static. Relevant Rust suites passed 370 tests, and
the Python audit harness passed eight. Targeted formatting, diff whitespace checks,
and web/WASM compilation passed. Clippy completed with warnings in existing and
previously modified code; it was not run with warnings-as-errors.

Private artifacts retain all seven diagnostic iterations, not just successful
results. Tests use disposable saves. The ordinary non-audit APK is distinct from
the diagnostic build. Saved-state build guards remain enabled; checkpoints from
older runtime builds may require recapture. No game archives, user saves or
protected startup checkpoints were reset, patched or deleted.

All 23 selected Rescue fixtures matched their saved safe-prefix reference and continued for three seconds without a native fatal or uncaught-thread error. Sixteen had no logged exception; seven retained caught-exception warnings. This includes the 19 earlier fixes and four newly improved native-fatal cases. It does not certify later gameplay or resolve the golf shutdown.
