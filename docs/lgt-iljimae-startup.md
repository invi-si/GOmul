# LGT Iljimae startup method dispatch

The exported rescue records a jump to address zero from guest PC 0x379a2.
A diagnostic cold start identified the receiver at 0x48857f10 as
java/io/DataInputStream. The call loads the receiver vtable word at 0x68
(method index 25), and stores successive results in a short array.
The ABI data now routes that slot to readShort()S. This passes the original
failure; the next failure identifies java/lang/String method index 21.
Its caller searches for a delimiter, tests a negative result, splits at the
result using substring(II), then obtains the suffix through word 0x70.
The ABI data consequently maps indexOf(I)I at 21 and substring(I) at 27.
These are generic class ABI entries; there are no game-name conditions.

Method semantics use the existing Java runtime implementation. The CLDC API
contract documents signed two-byte reads and EOFException:
https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/cldc11api.pdf
Slot positions come from the observed native call sites, not that Java spec.

Regression test invokes the raw ARM-callable table entries and checks positive
and negative short values, byte order, EOF failure, delimiter position,
not-found, and suffix content. No game data is included in the test.

A subsequent loader stage calls the same DataInputStream receiver through word
0x74 and stores successive 32-bit elements. Mapped readInt()I at index 28;
the regression test checks the raw result 0x89abcdef from big-endian bytes.

The next stage exposed a separate exception-boundary bug. RandomAccessFile
construction correctly raised FileNotFoundException, but the innermost Java
SVC unwound straight into an outer native catch. The suspended host constructor
subsequently resumed as successful, with an uninitialized file descriptor.
Host-to-guest Java method invocations now isolate their guest exception chain
and restore the caller chain on return. Errors propagate through host Java
frames before the enclosing guest SVC enters its catch. A regression installs
an outer native handler, invokes a failing nested String method, checks the
Java exception type, then verifies that the outer handler is still available.

Final normal APK verification on the Mac AVD: cold startup passes all captured
failures and displays the usage notice, with status Running. 32 LGT unit tests
and the hello-world integration test pass; workspace clippy completes with
existing warnings. Diagnostic logging and the temporary panic hook were removed.
Full gameplay and the visibly clipped notice text are not validated by this fix.

New-game selection then hit a different null target at 0x7be2. Reproduction
identified the receiver as java/lang/Runtime immediately after getRuntime;
the caller requests collection alongside System.gc calls and ignores the result.
Mapped gc()V at method index 13 (vtable byte offset 0x38). The integration test
calls that guest target and checks that a live Java String survives collection.
This connects the existing collector and does not alter collection policy.

The following collection exposed object-header corruption, not a collector
policy problem. A guest-memory write trace showed inherited Thread.daemon
(word 8) writing past atdata/a's compiler-declared four-word instance storage,
overwriting a neighboring array's vtable pointer. AOT objects now reserve a
host-field tail in guest memory; inherited host fields without a confirmed ABI
mapping use that tail. Compiler offsets and explicitly mapped ABI fields stay
in their original positions. Allocation, clone, destruction and field traversal
use the same storage layout. No application-name condition or host registry
was added. A synthetic four-word Thread subclass regression verifies status and
wide-field access, unchanged compiler words, a neighboring canary, cloning and
cleanup.

Normal APK new-game validation now passes the null Runtime.gc target and the
array-header panic. It reaches an animated LOADING screen without a fatal error,
but remained there during the observation window: gameplay progress is not yet
confirmed. 33 LGT unit tests and one integration test pass. Workspace clippy
passes with existing warnings. All temporary write-watch/panic diagnostics were
removed from the normal build.
