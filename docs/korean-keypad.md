# Korean keypad

The Android game header places Quick Save and Quick Load immediately before Settings. Holding Load retains the existing recovery action. The button to the right of Call switches the keypad between Korean and alphabetic/game labels.

Korean mode uses a shared 천지인 composer in the emulated WIPI LWC editor and native KTF/LGT input-method APIs. Open a supported text field, select 한글, and use the labeled number keys. In the Java editor, repeated consonant presses within 700 ms cycle the group. Native input follows its explicit-flush contract; use # between separate consonants on the same key. `#` commits the current syllable sequence, `*` inserts a space, and CLR/R deletes. OK finishes the editor. For example, `8 8 1 2 5 4 3 5 5` enters `한글` when repeated keys are pressed within that interval.

Numeric/phone/ASCII-restricted fields keep their constraints. Ordinary game keys retain their numeric key codes. Native screens using MC_imHandleInput receive composed Korean through their normal buffers; see [native input](native-korean-input.md). Games that implement a wholly private input engine still need their own supported interface.

Composition tokens, cursor and input mode live in guest fields. Mode selections are recorded in the checkpoint event stream; replay reconstructs them along with guest execution. Android reads a presentation-only mode mirror after checkpoint loading to update its labels.

Regression coverage includes all 21 modern vowels, final consonants and clusters, resyllabification, consonant cycling, component deletion, field length limits, programmatic replacement, numeric constraints and checkpoint event ordering.
