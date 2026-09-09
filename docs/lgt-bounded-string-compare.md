# LGT bounded string comparison and pack-selection crash

LGT standard-library import `0x40a` was registered as an unknown, void-returning stub. It now calls a shared `strncmp` implementation. The implementation compares unsigned guest bytes, stops at a difference or NUL, respects the requested limit, propagates required memory-access faults, and returns a signed comparison encoded in the ARM result register. It does not decode strings as UTF-8.

## Evidence

In Rhythm Star 2, pack selection calls a lookup that opens `skin.dat`, reads its four-byte record count, and scans 132-byte records. Import `0x40a` compares the selected name with each record's 64-byte name field. A zero result selects the record. The old stub left the first pointer in R0, so equal names could never match. The lookup returned NULL; a later guest allocation wrapper subtracted eight bytes and faulted at `0xfffffff8` (reported PC `0x6140`).

The saved database contained three complete records (400 bytes including the header), so missing companion data was not the explanation for this lookup failure. Saves were backed up before testing and were not reset or repaired. The game archive is unchanged.

The call shape, comparison use, and placement immediately after the known `strcmp` import support identifying this slot as `strncmp`. No game identifier checks, memory-fault suppression, CPU changes, or timing changes are included.

## Regression coverage

- Equal bounded prefixes, differences, and early NUL termination.
- Unsigned high bytes, including Korean encoded byte sequences.
- Zero-length comparisons without dereferencing the pointers.
- Exact mapping-boundary termination and required access faults.
- LGT SVC `0x40a` returning equality, negative, and positive results to guest code.

## Validation result

- Core unit suite: 84 passed.
- LGT unit suite: 24 passed, including the import-level regression.
- `cargo fmt` and `cargo clippy --workspace` completed; existing workspace warnings remain.
- Native ARM64 Android debug APK built and installed over the existing app without clearing data.
- On the Mac Android AVD, the existing save now reaches the pack selector with its preview populated and advances to difficulty selection. The prior fatal access did not recur on this path.
- The certificate prompt still reports the unavailable network service; acknowledging that error permits the title screen and menu. This patch does not implement that service.

This validates the reported pack-selection failure, not all gameplay or other LGT games.
