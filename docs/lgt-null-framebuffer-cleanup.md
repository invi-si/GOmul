# LGT null off-screen framebuffer cleanup

Metal Slug Survival failed during `CletWrapper.startApp` with an invalid access at address zero. Guest state-transition cleanup called the wrapper at `0x14a8`, which loaded an empty framebuffer slot and invoked WIPI import `0xcb`. GOmul's destroy handler dereferenced that null handle before validating it.

`destroy_offscreen_framebuffer` now treats a null handle as empty cleanup. Nonzero handles still pass through the existing ownership and backing-pointer checks; invalid accesses are not generally suppressed. No game-specific branch, archive modification, save reset, or timing change is involved.

Validation: 27 LGT unit tests passed. The new import-level regression exercises repeated null cleanup, valid create/destroy cycles, and invalid nonzero handle rejection. Formatting and workspace clippy completed with existing warnings. The native Android APK was rebuilt and installed over the existing application. On the Mac Android AVD, Metal Slug now passes the prior startup failure and renders the introductory notice, rating screens, and subsequently a Stage 1-1 scene. Full gameplay is not certified by this startup check.
