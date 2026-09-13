# Native text baseline alignment

A subsequent 짜요짜요타이쿤3 manual Rescue showed dialogue and speaker labels
below their intended positions. The recorded native drawing calls used baselines
68 and 88 for the dialogue and 111 for the name label. Graphics translations
were zero; text drawing used an unrestricted clip. This was not a text-wrapping
or display-dimension issue.

The WIPI-C graphics specification, `MC_grpDrawString` parameter description,
defines `y` as the text baseline. GOmul passed it unchanged to a shared canvas
adapter that adds ten pixels before placing glyphs. The native adapter now
subtracts that default ascent, matching the existing GetFontAscent result.
Java and the separate LGT graphics adapter are unchanged. Box positions, game
assets, saves, timing and font sizing are unchanged.

Replaying the same rescue now places both dialogue lines and the speaker name
inside their existing boxes. The replay remained Running. A pixel regression
checks uppercase glyphs above the supplied baseline at several vertical
positions, including clipping above the framebuffer. All 75 WIPI-C tests pass.

This is a correction for the currently supported default font, not an
implementation of missing native font selection or an assertion of complete
gameplay compatibility. Replay artifacts and initial saves remain private.
