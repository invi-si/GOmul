# Stopped emulator ownership cleanup

An Android process abort while opening another KTF game occurred in Scudo's mapping allocator during the new guest heap allocation, before guest startup. A clean-process launch of the same game succeeded. Repeated start/stop measurements in one process showed mappings increasing from 2,991 to 11,420 to 19,652 over three rounds.

Ownership cycles retained stopped sessions: executor tasks can capture their own System/Executor, registered ARM service contexts can retain the same ArmCore through the JVM, and Java method tables hold proxies that retain the JVM implementation which owns those tables. Dropping the frontend worker alone does not break those cycles.

Emulator destruction now explicitly releases pending tasks, queued events and ARM service handlers. KTF/LGT drop tasks before CPU handlers and explicitly empty their Java callback tables; Java-only SKT/J2ME also shut down their executors. References are removed under the respective lock and destroyed after releasing it, since future/context destruction may reenter those objects. Shutdown happens after polling has stopped, not as part of running guest scheduling.

Regression tests verify that task-owned executor and handler-owned CPU weak references expire after shutdown, including repeated shutdown calls. Android lifecycle instrumentation uses only cache saves and records memory/mapping counts across repeated launches. No game archive or normal save is edited.

The final normal APK passed eight consecutive launches of the reported game in one Android process: post-stop mapping counts stayed between 2,954 and 2,956, versus approximately 8,200 additional mappings per subsequent launch before the fix. Every launch remained Running. The initial task/service cleanup alone was insufficient; adding explicit Java-table teardown removed that growth. The affected backend/CPU/platform suites passed 217 tests, with KTF/LGT tests rerun after the Java-table change.

Three successive Quick Loads of a newly saved startup checkpoint also completed successfully in the normal APK, with the game still Running and 2,933 mappings after session stop. These checks used isolated cache saves; the normal library and saves were preserved.
