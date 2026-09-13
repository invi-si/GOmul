# KTF audit: optional WIPI Java contracts

The imported KTF boot audit exposed named class-resolution failures before guest
initialization completed. These are Java class contracts, not inferred numeric
native slots. Registration is shared across games and carriers; no title or
archive hash changes runtime behavior.

| Class | Implemented contract | Scope |
| --- | --- | --- |
| `org.kwis.msp.handset.LED` | static `getCount()I`, `get()I`, `set(I)V` | Reports zero indicator LEDs; unsupported LED bits have no effect. |
| `org.kwis.msp.media.MediaUnsupportedException` | `RuntimeException` subclass; empty and String constructors | Preserves the normal Throwable message. |
| `java.io.UnavailableException` | WIPI `RuntimeException` subclass; empty and String constructors | Resolved by name after the first exception definition enabled further loading. |
| `org.kwis.msp.lwc.ActionListener` | abstract `action(Component,Object)V` | Declares the interface; does not fabricate callback behavior. |
| `org.kwis.msp.lcdui.InputMethodListener` | abstract `notifyTextChanged(char[],int,int)V` | Declares the interface; does not implement text composition. |

Contracts were checked against the original WIPI API 1.1.1 SDK Javadocs, available
in this documentation mirror:

- [LED](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/handset/LED.html)
- [MediaUnsupportedException](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/media/MediaUnsupportedException.html)
- [UnavailableException](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/java/io/UnavailableException.html)
- [ActionListener](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/lwc/ActionListener.html)
- [InputMethodListener](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/lcdui/InputMethodListener.html)

The exception superclass follows this version explicitly; later API documentation
must not silently redefine an earlier contract. No external implementation code
was copied. The regression test checks class resolution, interface flags and exact
method descriptors, exception inheritance/messages, and zero-LED behavior after
multiple masks. Device boot and Rescue comparisons are private audit artifacts;
class resolution alone is not a claim of game compatibility.

Unknown KTF numeric database/kernel/graphics slots remain unresolved. A label
printed by an unimplemented stub does not prove the call site's ABI matches that
label. Confirm arguments and behavior before assigning another implementation.
