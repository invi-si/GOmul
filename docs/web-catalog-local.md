# Localhost game catalog

The searchable browser catalog uses the native GOmul runtime on this Mac. The browser provides the screen, keypad and settings; the local server runs the same emulator adapter used by Android. It is not an APK embedded in a web page, and requires the local server to stay running.

## Build and run

With Python 3, `unar` (including `lsar`, for ALZ attachments), Node/npm, the repository's Rust toolchain, `wasm32-unknown-unknown`, and `wasm-pack` on PATH:

```sh
npm ci
./tools/web-catalog/build-native.sh
npm run build:prod
npm run start:catalog
```

Open **http://localhost:8790**. The native build uses a source-derived checkpoint build identity and the production Thumb inline/table features. Rebuild it after changing Rust code. Checkpoints from a different build may be rejected; in-game saves remain separate.

The standalone WASM importer remains at `/index.html` for development. It does not gain the native catalog features and its existing browser saves are not migrated.

## Features

- Cover-grid library with a list-view toggle, name sorting, Korean search, carrier filters and thumbnails. Identical titles (normalized Unicode/spacing/case) share one card; selecting it asks which carrier version to run. Sequels and named editions stay separate. The carrier filter narrows the offered versions. Each version retains its own archive identity and saves. A second chooser appears if the selected post has multiple game archives.
- Recorded incompatible versions and user-requested support exclusions are excluded from the catalog and attachment/launch lookup using `tools/web-catalog/excluded-games.json`, with their sources recorded. The original 19 LGT and five KTF incompatible classifications remain; additional catalog removals are recorded in [web-catalog-support-exclusions.md](web-catalog-support-exclusions.md). This applies to existing cached catalogs too. Titles containing 맞고 or 올림픽 are additionally excluded across every carrier by user request. Other carrier versions and sequels remain independent; lower-priority or untested entries are not automatically classified as incompatible. Update this data when a compatibility classification changes.
- Mobile browsers show the game above bottom-left directions and a bottom-right keypad, including in landscape. All desktop platforms hide the on-screen game controls. Desktop keyboards use WASD for directions, 0/minus/equals for 1/2/3, O/P/left-bracket for 4/5/6, L/semicolon/quote for 7/8/9, and comma/period/slash for */0/#. Other bindings remain unchanged (Z/X/C = */0/#; Space/Enter = OK; Shift = soft keys). Save/load and settings remain visible; Korean mode is available in desktop game settings. Mac/Linux use the same desktop keyboard mapping; settings, save/load and back remain accessible.
- Keyboard and on-screen keypad support simultaneous directions/buttons, Korean input mode and audio controls.
- Per-game Quick Save/Load, undo-load recovery, and a separate protected startup checkpoint with automatic loading.
- Per-game display dimensions, full framebuffer option for LGT, emulated identity, and 0.25–2× execution speed. Display/identity changes restart the game. Fast-forward uses the existing native runtime behavior and may affect sound/presentation.
- Companion-data import from the selected archive when present, preserving existing earned data. Recognized Super Action Hero 3 data uses its established emulated identity. Create a protected startup checkpoint locally after reaching the desired scene; no checkpoint or game data is bundled with the source.
- Manual rescue ZIPs, including a screenshot and native replay evidence, plus export of automatic error captures. These are diagnostic captures, not a promise that every frozen scene can resume.
- Game-save reset with a local backup. Reset also clears that game's quick/startup slots and disables automatic startup loading.

Quick Save/Load uses the native replay/validation mechanism. Load reconstructs the recorded execution and may take time. The current runtime is retained until reconstruction succeeds; an unrecoverable transport timeout ends the process to avoid mixing command responses.

Each launch has a separate native process and temporary storage directory, addressed by an unguessable session capability. No list/read API exposes another session. The same browser cannot open the same archive concurrently (Web Locks); different browsers and games are independent. Hidden tabs pause the guest. Storage export is acknowledged at a guest-worker command boundary without changing pause state, held keys, timer semantics, or Quick Save slots.

## Local storage and downloads

Durable progress, quick/recovery/startup checkpoints and per-game settings are stored in this browser origin/profile's **IndexedDB `gomul_browser_saves`**, keyed by the source archive's SHA-256. They are restored into a new private temporary runtime on launch. Nothing is implicitly read from the former shared Mac save directory. Its existing contents are left untouched.

