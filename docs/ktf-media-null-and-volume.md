# KTF media faults and volume capability

Follow-up: [class selection and delayed callbacks](ktf-class-loading-and-delayed-callbacks.md)
resolves the three object-representation startup panics by selecting the existing
compiled definitions before retained JAR classfiles. A general object bridge was
not required for those packages; the original diagnosis below records this audit's
earlier state.

A deeper Android audit separated two media failures from an unrelated object
representation failure. These changes are shared WIPI Java behavior, with no
archive/title-specific branches.

## Null Clip references

`Player.stop(Clip)` and `Player.play(Clip,boolean)` called `Clip::player`, which
implicitly dereferenced `ClassInstanceRef` before the JVM could handle a null
object. A null guest reference therefore panicked in Rust instead of entering
Java exception handling. Five independently reproduced KTF crashes had this
same native stack.

`Clip::player` now raises a guest `NullPointerException` before dereferencing a
null wrapper. A non-null Clip whose internal player has not been initialized
remains distinct: play/stop still return false. Valid loaded clips retain their
existing behavior. This restores the null-field-access fault boundary; it does
not invent successful playback or swallow the guest exception. The WIPI Player
documentation does not explicitly specify null-argument behavior; this choice
preserves the ordinary Java semantics of the field access the implementation
performs, and allows the guest's existing catch handler to run.

The regression test reproduced the host panic before the fix, then verified
catchable exception type for both entry points, empty-clip behavior, and existing
loaded-clip playback tests. Native Android boot and longer input probes are kept
as private evidence outside the repository.

References:

- [JVM getfield null behavior](https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-6.html#jvms-6.5.getfield)
- [WIPI 1.1.1 Player](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/media/Player.html)

## Volume capability count

`HandsetProperty.getSystemProperty("VOLUMELEVEL")` returned an empty string.
Three observed startup failures parsed it as an integer and failed. The documented
meaning is the number of supported volume steps, not the current percentage.
The emulated audio path currently provides one fixed-gain audible level, so the
property now returns `"1"`. This is a conservative capability declaration; it
does not add attenuation, claim 100 adjustable levels, or change audio gain.
Other property behavior remains unchanged. A regression test parses the returned
value using guest `Integer.parseInt`, checks the count, and checks that unrelated
property responses remain unchanged.

Reference: [WIPI 1.1.1 Volume](https://nikita36078.github.io/J2ME_Docs/docs/WIPI_API_1_1_1/org/kwis/msp/media/Volume.html).

## Diagnostics and unresolved object bridge

The default-off `compatibility-audit` build logs a native panic's source location
and backtrace while its per-run tracing subscriber is active. It installs the
hook once and retains the preceding hook. Ordinary builds compile this out.

Three other crashes reached `JavaValueCodec::object_to_raw`: actual classfile
instances created by the bytecode runtime reached code that requires a
KTF guest-memory object or array. The packages contain Java Clet wrapper classes.
The common classfile loader constructs a bytecode-backed class, whereas the KTF
native encoder only understands native-backed instances. Supporting that bridge
requires preserving object identity, fields, inheritance and GC references in
guest memory. Returning zero or inventing a host-pointer registry would not fix
it. No such workaround is included here.
