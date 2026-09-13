# KTF Clet repaint delivery and native presentation

A manual Rescue from 짜요짜요타이쿤3 reproduced an immediately black screen.
The emulator remained Running: timers and native image operations continued.
The baseline replay produced only the initial black frame. A fresh five-second
diagnostic run recorded 49 native repaint requests, 1,304 image draws and 213
framebuffer copies, without a guest `MC_grpFlushLcd` call.

## Cause and shared correction

`MC_grpRepaint` posted a frontend redraw but did not set the guest Display's
pending repaint flag. The Java event queue therefore ignored these requests
before the first explicit native flush. The KTF adapter now marks the request
before posting the existing event; it does not invoke the guest callback inline.

Delivering the callback alone produced 48 frames in a fresh five-second probe,
but every frame was still black. The native game was drawing into its WIPI-C
screen framebuffer while MIDP presented its separate Java backing image. The
registered native paint callback was an empty return, so waiting for that
callback to call FlushLcd could never work.

The KTF adapter now binds a guest-owned Runnable to Display when a native
screen framebuffer exists. At the end of a requested native paint cycle, it
presents the current guest framebuffer through the existing FlushLcd path.
Explicit native flushes suppress duplicate presentation. Actual Java screen
drawing selects Java presentation; merely changing the clip or resetting
Graphics does not. A completed implicit native flush has the same presentation
ownership postcondition as an explicit flush, preventing a later Java paint
from overwriting it with stale pixels. Requests made during painting remain
pending for the next cycle.

All mutable selection state and the framebuffer handle live in guest fields.
The repaint request does not allocate a framebuffer or copy guest pixels.
The presenter reads the framebuffer at callback completion, so intervening
guest writes are visible. Existing save/replay build guards remain enabled.

No class/game-name special case, unknown native slot mapping, timer change,
CPU scheduling change, forced input or refresh-cap adjustment is involved.
This does not implement arbitrary composition of native pixels with Java UI
chrome or establish that the original platform used physically aliased buffers.

## Contract evidence

- The WIPI-C graphics specification describes `MC_grpRepaint` as a queued
  request for `paintClet`, not an inline call. Its double-buffering section
  requires presentation before screen-buffer writes reach the LCD.
- The original KTF SDK's *WIPI-M022-과금 API 개발자 가이드* sample obtains the
  screen framebuffer and draws from `paintClet` without explicitly calling
  FlushLcd. The SDK Java Display documentation states that serviceRepaints
  includes buffer flushing.
- An independent [2007 WIPI-C author example](https://breakbrain.tistory.com/18?category=144234)
  likewise draws text in `paintClet` without an explicit flush, using the Clet
  wrapper classes and showing the rendered result.

Unidentified KTF Interface12 slots were investigated but left unchanged.
Other native games call the same slot followed by an explicit FlushLcd, so
its proximity to repaint is insufficient evidence to label it a flush API.

## Sprite clipping

Once presentation worked, the menu exposed another shared issue: DrawImage
ignored its graphics context, displaying whole sprite atlases where the game
had requested clipped menu slices. The image path now honors the context's
signed, right/bottom-exclusive clipping rectangle and rejects nonpositive draw
dimensions. Source coordinates retain their original alignment when clipping.
Context initialization disables clipping, as specified; an explicitly empty
rectangle still draws nothing, and a NULL clip reset restores the default.
Other drawing modes and unrelated native mappings are outside this change.

## Validation

Regression coverage includes deferred/coalesced delivery, repaint requeue,
Java/native ownership, explicit flush suppression, live RGB565 guest pixels,
presenter GC reachability and presentation after a previous native flush.
Game archives, recordings, screenshots and initial saves remain private test
artifacts. Android replays use isolated copies of the initial saves.

The fresh-start replay now reaches the animated title screen. Replaying the
original rescued session and sending a confirm press reaches the clean main
menu, remains Running, and produces 134 cumulative paints in that bounded run.
The intermediate build without clipping showed repeated menu atlases; the
same rescued input sequence with clipping presents correctly separated rows.
These paint counts are diagnostic observations, not gameplay FPS or proof of
full-game compatibility.

The MIDP, WIPI Java and KTF suites pass 122 tests; the WIPI-C graphics selection
passes 29 tests. Web/WASM compilation and targeted Clippy complete; Clippy
reports existing style warnings outside this fix. No physical-device or
full-playthrough result is claimed.

The ordinary non-audit Android APK is installed in the Mac AVD. Its installed
APK hash matches the packaged artifact, and all 217 game archives present
immediately before installation remain present afterward. Regular game data
was not reset or replaced; test replays used app-cache save copies.
