# Experimental Thumb instruction-class table

The `experimental-thumb-table` feature is disabled by default. It replaces only
the ordered Thumb instruction-class mask search with an immutable 256-entry
table indexed by the fetched halfword's high byte. The original decoder remains
the feature-off implementation. The table is built at compile time from the
same ordered masks, preserving precedence. A separate low-byte check preserves
the exact `0xbf00` NOP; the other `0xbfxx` encodings remain undefined.

Instruction fetches, memory alignment/fault behavior, operand extraction,
execution, flags, mode changes and CPU layout are unchanged. No result is keyed
by a guest address, so host patches and guest code writes require no invalidation.
Long branches still read and validate their second halfword during execution.

The decision to test this change followed measurements on the physical Fold5:
an own-process native AArch64 signal sampler, with all guest profiling compiled
out, attributed 12.8–13.6% of Thumb add/branch samples and 22.1–23.1% of Thumb
load/add/store/branch samples to inline decoder frames. Each workload had two
sampled 300-million-instruction trials and two same-binary controls in ABBA
order. All 17 registers and all 128 KiB of fixture RAM were validated afterward.
The timer delivered about 250 samples per CPU second, despite a requested 1 ms
interval. Sample/control elapsed differences were within run variation; small
negative differences are not a speedup. These are native synthetic samples,
not percentages of WebAssembly game time. See `tools/cpu-bench/PC-SAMPLING.md`
in the surrounding workspace for the diagnostic protocol and ABI checks.

The feature was implemented only after those corrected physical samples were
recorded and the current feature-off frontend was frozen. Tests compare all
65,536 Thumb encodings against the ordered reference, and the existing ARM and
Thumb execution fixtures run with the feature both disabled and enabled. The
feature may coexist with optional profiling, allowing architectural-equivalence
checks without enabling profiling in performance comparisons.

Retaining or enabling this experiment requires matched, warmed, feature-off
versus table measurements in the physical Android WebView and the actual game.
Native sample fractions alone do not establish a WebAssembly speedup.
