# Local web review — 2026-09-13

The reviewed build is running at `http://localhost:8790`. It is suitable for continued local testing, but the current architecture is not ready for public multi-user hosting. No publication, commit, or push was performed during this review.

## Fixes

- Native RPC now applies one deadline to the entire response. Previously a partial response could pass the initial readiness check and then block indefinitely in `readline()`. Fragmented replies are assembled with a 64 MiB limit; malformed, oversized, or timed-out responses close the runtime instead of allowing subsequent commands to consume stale responses. Complete command errors preserve the running process. Transport failure during startup loading is propagated instead of reporting a successful launch.
- Static GET and HEAD requests share Host/Origin and file containment checks. Directory listings, dotfiles, traversal, and symlinks outside the built website are rejected. Responses include framing, MIME-sniffing, base-URL, and referrer protections. These are targeted protections, not a claim of a comprehensive security audit.
- Non-object JSON command bodies are rejected before native dispatch. A damaged catalog cache is refetched, and catalog cache replacement is atomic.
- The browser applies visibility state when a game finishes launching in an already-hidden tab and on page restoration. An inactive session cannot resume through a later visibility event.
- `webpack-merge`, imported by the build configuration, is declared directly instead of relying on its presence as a transitive dependency.

No CPU execution, guest timer semantics, game pacing, APK, or native binary changes were made in this review.

## Verification

| Check | Result |
| --- | --- |
| Python catalog/native/HTTP suite | 25 passed |
| JavaScript catalog/platform/input suite | 26 passed |
| TypeScript type checking | Passed |
| Production browser bundle | Passed; existing WASM reused, three bundle-size warnings remain |
| Fresh temporary dependency installation | `npm ci --ignore-scripts --no-audit --no-fund` passed; 360 packages |
| npm dependency audit | Zero reported vulnerabilities at review time |
| Release source/history audit | 11,509 files/history blobs checked, zero findings |
| Whitespace check | `git diff --check` passed |

An isolated native smoke test using KTF 미궁 booted and rendered, then exercised Quick Save, Quick Load, undo-load recovery, a distinct protected startup slot, automatic startup reload, and rescue ZIP export including a screenshot and diagnostic evidence. Temporary test data was removed; existing user saves were not reset.

After the server restart, the live browser loaded the catalog and displayed the combined 놈3 carrier chooser. A game session subsequently reached the player. This is a smoke check, not a full gameplay or all-browser compatibility certification. JavaScript test results are automated module tests, not 26 browser playthroughs. The clean installation check does not represent a fresh OS installation or a full native/WASM toolchain rebuild.

## Before public hosting

The Python server owns **one native emulator process**. Starting another game replaces that process. Saves are separated by game archive identity, **not by website user**. Binding to loopback and checking local request origins limits exposure; it does not create a multi-user service. Mobile/Windows control layouts do not remove the requirement for a reachable local native server.

Choose a deployment architecture before exposing this server publicly:

1. Run emulation and private save storage in each user's browser, bringing the browser runtime to the required feature parity; or
2. Provide authenticated, isolated emulator workers and private persistent storage per user, with session ownership and resource limits.

The catalog's dependence on an external site and signed attachment URLs remains an availability limitation. Game/thumbnail redistribution permissions were not established by this code review. ROMs, downloaded thumbnails, saves, and private checkpoints remain outside the source repository.

Build and local operation details: [web-catalog-local.md](web-catalog-local.md).

## Follow-up: browser-owned saves and catalog policy

The shared-session/save blocker above was subsequently addressed for the local native website: each launch now has a temporary capability-scoped worker, and durable per-game save bundles reside in IndexedDB in the originating browser profile. Same-game tabs use Web Locks to prevent overwrite conflicts. Old shared Mac saves are not automatically imported. See [current storage and deployment details](web-catalog-local.md).

The catalog also excludes recorded incompatible carrier versions and all titles containing 맞고 or 올림픽. These latter exclusions are a user support policy, not newly discovered emulator failures. Desktop on-screen game controls are hidden on Mac/Linux as well as Windows; the toolbar remains available.

Follow-up validation: 34 Python tests, 29 JavaScript tests, and the new native storage snapshot test passed. TypeScript checking and production native/browser builds passed. A real KTF 미궁 session exported quick/startup slots; a simultaneous clean session had neither; a third fresh runtime restored the exported bundle and passed startup loading, Quick Load, undo-load and rescue export. Temporary directories were removed on stop. Browser inspection confirmed desktop controls are hidden. The live catalog has 272 versions, with zero 맞고/올림픽 title matches. No APK was rebuilt or release published.
