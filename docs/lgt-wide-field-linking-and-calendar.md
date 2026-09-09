# LGT wide-field linking and Calendar dispatch

Bungeoppang Tycoon 3 exposed two additional shared runtime gaps after its
missing LWC constructor/classes became linkable. The original archive was used;
no game code or save was patched.

## Wide field import outputs

The public member linker crashed at address zero from Java SVC 23, invoked at
guest PC 0x13dc. The field-import table contains name/descriptor pairs, with a
zero/zero pair immediately after each two-word `J` field. For example, the
`d` class imports contain these pairs at indices 5/6, 12/13 and 26/27.

These are two-word field entries, not fields named by a null pointer. Both output
indices must be initialized: the ordinary entry receives the resolved low-word
index, and its empty successor receives low-word index + 1. Simply skipping the
empty row is incorrect: generated code uses the second index to store the high
half separately. An intermediate diagnostic build demonstrated corruption of
`Card.canvas` at word zero when the second output remained zero.

The shared instance/static linker now handles this shape. It only accepts an
empty pair after a J/D descriptor; arbitrary null member names still fault.
Regression tests cover J/D recognition, invalid empty entries, consecutive wide
fields, the actual output pair and a following field, plus untouched output
guards. Existing member-link tests remain in place.

## Calendar compiler dispatch

After linking, startup called the Calendar receiver's dispatch word at +0x50
(method index 19), with field arguments 1, 2, 5 and 11: year, month, day and hour.
The receiver is the generated `ek:Ljava/util/Calendar;` field. GOmul had no ABI
entry at that position and execution jumped into unrelated data.

The ABI data now maps `java/util/Calendar.get(I)I` to index 19. A regression test
constructs GregorianCalendar and calls the raw ARM dispatch target for those
four fields, comparing results against ordinary JVM calls. Calendar behavior
itself was not changed.

## Validation

The final native APK passed startup in the Android AVD, displayed the opening
notice and animated title screen, and responded to OK. Further gameplay is left
for manual testing; this does not establish full-game compatibility.

`cargo test -p wie-lgt -p wie-wipi-java`: 62 tests passed (31 LGT unit,
1 LGT integration, 30 WIPI Java unit). `cargo fmt`, `cargo clippy --workspace`
and the Android APK build passed; clippy reports existing warnings.

CPU timing, event scheduling, networking,
refresh limits and game data are unchanged. Temporary panic/field probes are
removed from the final build. Android's existing panic catch now includes the
panic message instead of discarding it behind a generic stopped message.
