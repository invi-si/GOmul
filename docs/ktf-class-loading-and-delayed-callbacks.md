# KTF class selection and delayed UI callbacks

## Retained classfiles can shadow native definitions

Three tested KTF packages contain both `Clet` wrapper classfiles and compiled
definitions exported by `client.bin`. The inherited parent-first `loadClass`
searched the JAR before KTF's `findClass`. It constructed a bytecode-backed object
and passed it into a native superclass constructor, where the KTF object encoder
panicked on a failed downcast.

The earlier [media audit](ktf-media-null-and-volume.md) correctly located that
representation mismatch, but its proposed next step—a general object bridge—was
larger than necessary for these packages. Executing the image's class lookup
confirmed that usable compiled counterparts already exist.

KTF's loader now returns an already loaded definition first, then asks the
native image for an exact exported name, and falls back to the existing parent
loader when no native definition exists. Array classes retain the JVM's normal
construction path. No game/class-name exception list, synthetic object pointer,
host metadata registry or archive modification is involved. Resource lookup is
unchanged.

The focused test supplies distinct definitions with the same name from an image
and a parent loader. It verifies native precedence, cached Class identity, parent
fallback, array loading, inherited Vector methods/fields, reference fields,
object encoding/decoding identity and GC retention. Removing the new `loadClass`
override makes that test fail.

This is not general support for passing arbitrary bytecode-only objects to a
native KTF method. That broader limitation remains. Java permits custom loader
policies; this policy is specific to KTF's compiled-image execution model, with
no change to LGT/SKT or the generic Java loader. See the JVM specification's
[loading and linking chapter](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-5.html).

## Delayed Display.callSerially was a no-op

알바 전설 편의점2 repeatedly relies on
`Display.callSerially(Runnable,int)` to continue after painting its introduction.
The overload previously returned without registering the callback.

The [WIPI 1.1.1 Display specification](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/lcdui/Display.html#callSerially(java.lang.Runnable,%20int))
defines asynchronous event-thread delivery after the requested delay, treating
negative delays as zero and rejecting a null Runnable.

The overload now uses the existing backend timer event to make the Runnable
eligible at its deadline, then enqueues it through the existing serial-callback
path. It does not run the Runnable inline or on a new worker thread. Temporary
JVM global references retain the guest Display and Runnable until enqueueing;
their fields and identity remain in the normal guest object representation.
Zero/negative delays use the existing immediate enqueue path. There is no change
to the event loop, timer cancellation, sleep intervals, scheduler policy or
painting caps.

The fake-clock regression checks no execution before 100 ms, two independent
registrations of the same Runnable, exactly-once delivery for each registration,
same event-task execution, GC retention while pending, asynchronous zero/negative
delays and null rejection. Restoring the old no-op makes it fail. Existing MIDP
tests continue to cover painting before serial callbacks and timer/input fairness.

## Newly exposed storage call remains unresolved

After correct class selection, 셔터 proceeds into native startup and reaches
database-table slot 11. Its historic label, `MC_dbGetRecordSize`, is not evidence
of the actual KTF contract. The caller loads the target into R0, calls through
`bx r0` without a database handle, and compares the return value with 5999 before
setting a flag. This rules out treating it as a standard handle-based record-size
query. It suggests a storage-capacity/availability check, but does not establish
whether the value means total or available space. No speculative mapping or
forced-success return was added. Temporary register/stack diagnostics were removed.

Private Android boot/input probes and rescue results are retained outside the
repository. Running/painting during a bounded probe is not full gameplay
compatibility, and any cross-build replay divergence must be reported separately
from a successful regression replay.

## Validation follow-up: empty executor with a frozen clock

Two MIDP tests stalled after their work completed because `Executor::tick` kept
waiting for its eight-millisecond slice to expire with no remaining tasks. Their
explicit test clock was no longer advancing. An empty executor now returns
immediately. Nonempty task execution, ordering, deadlines and timer handling are
unchanged. A bounded-clock regression reproduces the old spin without hanging
the test process, then verifies an empty tick, a subsequently spawned yielding
task, and return to idle. The formerly stalled alert and ticker tests pass
without modifications to their assertions or clocks.
