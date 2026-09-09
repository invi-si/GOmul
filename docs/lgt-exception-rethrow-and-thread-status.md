# LGT opening-scene stall: exception rethrow and thread status

MapleStory 2007's opening story stopped presenting frames and ignored OK while
an app worker consumed one CPU core. A five-second native sample captured
20,033 samples with no losses. Much of the time was in Java class-name reads
and assignability checks, not ARM instruction execution (4.70% self samples).

A bounded diagnostic captured the same exception, exception frame, and resume
address repeatedly. Guest disassembly at the catch dispatch demonstrated that
an unmatched exception is rethrown without a separate pop; the normal path
pops its exception frame. GOmul retained the exceptional frame, so rethrow
returned to the same handler indefinitely.

Unwinding now unlinks and frees the consumed guest exception frame while
retaining the pending exception and restoring its saved CPU context. Tests
cover nested rethrows reaching the outer frame and finally leaving the guest
handler chain, restored registers/SP, and normal push/pop afterward.

With the loop removed, the original failure became observable:
`Unimplemented: java/lang/Thread vtable index 13`. The guest call at 0x1c9ee
loads dispatch offset 0x38, takes no explicit arguments, and branches on the
result while handling a loading thread. The LGT ABI data now maps this slot
to `isAlive()Z`. The existing runtime reads the guest `alive` field; no scheduler
or timing policy changes are required. An integration test calls the actual
compiler slot and verifies false/true/false guest state transitions.

No game archive edits, scene skips, fake success, or save resets are used.
Temporary diagnostic instrumentation is removed from these changes.

Live validation: the normal APK with both fixes was installed in the Mac AVD.
The opening story continued presenting roughly 15–17 frames per second; OK
advanced to the next character-dialogue scene. Previously it produced zero
frames and did not respond. Full gameplay has not yet been tested.

Validation: 29 LGT unit tests and the hello-world integration test pass.
Workspace Clippy completes with existing warnings.
