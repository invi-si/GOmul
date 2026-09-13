# Dynamic string precision in dialogue

Wild Frontier displayed literal `%.*s` after the speaker name. The shared
formatter stopped parsing on `.` or `*`, leaving the formatting instruction
visible rather than consuming the precision and string arguments.

`sprintf` and `vsprintf` now support literal/dynamic precision, dynamic width,
negative width (left alignment), and negative dynamic precision (unspecified).
Strings are processed as guest bytes: C precision limits bytes, not Unicode
characters. Bounded strings stop at precision or NUL without reading beyond the
limit. This preserves even partial EUC-KR sequences used by progressive text.
Width padding also counts bytes. Numeric width/precision and following argument
consumption have regression coverage. Existing 4096-byte padding limits remain.

Targeted LGT/WIPI C suites: 92 tests passed. No game data, guest timing, or
per-game runtime branches were changed.
