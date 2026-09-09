# LGT opaque-image pixel format

## Failure and evidence

Iljimae Hero Story 2 displayed solid rectangles instead of Korean letters and striped dialog borders. Its font is assembled by guest code from small PNG strips of Hangul components. Diagnostic logging showed direct pointer access to the 16-bit screen and to decoded 32-bit font images, including 5x304, 7x209 and 9x255 strips. The guest uses the display pixel layout to read those image buffers.

## Change

Opaque decoded LGT images are converted to tightly packed RGB565 before their public image/framebuffer wrappers are constructed. Image and framebuffer access still reference the same guest-backed allocation, so guest writes remain authoritative. The temporary ARGB allocation is freed. Images containing transparent or partially transparent pixels retain their existing ARGB representation and blending behavior.

This is an LGT graphics change, with no application-name checks, archive edits, font replacement, CPU changes or timing changes. Raw pointer access to alpha-bearing images remains outside the evidence established here; this patch does not claim complete native image-format compatibility.

## Validation

- 26 LGT tests passed, including RGB565 byte layout on odd-width rows, alpha preservation, and the earlier string-comparison import test.
- The import regression test was also made independent of the optional futures executor feature so the LGT suite runs standalone.
- Formatting and workspace clippy completed with existing warnings.
- Normal ARM64 Android APK built and installed without clearing saves.
- The same introductory notice now displays readable Korean text and an intact border on the Mac Android AVD.
- Temporary tracing was removed. Its oversized log was moved out of the save directory after triggering the checkpoint input-size guard; no game save was reset.

Full gameplay and other affected games, including Rhythm Festival, are not yet validated with this change.
