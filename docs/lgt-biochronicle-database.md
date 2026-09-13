# LGT database open-or-create

Bio Chronicle booted into a save-failure dialog drawn over its ordinary
notice. The native trace opened `SAVsetup`, `SAVchara`, and `SAVdata` with
mode 8/type 1, queried three-word record metadata, then used the third word
as the stream length. The previous implementation returned a handle without
creating a backing record. Metadata lookup failed, leaving stack values
where the game expected a length.

The caller at 0xf248 expects an openable empty stream on first use; the
write path at 0x33d20 treats an open failure as a save failure. Returning
not-found was tested and rejected. Mode 8 now creates record 1 with zero
bytes only when no database/resource exists. Reopening preserves data.
Other modes retain their previous behavior. No game-name checks or patched
game files are used.

Regression test: first open exposes size zero, metadata stays within its
output boundary, write/close/reopen preserves both size and bytes.
Android validation: the game created its 116-byte setup record, the error
and overlapping notice disappeared, and OK advanced into the opening
story sequence. Full gameplay is still subject to user testing.
