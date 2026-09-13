# LGT Legend of Master: TextComponent linkage

The latest local Rescue reported a startup failure resolving instance field
`org/kwis/msp/lwc/TextComponent.iMode:I`. The full native error, register dump,
and stack reproduced exactly on the originating APK using isolated initial
saves. The report has no successful tick prefix: this is startup linkage,
before gameplay, rather than a fast-forward or framebuffer failure.

The shared WIPI Java TextComponent prototype now declares the integer instance
field. Its storage follows normal guest-object initialization and lifetime;
there is no game-name branch, hard-coded returned value, or host-side registry.
The field is appended after existing fields to retain their declared ordering.
This restores linkage/storage only, not a complete input-method implementation.

Regression coverage constructs both TextBoxComponent and TextFieldComponent,
checks inherited zero-initialized mode storage, writes several integer values,
and verifies instance isolation and unchanged text, constraint, and child height.
All 33 WIPI Java tests, 34 LGT unit tests, and one LGT integration test pass.
Workspace clippy and the regular Android APK build pass with existing warnings.

The patched APK fresh-starts from an isolated copy of the rescued initial saves
and reaches the Korean usage notice (two guest paints), without the fatal error.
Automated OK, 5, and soft-key inputs did not advance that notice in the first
checks. The follow-up below identifies why. Game bytes, regular saves and Quick
Save slots were not patched or reset. The rescue ZIP/game remain private local
fixtures.

## Follow-up: silent update-thread failure

A diagnostic build found `net.wie.WieError: Invalid memory access; address: 0`
in `f.run()V`. The Java Thread wrapper logged the uncaught exception and ended
that thread, while the application still reported Running. This was not a
connection loop or a missing acknowledgement key.

The guest call at `0x93adc` invokes Java import `0x20` (pop exception frame),
after yield/sleep inside a registered catch scope. GOmul's invocation wrapper
previously cleared one global exception context before awaiting native execution
and restored it only when execution completed. Other asynchronous invocations
could overwrite a suspended invocation's chain; pop then dereferenced zero.

Each invocation now retains its two-word exception context in guest memory,
installs it only during a poll, and restores the enclosing context before
yielding to the executor. Nested calls remain isolated from parent catches.
Completed/canceled scopes release their context and remaining owned catch frames.
No timer delays, guest code, key mapping, or CPU scheduling policy was changed.

Regression tests cover A/B/A resumption order, independent pending exceptions,
balanced pop, nested scopes, cancellation and complete allocation recovery.
Existing unwind/register restoration and rethrow tests remain enabled. The
diagnostic reproduction advances from the notice into animated publisher
screens (105 paints in the short continuation) with no uncaught thread error.
This fixes the observed stall; full gameplay compatibility is still unverified.

Final regular APK: 36 LGT unit tests and one integration test pass; workspace
clippy/build pass with existing warnings. A 20-second isolated run produces
190 paints and shows the live "press any key" notice. Quick Save, a speed
change, and Quick Load pass deterministic reconstruction. The corrected APK
is installed in the AVD and copied to the local latest-download path.

## Character-confirmation resource loading

A subsequent rescue fails in `f.paint` with an unimplemented
`java/io/ByteArrayInputStream` vtable slot 13. The failure reproduces exactly on
its originating regular APK. At guest `0xea7bc`, the stream receiver calls
vtable byte offset `0x38` with a low/high signed-long byte count after computing
the remaining resource offset. LGT InputStream slot 13 is now mapped to
`skip(J)J`, between the read overloads and available/close. The existing runtime
implementation supplies stream-position updates and EOF behavior; no game patch
or fake return value was added.

The native-vtable regression calls slot 13 on ByteArrayInputStream and checks
negative/zero counts, positive progress plus the next byte, a count above 32 bits,
EOF capping, and both return registers. All 36 LGT unit tests and the integration
test pass. The patched diagnostic replay's pre-failure framebuffer matches the
rescued frame byte-for-byte (240×320, paint count 476). Continuing five seconds
passes the former crash into the story intro (525 paints).
An independent rescue-verify replay also matches the captured prefix frame and
saved-data tree, then continues without the fatal error. Workspace clippy and
the regular APK build pass; the regular build is installed and the local latest
APK updated. User game archives and save/checkpoint slots remain untouched.

## Dialogue Vector calls

The next rescue reproduced a null call target at `0xe2d08`. Temporary receiver
inspection identified `java/util/Vector`; the caller appends a text array through
vtable byte offset `0x78` (method slot 29). Mapping `addElement(Object)` let the
same replay reach an unmapped slot 19. That caller uses the returned integer as
the number of queued text entries before a marker (`indexOf(Object)`), then
retrieves/removes the first queued entry using slot 24 (`firstElement()`) and the
already-mapped `removeElementAt(0)`.

All three slots are now declared in the shared LGT ABI data. Existing Java
Vector implementations supply the behavior. Regression calls go through the
native vtable and check append order, duplicate/null elements, first-match search,
missing-element -1, removal and the empty-vector exception. Temporary receiver
inspection is removed from the regular implementation.

The same routine then exposed an incorrectly declared Java import `0x12`.
At `0xe304c`, it passes the retrieved object's native class descriptor and a
`java.lang.Class` object returned by GetArrayType. The old handler read the
second argument as a UTF-8 class-name string. The source now resolves through
its native descriptor and the target through the existing Class-object resolver
before the JVM assignability check. Tests invoke the actual
native import for matching/different primitive arrays, reference-array covariance,
Object/String directions, and a Vector/List interface relationship.

All 37 LGT unit tests and one integration test pass. The patched diagnostic
replay matches the rescued pre-failure framebuffer byte-for-byte, then reaches
the cave dialogue (486 paints after five seconds) without the fatal error.
This verifies the failing dialogue path, not the rest of the game.
An independent replay verifies the prefix framebuffer/saved-data oracle and
continues successfully. Workspace clippy and the regular APK build pass with
existing warnings. The installed AVD APK and local latest APK are updated;
game archives, regular saves, and Quick Save slots are preserved.
