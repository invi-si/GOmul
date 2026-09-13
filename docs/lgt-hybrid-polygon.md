# Hybrid 2 polygon import

Rescue `1788998009451` stops during gameplay at LGT WIPI C import `0xf0`.
The caller at `0xdbf0` passes the framebuffer, two guest arrays of signed
32-bit coordinates, a count of four, and the graphics context on the stack.
Its caller at `0x351ea` constructs four-point colored game indicators.

The signature matches the WIPI polygon APIs documented in:
https://github.com/mirusu400/wipi-wiki/blob/main/src/content/docs/c-api/graphics.md

The mapping to filled polygons is inferred from the native drawing wrapper and
its indicator construction; original-device edge rasterization is not verified.
The implementation uses an even-odd pixel-center scan conversion, respects
existing LGT translation/view clipping and framebuffer color conversion, and
reads all guest vertices before drawing. No game-name branch or game archive
modification is involved. Only the observed `0xf0` import is added.

Tests cover concavity, reverse winding, empty/degenerate polygons, clipping,
large signed coordinates, and RGB565/ARGB framebuffers. The targeted LGT and
WIPI C suites pass (90 tests). Existing primitive graphics-context limitations
(such as custom operations on non-image primitives) are unchanged.

Android diagnostic replay of the rescue safe prefix followed by five seconds
of live continuation passed: `Running; paints=240`. The resulting frame shows
the dash tutorial, beyond the failed polygon operation. The replay used an
isolated cache save tree. This confirms progression through this failure, not
full-game compatibility or exact original-device rasterization.
