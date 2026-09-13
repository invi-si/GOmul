# Shared native Korean input

The Android keypad labels were previously connected only to the WIPI Java LWC editor. A native name-entry screen continued to receive numeric keys because the native input-method service returned zero without writing either output buffer.

A private replay and caller inspection established the KTF Interface3 layout: slot 0 is the six-argument input handler, followed by set mode, get mode, mode count and mode table. Key presses use event 2; the LGT bridge uses 502. Both KTF table forms and the existing LGT services now route to the same WIPI-C input implementation. There are no archive-name, game-name or guest-PC conditions.

The public contract is documented in the [WIPI input-method specification transcription](https://github.com/mirusu400/wipi-wiki/blob/main/src/content/docs/hal/input-method.md) and [C graphics/input API](https://github.com/mirusu400/wipi-wiki/blob/main/src/content/docs/c-api/graphics.md): mode discovery returns language strings, and input produces separate committed and composing buffers. Native Korean output uses EUC-KR, matching the existing native string APIs.

EN/L and EN/S retain indices 0 and 1; KO and N123 are appended. The Hangul automaton is shared with the Java editor. Native composition uses explicit flush (-99); the Korean keypad also provides # to commit and * for space. Each host mode selection applies at the next native input call. Guest mode selection remains available through the API.

Mode, pending tokens and alphabetic composition live in an allocated guest structure referenced by a guest static field. Replay reconstructs it; no host composition registry is used. Buffer capacities are checked before committing state; key releases do not duplicate text. This enables games using these native input interfaces. It cannot automatically replace a game's wholly private input engine that never calls them.

Tests exercise streaming Hangul, syllable commit, component deletion, capacity rejection without state consumption, guest-state restoration, 10,000 deterministic key presses, and the existing LGT six-argument ARM SVC path for English input.

Validation on the Mac Android environment: the captured name-entry screen displays 한글, remains Running, and passes a checkpoint round trip with unfinished composition. The KTF/LGT/MIDP/WIPI-C/WIPI-Java suites passed 241 tests. The native set-mode result now follows the documented boolean success contract; the LGT SVC regression was rerun after that correction. No original game archives or normal saves were modified.
