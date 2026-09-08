# Standalone Android checkpoints (experimental)

Quick Save and Quick Load are beside CALL. Hold Quick Load to undo the last successful load. Each imported archive has its own Quick Save and recovery slot, stored in app-private storage. Slots survive closing the app; uninstalling or clearing app data deletes them. No game files or starter saves are bundled.

On physical Android, these are **replay checkpoints, not instant memory snapshots**. Starting a game records external inputs, clock samples, redraw-consumption boundaries, and the observed guest task order. Quick Save stores this journal and the session's initial persistent data. Quick Load reconstructs a separate runtime while the current runtime is suspended. It compares the resulting display and persistent data before replacing the current session. Replay failures restore the original persistent data and leave the current runtime available. An interrupted load is rolled back at the next game launch.

Important limits:

- Loading takes longer for longer sessions; reconstruction currently has a three-minute limit.
- Recording uses at most 64 MiB of journal memory, plus game-data copies. Reaching the limit disables new checkpoints for that session. Earlier checkpoints remain on disk.
- Active audio restarts from the beginning after loading. Audio playback position is not captured.
- Checkpoints are local and tied to the game archive and APK build. They are not compatible with Mac AVD snapshots.
- Display and file equality do **not** prove all hidden CPU/JVM state is identical. The user confirmed Quick Save and Quick Load work in the Android virtual device using the standalone APK path, without the Mac helper. Automated correctness tests and physical-phone checkpoint validation have not been performed; this is not a claim of universal save-state support.
- Recording adds overhead. Its performance impact has not been measured.

The Mac helper still uses its existing whole-AVD snapshots when configured. It retains its existing protected-startup behavior. Importing companion game data remains automatic and independent of Quick Save/Load.

Validation: the APK compiled successfully, and the user manually confirmed Quick Save/Load in the Android virtual device. The assistant did not run automated tests or inspect gameplay, as requested.
