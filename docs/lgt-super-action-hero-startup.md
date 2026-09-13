# LGT Super Action Hero startup rescue

The submitted rescue exactly reproduces a missing
`DialogComponent.setButtonString(int, String)` import on the originating APK.
Fixing linkage uncovered further background-thread failures; a Running status
with a black frame did not establish successful startup.

Changes are shared WIPI Java functionality and LGT ABI support, not game edits:

- Dialog button labels are stored per instance. WIPI button IDs 20/21 are
  validated; TYPE_NONE leaves labels unchanged. This does not implement modal
  presentation or manufacture an OK/cancel response.
- Native imports 0xfd and 0x5b write/read 64-bit array elements. Guest call sites
  and a diagnostic trace identify a long array initialized to -1 at indices
  0/1/2, followed by a resource decoder loading wide table entries. Both helpers
  use the existing eight-byte wide-array prefix and low/high register order.
- DataInputStream.readUTF maps to native vtable index 32 and Random.setSeed to
  index 10. Existing Java implementations provide behavior.
- Assignability import 0x12 accepts the class-name pointer used by native catch
  clauses as well as the Class object used by array-type helpers. The latter
  is identified by its actual guest Class dispatch table. Previous array
  assignability tests remain in place; no application names select behavior.

The dialog method contract and constants come from the WIPI 2.0.1 standard,
DialogComponent pages 238/241 ([archived standard](https://manualzz.com/doc/13408437/korea-wireless-internet-standardization-forum-wireless-co...)).
Implementation is independent. Carrier call conventions were established from
local guest call sites; no reference-emulator code was copied.

Validation: 38 LGT unit tests, one LGT integration test, and 34 WIPI Java tests
pass. Native tests cover full wide-array word values, preserved neighbors and
prefix, UTF strings including Korean/empty/EOF, seeded random sequences versus
the Java API, and both assignability target representations. Dialog tests cover
button IDs, invalid IDs, TYPE_NONE and separate instances.

Fresh startup reaches the usage notice. Rescue continuation with OK input
advances to the Com2uS logo (four paints). This is startup progression, not
full-game compatibility. Two PNG PLTE checksum failures remain in diagnostic
logs and need separate investigation if images are visibly missing. Modal
LWC dialogs remain unsupported.

The original rescue predates any completed scene: the candidate safe-prefix
frame is not identical to its exported frame. Do not claim before/after image
equality for this rescue. Repeated candidate safe-prefix validation uses its
own captured frame/save oracle. All test saves are isolated from user data.
Temporary diagnostics and game/rescue contents are excluded from source.

## Loading-screen palette correction

A later live test stalled after a title-screen keypress. The recorded runtime
log showed several PLTE CRC mismatches followed by `NullPointerException:
image is null` in `i.keyNotify`; the event handler then stopped producing frames.
A no-input title run did not reproduce it, so validating only the title was
insufficient.

Shared image decoding now normalizes only PLTE checksums in a private buffer.
Chunk bounds and palette-length limits are checked before normalization; the
normal PNG decoder continues to validate structure and every other chunk CRC.
The original byte array/game archive is untouched. Valid palettes require no
buffer copy. This supports games that modify palette bytes without updating
that chunk's checksum. It does not generally disable CRC checking.

A generated one-pixel indexed PNG test verifies red-to-green palette editing,
unchanged caller bytes, valid-image zero-copy normalization, rejection of bad
IHDR/IDAT checksums, and short/truncated inputs. All 53 backend tests pass;
workspace clippy completes. crc32fast was already in Cargo.lock through the
PNG dependency and is now an explicit backend dependency for this checksum.

A six-second fresh start followed by OK inputs and ten seconds of continuation
reaches the tutorial with characters and Korean dialogue (89 paints), without
image decoding failures or event exceptions. This supersedes the startup-only
result and the known PLTE failures above; full-game completion and modal dialogs
remain untested/unsupported respectively. Tests use isolated initial saves.
