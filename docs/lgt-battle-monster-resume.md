# Battle Monster: Clip resume linkage

Rescue 1789028550413 fails during native method resolution: org/kwis/msp/media/Player.resume(Lorg/kwis/msp/media/Clip;)Z. The shared WIPI Java Player exposes resume(BaseClip), but compiled callers resolve the exact descriptor; the Clip subclass relationship cannot substitute a different signature at linkage time.

Add the Clip overload using the existing resume handler. The backend does not support resuming paused audio, so this returns false consistently with resume(BaseClip). It does not restart audio from the beginning or claim successful resume. This is a shared API compatibility fix, with no game-specific conditions or save changes.

The regression test resolves and invokes both exact descriptors on a Clip and verifies unsupported status without creating an audio player. Existing playback and stop tests remain in place. Full audio pause/resume support is outside this linkage fix.

The regular APK then exposes Runtime vtable index 12 (byte offset 0x34), LR 0x1154. The ARM caller obtains Runtime, invokes the slot without arguments, then enters a class-initialization helper that overwrites the return registers; no numeric result is consumed. This supports identifying the call as gc()V. Preserve the prior gc slot 13 and add slot 12 as an ABI alias rather than moving the old slot. This identification is based on native call-site analysis, not an original SDK symbol table. The generic vtable builder now populates repeated explicit method mappings with the same target and rejects collisions. The native regression invokes both slots and checks that a live String survives collection.

Validation: 35 WIPI Java tests and 40 LGT unit tests plus one integration test pass (76 total). Targeted clippy passes with the existing LGT class_instance collapsible-if warning. Installed the regular APK without resetting saves; live startup passes both failures, the usage notice and publisher logo, reaching the Battle Monster title screen. Full gameplay remains for manual testing. Updated GOmul-latest.apk.

## Forest freeze: dialogue thread

The app remained Running with zero paints/sec and unresponsive movement in the forest. A private checkpoint replay reproduced an uncaught exception in a.run: StringBuffer vtable index 22, LR 0xff9c. At 0xff84 the caller loads a UTF-16 unit with ldrh and invokes offset 0x5c; the failing argument is 0xc624 (오). Map append(C) at index 22. The same routine then calls offset 0x2c, tests the returned length against zero, converts to String, outputs the line, and calls offset 0x38 with zero to clear the buffer. Map length()I at index 10 and setLength(I)V at index 13.

Native tests invoke these exact slots, verify append returns the same receiver, preserve Korean and surrogate-pair text, distinguish the existing append(int) slot, and cover UTF-16 length, truncation, NUL padding, clearing and negative-length exceptions. 40 LGT unit tests plus one integration test pass. Targeted clippy passes with its existing warning. The corrected diagnostic replay no longer logs the thread exception, but diverges from the old tape after execution changes; it is not a successful full replay validation. Temporary diagnostic logging was removed from the regular build.

Regular APK live validation: Continue now reaches the battle/capture tutorial with Korean dialogue rendered (the * INFO prompt), beyond the frozen forest scene. Left there for manual testing. Preserved in-game saves; GOmul-latest.apk updated.
