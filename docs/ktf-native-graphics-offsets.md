# Native graphics destination offsets

Action Hero 3D's difficulty menu stacked its title, choices, backgrounds and
cursor at the top-left while its frame was centered. The user's manual rescue
reproduced the defect. Drawing-call logs show repeated graphics-context offset
changes followed by local-coordinate fills, image blits and strings. The shared
WIPI-C renderer stored offsets but did not apply them to these operations.

The renderer now applies signed 16-bit destination offsets to pixels, filled and
outlined rectangles/arcs, lines, image blits and text. Image source coordinates
and clip coordinates retain their existing meaning; text keeps its baseline
convention. Zero offsets preserve the prior coordinates. This is a shared API
fix with no game-name branches, asset modifications or timing changes. The LGT
native adapter uses its own translation path, which was not modified.

The original rescue now renders a centered difficulty panel, separate Normal and
locked choices, and an aligned cursor. Tests compare translated drawing with
explicit destination coordinates for positive, negative and zero offsets, and
check direct/indirect image handles with clipping and fixed source coordinates.
Both new tests fail when translation is disabled and pass with the fix.

Validation: 218 related KTF/LGT/WIPI C/WIPI Java tests passed. Clippy completed
with existing warnings. Samguk, Dragon Road and DarkSlayer2 rescue frame/save
oracles still match and their continuations report no error. The deliberately
changed Action Hero menu is visually compared against the original rescue rather
than incorrectly requiring its broken frame to remain identical.

Private evidence includes the original rescue, drawing trace, fixed capture,
test logs and APK verification. No rescue or game data is included in this source
document. Longer gameplay and unrelated graphics API gaps remain outside this fix.

The normal APK passed a fresh Action Hero 3D boot/input smoke test and its installed
bytes match the delivery artifact. Temporary graphics logging was removed.

APK SHA-256: `c5d270227f63d059a06847c89b53199342d6de317c8b64e921329fbcb1f5c84a`.
