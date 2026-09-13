# LGT messaging and stream ABI compatibility

A Nurse Tycoon 2 rescue reported `NoClassDefFoundError` for
`org/kwis/msf/io/Message`. The runtime now provides this WIPI buffer/address
container with guest-backed fields, slice bounds, and message metadata.
API reference: https://nikita36078.github.io/J2ME_Docs/docs/KTF_WIPI_API/org/kwis/msf/io/Message.html

Startup also uses `Jlet.getCurrentJlet()`, now an alias of the existing
`currentJlet` accessor. This does not create another lifecycle instance.

Following these calls exposed missing LGT native virtual-method mappings.
The serializer calls DataOutputStream slots 15/19 (boolean/int), then
ByteArrayOutputStream slot 16 (byte array), and the matching reader uses
DataInputStream slot 22 (boolean). These mappings are carrier ABI data;
no game-name checks or game archive changes are involved. Object slot 5
maps to the existing notify implementation.

Regression validation exercises the raw native slots, checks the exact
serialized bytes for true/false and positive/negative integers, and reads
them back through native DataInputStream slots. Message tests cover shared
buffer identity, slice bounds, metadata, and address reset. Jlet tests
verify both getters return the same guest instance.

A further startup failure exposed a Class prefix collision: `nameBytes`
occupied word zero, whose bit 0x2000 selects an alternate native static-field
layout. Pointer bits could therefore redirect static initialization through
string data. The ABI data now reserves word zero for native flags and places
host references in words 1/3, preserving native name/state words 2/4 and the
static-field base at +0x14. Regression checks cover flags, reference storage,
and the existing static-field and class-initialization behavior.

Validation: 34 LGT unit tests, one LGT integration test, and 32 WIPI Java tests
pass. Workspace clippy completes with existing warnings. The regular Android
build boots to the opening usage notice without the rescued fatal error.
Progression beyond that notice has not been confirmed; automated OK/L taps
did not advance it. No full gameplay compatibility claim yet.

Temporary native-memory diagnostics were removed. Game archives, timing,
and user save files were not patched or reset.
