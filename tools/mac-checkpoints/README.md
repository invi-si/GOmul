# GOmul Mac checkpoint helper

See [build/setup instructions](../../docs/build.md) and the [user-supplied data flow](../../docs/action-hero-setup.md).

This is a host-assisted whole-AVD checkpoint implementation, not portable CPU/JVM serialization. Private tokens, connection configuration, per-game slots and snapshots are generated locally and excluded from source control. It supports one selected Mac AVD per user.

Quick slots and protected startup are keyed by the imported archive's SHA-256. The backend locks operations, publishes newly verified snapshots atomically, creates recovery before Quick Load, and preserves other games' disk state before restoring the active game's snapshot. A failed disk transfer leaves its host backup for recovery. The snapshot also contains Android system state and the captured APK version.

Run tests with `python3 -m unittest discover -s tools/mac-checkpoints -v`. Tests use fake ADB and synthetic data; they contain no commercial game files. The HTTP helper requires its locally generated token and uses a loopback-only ADB tunnel.
