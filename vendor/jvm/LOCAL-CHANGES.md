# Local JVM dependency patch

Upstream: `jvm` 0.1.1 from crates.io, https://github.com/dlunch/RustJava (MIT; see LICENSE).

Changes in src/jvm.rs and src/garbage_collector.rs add an optional native root snapshot provider. Candidates are matched against registered live object identities before the existing transitive marker visits them. An incomplete snapshot skips collection. Other JVM users have no provider and retain their previous collector behavior.

The KTF adapter supplies guest image/static, register and active stack contents at collection time. It does not keep a host registry of guest references. This is conservative: a pointer-shaped integer can retain an otherwise dead object until overwritten. It does not implement JNI handles or a moving collector.

Integration regression coverage lives in wie-ktf/src/runtime/java/jvm_support.rs; core snapshot coverage lives in wie-core-arm/src/core.rs.
