# Browser revival: client-only WASM baseline

Status: local development prototype, not a published release or an iPhone compatibility certification.

## Scope and catalog

The September 13 Android library snapshot contains 229 game archives, grouped into 214 titles. All archive-bearing entries are included at the user's request. Another 27 leftover folders contain no ZIP/JAR and are not catalog games. Android compatibility is the selection criterion, not evidence that every game already works through the browser adapters.

The private snapshot and ROMs remain outside this repository. No Android personal saves were copied into the browser catalog.

## Architecture

`tools/web-catalog/local_wasm.py` serves static assets and catalog/archive GET requests on loopback. It does not launch native emulator sessions or receive save data. `catalog-index.ts` now starts `wasm-player.ts` instead of the native session client.

A dedicated module worker owns the WASM emulator. The page handles touch/keyboard input, canvas presentation and audio output. A readiness handshake waits for asynchronous WASM initialization before sending the game. Worker frames are copied/transferred to the page; presentation keeps the newest frame at each animation callback. This does not skip guest CPU instructions or guest drawing calls.

The backend's existing 8 ms tick budget and guest timer implementation remain unchanged. The worker yields with a timer between updates instead of waiting for the page's animation callback. This changes host service opportunities; it is not proof of original-phone pacing and must be checked in gameplay. No interpreter optimizations were added in this stage.

`?runtime=main` selects the old page-thread driver for a comparison using the same built emulator. `?profile` exposes bounded worker-input delivery measurements as `window.gomulWorkerMetrics`. Those measure page enqueue to entry into the worker key handler, **not physical input-to-photon latency or final guest consumption**.

## Browser saves

IndexedDB storage is local to the browser profile and origin. Each catalog version has a separate namespace, even if guest application IDs match. A Web Lock prevents two tabs in the same profile from running the same catalog game concurrently.

Record write/delete promises resolve after transaction commit. Returning to the library waits for outstanding IndexedDB writes before terminating the worker. This does not claim to force a game to save unsaved progress, or protect against operating-system termination, cleared website data or private-browsing disposal.

The browser database adapter now follows native record allocation (first free positive ID), path normalization, empty database existence, directory operations and record deletion. Compound record keys separate database names from record IDs. Existing flat-key saves migrate when opened; original keys are retained as recovery copies. Previously ambiguous flat keys cannot be reconstructed with certainty. File storage retains per-application separation, sparse writes and truncation.

“새 저장으로 시작” selects a fresh private namespace and installs initial data again. It retains the old namespace; “이전 저장으로 되돌리기” switches back. This is a fresh-start/undo facility, not an emulator quick-save checkpoint. Normal in-game saves remain browser-local.

Quick-save slots, native rescue capture, protected startup checkpoints and a separate companion-file import UI remain unported. Disabled quick-save/load controls identify unavailable features; do not describe this prototype as full APK parity.

## Android compatibility wiring

| Area | Browser status |
| --- | --- |
| ARM/JVM, vendor APIs, fonts/images, timer cancellation, guest networking behavior | Built from the same current shared Rust crates as Android; no separate old emulator checkout |
| Archive preparation | Normalizes one nested application root and rejects ambiguous application roots |
| KTF bundled P/p data | Uses the existing shared KTF installation loader |
| Inotia 1 original bundled preferences | Exact original preferences fingerprint selects matching emulated identity and numeric direction mapping; explicit user identity takes precedence |
| LGT accompanying data | Recognized companion records are installed only where missing, with supplied identity or the known Action Hero importer default; existing records are preserved |
| Input | Existing keyboard layout retained; Korean input mode reaches shared guest event handling; directional/numeric aliases share held-key state |
| Display | Native-style opaque LCD presentation retains RGB, follows actual framebuffer dimensions, and exposes per-game display overrides; full framebuffer override is consumed by the shared LGT path |
| Speed | Global browser-persistent 0.25–3× guest clock setting; greater than 2× labeled experimental; not a promise of achieved throughput |
| Failure reporting | Unhandled worker asynchronous rejections surface as errors rather than silently leaving a frozen view |
| Native optimized CPU feature flags | Not automatically enabled for WASM; require their own correctness/performance evidence |

Launcher metadata is confined to the browser adapter. No new game-specific branches were added to shared runtime code. Launch preparation runs in the worker and diagnostic main-thread catalog paths, and in the standalone file library. Setup and emulator execution share the per-game save lock.

If an earlier browser attempt already changed Inotia's preferences, return to the library, refresh to load this build, reopen it and select **게임 설정 → 새 저장으로 시작**. This preserves the previous browser save instead of overwriting it. Its data/identity setup was checked directly; the reported download screen was not manually replayed in this validation.

## Run locally

Requirements: the repository Node dependencies, Rust's `wasm32-unknown-unknown` target, `wasm-pack`, and Python 3.

```sh
npm run build:prod
python3 tools/web-catalog/local_wasm.py --games /path/to/private/android-games-snapshot
```

Open `http://localhost:8791/`. The game directory has one subdirectory per import with `library.json`, a ZIP/JAR and optionally `thumbnail.jpg`, matching the Android library layout. The original server-based web catalog is not needed.

Phone access needs an explicit trusted HTTPS setup; the loopback development URL is only accessible on the Mac. No public deployment is part of this change.

## Validation

- Optimized production WASM build; shared runtime builds from the current working tree.
- 14 Rust adapter tests pass: launch settings, metadata, clock continuity, database migration/lifecycle, sparse file storage, and opaque LCD presentation.
- 37 browser-side unit tests pass: catalog, platform/input mapping, held-key aliases, browser locks/storage, global speed/per-game settings, and IndexedDB transaction/shutdown completion.
- TypeScript type checking passes. Production bundling passes with asset-size warnings (the optimized WASM is about 17.8 MiB).
- Fresh Chromium and desktop WebKit runs of Mini Game Heaven 1 reached the menu, accepted the scripted key sequence and returned to the library without page/console errors. These checks created private IndexedDB records and made no native emulator-session requests.
- Direct built-WASM setup checks in Chromium and WebKit recognized the actual Inotia 1 archive's matching identity/numeric directions. Action Hero 3 imported its 2,720-byte accompanying `savedata`, retained a deliberately changed record on repeat preparation, and installed separately under a second namespace. These were isolated browser contexts, not the user's saves. No Inotia gameplay or download-screen navigation was performed.
- Chromium settings integration checked the Korean-mode toggle, persistent 2× speed and per-game dimensions, restart, creation of a fresh namespace, and return to the previous namespace. Both initialized storage namespaces remained after exiting; no page errors occurred. This checks UI/storage wiring, not Korean composition in every game or dimension support beyond the shared platform implementation.
- Before the worker change, fresh desktop Chromium runs reached visible Korean introductory screens in Welusia and Reicarna without page errors or native-session requests. This was startup smoke coverage, not completed gameplay compatibility.
- Actual iPhone Safari performance remains unmeasured. Desktop WebKit/mobile viewport checks, where reported, establish browser-engine/layout compatibility only.

The Mini Game Heaven 1 scripted input check observed median page-to-worker key-handler delivery of approximately 0.1 ms in Chromium and 1 ms in desktop WebKit. This is a short startup smoke result, not a gameplay latency or physical iPhone measurement. The full Android-supported catalog has not been browser-validated.

Local rebuilds retain hashed JS/CSS/font/WASM assets for already-open tabs, avoiding missing chunks after a rebuild. Prepare a fresh empty output directory when packaging a release so old local assets are not shipped.
