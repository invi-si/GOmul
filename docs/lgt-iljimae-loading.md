# LGT native Thread.run mapping

Starting a new game in 일지매영웅전기 kept painting LOADING while the loader thread returned immediately. A diagnostic trace showed Thread.start followed by the host base run() with no Runnable target, so no loading work occurred.

The archive’s compiler table for atdata/a extends java/lang/Thread and places its native loading routine in virtual slot 11 (table word 12). The LGT ABI already mapped start to slot 10 but omitted run. Add run()V at slot 11 so generic compiler-table linking reconstructs the override rather than invoking the empty base implementation. No game-name checks, archive edits, timer changes or loading bypass are involved.

Validation: all 39 LGT unit tests and the integration test pass. The new synthetic native Thread regression uses a hard-coded compiler slot with absent member metadata and verifies virtual run changes a guest field. On the API 36 Android emulator, the regular APK progresses from the usage notice through New Game into the opening scene and mother’s dialogue. Later gameplay is not yet validated.

Temporary CPU/host probes were removed before the regular build. The diagnostic frames.log was removed from this game’s device save directory because its size exceeded the checkpoint limit; no game save was removed.
