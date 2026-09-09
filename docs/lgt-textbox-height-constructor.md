# TextBoxComponent constructor compatibility

Bungeoppang Tycoon 3 stopped at startup while resolving
`org/kwis/msp/lwc/TextBoxComponent.<init>(Ljava/lang/String;II)V`.
The shared WIPI Java class only exposed the two-argument constructor.

The archived WIPI API reference identifies the overload as
`TextBoxComponent(String data, int constraints, int h)`:
https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/lwc/TextBoxComponent.html

The overload now initializes the parent, stores the supplied constraint and
text, and preserves explicit height in guest object state. `getHeight()`
returns that height. The two-argument overload retains its previous default
height. Constraints outside the documented range 0–5 are rejected.
Shared TextComponent setString/getString now preserve actual per-instance
strings instead of discarding data and returning the placeholder "temp";
null represents empty text.

Tests exercise both constructor descriptors, Korean text round trips, text
replacement, null input, instance independence, explicit height, and invalid
constraints. No game files or saves are modified.

This is constructor/text-state support, not a complete LWC editor. Existing
input-method, painting, layout, and maximum-length stubs remain separate work.

Additional startup dependencies are now linkable: FormComponent constructors
preserve orientation and the DialogComponent constructor preserves its work
component, title and validated type. This does not implement modal editing;
`doModal()` explicitly raises UnsupportedOperationException instead of inventing
an OK/cancel result. Full LWC layout/interaction remains unsupported.

Dialog constants and Form constructor orientation were checked against the
archived KTF WIPI 1.1 API reference:
https://nikita36078.github.io/J2ME_Docs/docs/KTF_WIPI_API/org/kwis/msp/lwc/DialogComponent.html
https://nikita36078.github.io/J2ME_Docs/docs/KTF_WIPI_API/org/kwis/msp/lwc/FormComponent.html

Live validation: startup notice and animated title reached in the Android AVD.
See [the LGT linker report](lgt-wide-field-linking-and-calendar.md) for subsequent
faults uncovered and fixed after class resolution succeeded. Modal editing was
not exercised during startup.
