# Pending redraws and input backlog

An Android AVD live trace of the 영웅서기2 save-selection menu isolated
severe input delay in the shared backend event queue. A 3.17-second trace
retained all records: eight physical touch transitions reached the worker
in 0.02–8.10 ms, but the six whose dequeue was observed waited
1,121–1,450 ms from the Android listener to backend dequeue. The remaining
two were still pending at the capture boundary.

The first key waited behind 22 Redraw events. Redraw dispatch accounted for
1,110 ms of its 1,121 ms queue residence; subsequent keys encountered
21–24 redraws. Paint throughput was still about 20.5/sec. Throughput did not
measure input responsiveness, and the trace does not establish the first
visually affected frame or physical presentation latency.

`Event::Redraw` carries no pixels, region, callback, or frame identity: it
invalidates the whole display. The frontend already combined requests until
delivery to the backend, but delivery could occur faster than the guest
finished repainting. The backend retained every duplicate notification.

`EventQueue::push_traced` now keeps the first pending Redraw and combines
additional redraw requests into it. It derives pending state from the queue
itself. Removing that event allows a subsequent request, including a request
made during painting, to queue again. Keys, repeats, timers and notifications
retain their FIFO order; no timer delays, CPU budgets or refresh caps change.
Trace metadata is added only for retained events.

This follows the asynchronous, coalescible repaint contract described by
[MIDP Canvas](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/lcdui/Canvas.html).
The change combines invalidation requests, not guest timer callbacks or
explicit framebuffer presentations. There is no game-specific dispatch.

Regression coverage submits 100 duplicate requests with intervening key and
timer events, checks their order, and verifies a new repaint can be requested
after dequeue. It also checks diagnostic queue alignment. Backend (with
input tracing), MIDP and KTF suites pass 131 tests, including the existing
native repaint/requeue tests.

Android checkpoint replay is build-specific; old Quick Saves keep their
existing build guard. Ordinary persistent game saves are not migrated or reset.

The fixed diagnostic APK was installed and booted successfully. A matched
post-fix menu trace is still pending: the active session moved to other games
during measurement setup. No post-fix latency or resolved-gameplay claim is
made yet. The normal APK with input tracing compiled out also builds
successfully. Plain backend queue tests pass separately without tracing.
