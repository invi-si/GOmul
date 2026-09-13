# Android thumbnail library

The Android library now uses a responsive, recycled thumbnail grid with Korean title sorting and search. Titles use the same NFKC, lowercase, whitespace-insensitive grouping as the former website. Tapping a shared title offers its carrier versions; long-press retains per-version data import, save reset and deletion. Gameplay controls are unchanged.

Artwork (`thumbnail.jpg`) and optional display metadata (`library.json`, title/carrier/sourceId) are local sidecars beside an imported archive. Missing artwork uses a placeholder. A bounded 8 MB decoded-image cache and sampled decoding limit artwork memory. Games and thumbnails are not bundled into the APK or source repository.

The current test environment restored 246 available supported carrier versions and verified every archive, thumbnail and metadata file. SKT post 447 (어스토니시아 스토리) has no game attachment and could not be restored; its KTF version remains available. Twelve previously excluded legacy library entries were moved into a private retired-library folder, preserving archives and saves. Five existing alternate archives/episodes were retained and given artwork. Existing save/checkpoint directories were not reset or replaced.

Android validation: build succeeds; grid artwork renders; EX search yields a single 드래곤나이트EX tile with KTF/SKT selection; long-press opens the existing import/reset/delete menu. These are library checks, not new gameplay-compatibility claims. The previous encodeImage background-capture fix remains in the installed native library.
