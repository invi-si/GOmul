# Korean byte-prefix width measurements

TalesWeaver: Ispin's opening notice and dialogue contained boxes at line breaks.
The archive text was valid CP949. A temporary text probe showed the guest
measuring prefixes one byte at a time, then passing misaligned byte sequences
for later lines (for example, a sequence decoded as `酉嘯�` instead of Korean).

The shared width path decoded an incomplete final lead byte as U+FFFD, whose
narrow advance underestimated the full Korean character. A byte-prefix wrapping
loop could consequently accept the lead byte, then overflow on the trail byte.

For bounded width measurements only, an incomplete final EUC-KR/CP949 sequence
now reserves a full-width cell using U+3000. A streaming decode without EOF
distinguishes an incomplete sequence from malformed input. Complete text,
drawing, unbounded strings, and invalid standalone bytes keep their existing
behavior. No byte beyond the requested range is read. No game-specific branch,
archive change, save change, timing change, or font replacement is involved.

Regression coverage exercises mixed ASCII/Korean prefixes, equality of partial
and complete Korean cell widths, unchanged drawing decode, invalid lengths,
and malformed standalone bytes. WIPI-C and LGT unit tests pass; workspace
Clippy passes with existing warnings.

Live validation: the normal candidate APK was installed in the Mac Android
emulator. The opening notice that previously contained boxes on most lines now
renders complete Korean text across all line breaks. The later dialogue scene
has not yet been revisited. The original game archive and saved data were kept.
The temporary diagnostic log was moved outside the save directory after it
triggered the checkpoint size limit; the candidate has no text probe.
