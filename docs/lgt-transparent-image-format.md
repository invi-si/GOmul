# LGT transparent image pixel format

Bio Chronicle's blue speaker nameplate rendered as a yellow/cyan checkerboard.
The original embedded PNG confirms the blue appearance and includes a transparent
palette entry. LGT advertised a 16-bit RGB565 display while decoded images with
any transparency retained 32-bit ARGB storage. Opaque images already used RGB565.

Decoded LGT images now expose RGB565 pixels consistently. A separate alpha plane
lives in guest memory after an ABI-compatible framebuffer-record prefix. Normal
image drawing reconstructs alpha using current guest RGB565 pixels, preserving
transparent and partially transparent drawing without a second color buffer.
Native framebuffer access observes the same authoritative RGB565 pixels.
No game identifiers or game resource edits are used.

Regression checks cover packed RGB565 row bytes and preservation of zero,
partial, and opaque alpha values. The user confirmed the visual glitch fixed
in the Android environment. Full cross-game regression remains ongoing.
