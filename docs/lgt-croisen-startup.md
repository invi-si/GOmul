# Croisen startup phone identity

The LGT archive starts with a native Clet. Startup requests PHONENUMBER into a 16-byte buffer, copies 12 bytes to its phone buffer, then calls strlen and subtracts four before calling memcpy (service 0x414). With no configured identity, PHONENUMBER returned an empty string while MIN already returned 01000000000.

The resulting copy length was 0xfffffffc (R0=0x400fff98, R1=0x150b7e0, R2=0xfffffffc, LR=0x1b63). The copy eventually faults at source 0x1520000. Before that fault, destination writes corrupt guest JVM metadata. Error reporting then repeatedly fails to find java/lang/String.<init>:(II[C)V, with an empty String method table, until the Android thread stack overflows. This explains why the whole app disappeared rather than displaying an ordinary fatal-error screen.

Use the existing default emulated identity for both PHONENUMBER and MIN in the shared WIPI C property API. Explicit platform identities still take precedence. No game-name conditions, copied game data, save resets, timer changes or CPU changes are involved.

Regression coverage exercises both aliases with the default and configured identities, exact-size NUL-terminated output and short-buffer rejection without writes. This fixes the observed startup trigger; it does not claim that arbitrary guest memory corruption can no longer break exception reporting.

Validation: 59 WIPI C unit tests, 40 LGT unit tests and one LGT integration test pass (100 total). Targeted clippy passes with an existing collapsible-if warning in LGT class_instance.rs. The regular native APK was installed in the Android environment without resetting saves. Croisen now progresses past startup into the tutorial character/status dialogue; no full-game compatibility claim. Temporary crash probes were removed, and GOmul-latest.apk was updated.