Progress synchronizes every 15 seconds, after save/load/settings/reset actions, when a tab becomes hidden (best effort), and before returning to the library. A write is considered successful only after its IndexedDB transaction commits. Abrupt browser/device termination can lose changes since the last synchronization. Clearing site data or changing browser/profile/origin does not preserve these saves. Private browsing may discard storage. This is browser-profile isolation, not separate login accounts within the same browser profile.

The native worker snapshots storage at a command boundary before ZIP transport. Bundles are limited to this archive's saves, known checkpoint slots, and validated settings; no game archives, arbitrary server paths, rescue files or old Mac saves are imported. ZIP traversal, links, unrelated identities and excessive sizes are rejected. Runtime directories are deleted on stop or idle expiry (five minutes); up to eight sessions run concurrently. An unclean OS/process termination may leave temporary directories for system cleanup, but they are not reused for later launches. Active emulation still requires temporary server-side copies; this is not browser-side WASM execution.

Catalog/download cache: `$XDG_CACHE_HOME/gomul-web`, or **`~/.cache/gomul-web`**. `python3 tools/web-catalog/server.py --cache PATH --port 8790` selects another cache. Cache deletion does not delete browser saves. Downloaded ROMs, thumbnails and signed CDN URLs remain outside the repository.

The source site's category pages supply the index; ROMs download only when selected. ZIP/JAR and ALZ attachments are accepted. ALZ members are decoded through bounded `unar` stdout reads without extracting archive-controlled host paths; install `unar` on the server (for example `brew install unar` on macOS). Single-folder KTF/LGT bundles and ZIPs with prepended data are canonicalized deterministically, preserving stable checkpoint archive identities across sessions. Interrupted or invalid ZIP downloads are not retained as valid cache hits. The catalog snapshot lasts 24 hours across server restarts. Attachment metadata lasts one hour; file selection and cached ROM requests reuse it. An expired attachment link is refreshed once while preserving the selected filename. Thumbnails use a separate bounded request pool, at most two browser requests, and a one-day browser cache.

The server binds IPv4 loopback and validates Host/Origin headers. Download targets come from catalog posts and are restricted to the source site and its HTTPS CDNs, including redirects. Requests have size/time bounds. The native transport uses private subprocess pipes and session tokens; it accepts no browser-provided local paths or arbitrary download URLs.

This remains a loopback-only integration. Session/save isolation is implemented; public deployment still requires an HTTPS hosting design, access/abuse controls and appropriate process sandboxing. It is not a public ROM mirror. A catalog entry is not a compatibility claim. No release was published. The presence of archives on the source site does not establish permission to redistribute them.

## Validation

```sh
python3 -m unittest discover -s tools/web-catalog -v
node --test wie-web/tests/*.test.mjs
npx tsc --noEmit -p wie-web/tsconfig.json
GOMUL_CHECKPOINT_BUILD=local-web-test cargo test -p wie-android --features local-web
```

The native integration passed a real KTF 미궁-미술관 살인사건 smoke test: boot, quick save/load, undo-load, protected startup save/relaunch, independent quick-slot preservation, manual rescue ZIP with screenshot, and save reset with backups. Test data was isolated from player saves. Automated tests cover archive normalization/path rejection, companion-data preservation and identity setup, screenshot channels, stale sessions and per-game settings, alongside existing catalog/input/checkpoint tests. Seventeen Python tests, nineteen browser catalog/input tests and twenty-three native adapter tests passed, together with TypeScript checking and production builds. Browser checks confirmed the title screen, toolbar save/load, Korean keypad mode and settings layout without console errors. A separate native check imported Super Action Hero 3 companion data and reached an active rendering state; it was not a full authentication/gameplay test. These checks are not full game playthroughs.

## Earlier download latency diagnosis

The original server re-read the source page for both file-list and ROM requests even when the ROM was cached. A cached 1,907,988-byte Reicarna archive took 8.832 + 10.254 seconds and 9.722 + 9.424 seconds in two measured request pairs. After attachment metadata caching, the same paired localhost API calls took about 7.3, 2.2 and 2.0 milliseconds in three rounds. These are warmed API measurements, not first-time CDN transfer or emulator startup measurements. The native catalog also avoids downloading the ROM into the browser before launching the local engine.

The cover-library update groups the live 310 carrier entries into 286 distinct normalized titles. Twenty-six catalog/input/platform tests passed. Browser checks covered the NOM3 KTF/LGT chooser, cancellation, sorting and list/grid switching; a 390-pixel mobile viewport showed three cover columns without horizontal overflow.

Selecting a cover shows a loading overlay with its enlarged thumbnail and preparation status; it closes when the player opens or preparation is canceled/fails. Missing thumbnails use a title-letter fallback.
