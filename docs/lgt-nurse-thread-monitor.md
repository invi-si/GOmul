# Nurse Tycoon 2 startup monitor dispatch

The usage screen painted correctly but did not advance because GameCanvas.run terminated on `Unimplemented: java/lang/Object vtable index 9`.

The native ARM loop enters a monitor, invokes the no-argument notification at vtable word 10, then passes a signed 64-bit timeout to word 8 before leaving the monitor. Add LGT ABI mappings for Object.notifyAll()V at index 9 and Object.wait(J)V at index 7. Both use the existing Java monitor implementation; no synchronization, timing or game code is bypassed.

Regression coverage checks the fixed slots, monitor ownership rejection, successful owned notification and negative long timeout rejection through native dispatch. All 39 LGT unit tests and the integration test pass. The regular Android APK advances beyond the usage screen to the publisher logo on OK input.

The next menu-stage failure exposed String.indexOf(String,int), index 26. The ARM parser passes a delimiter string and starting index, increments the returned position, and uses substring(start,end) to extract the field. Map that overload to the existing Java implementation. Native-dispatch regression checks successive delimiters and the absent-delimiter result with Korean text.

After both mappings, the regular APK reaches the in-game town scene (date/time display, buildings and fatigue bar), with no fatal status. Leave later gameplay validation to the user; no save files were reset.
