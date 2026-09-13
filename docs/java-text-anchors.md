# Java text vertical anchors

In 간호사타이쿤2 at a 240×380 display, the usage screen draws “아무키나 누르세요” at (120,378) using HCENTER|BOTTOM. Before the fix, the shared MIDP graphics adapter used the horizontal anchor but ignored the vertical bits, drawing the text below the supplied y rather than above it. Increasing the framebuffer height therefore moved the text downward again instead of resolving clipping.

Apply BOTTOM and BASELINE offsets for drawString, drawSubstring, drawChar and drawChars before translation. Keep existing top/default positioning, clipping, horizontal alignment and font metrics. The current renderer uses a fixed 12-pixel line height and a baseline 10 pixels below the top; variable font sizing remains separate work. WIPI Java delegates these drawing calls to the shared adapter, so the correction is not specific to this game.

Contract: https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Graphics.html

Regression test compares actual image pixels across all four text APIs with equivalent TOP/BOTTOM/BASELINE/default positions, translation and a clip reaching the bottom edge. It also requires nonblank output.

Validation on the regular APK: the complete bottom prompt is visible at the unchanged 240×380/full-frame setting. Eight MIDP graphics tests pass; WIPI Java tests pass; targeted clippy passes. The broader MIDP run was stopped after the timed-alert and ticker tests failed to finish; that run is not counted as a suite pass. Diagnostic probes were removed from the shipped build.
