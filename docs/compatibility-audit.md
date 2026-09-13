# Automated compatibility audit

`tools/compatibility/audit.py` inventories the installed Android library, scans
nested game archives, runs bounded boot/input smoke sequences, and batches the
existing Rescue replay runner. All output is private local evidence; keep it
outside the repository. Games, saves, logs and recordings must not be committed.

Use Python 3.11+ for scanning (standard-library `tomllib`) and an explicitly
selected ADB device. Build the Android test APK with
`:app:assembleDebugAndroidTest`. Boot audits require a native APK built with the
`compatibility-audit` feature. This writes the Rust warnings/errors to
`saves/case.frames.log` outside the replayed save tree; ordinary Android logcat alone cannot catch guest-thread failures.
The diagnostic build is for compatibility evidence, not performance measurement.

```sh
# With the normal Android/Rust build environment configured:
GOMUL_COMPATIBILITY_AUDIT=1 sh wie-android/build.sh
# Install the generated app APK and Android test APK before running the audit.
python3 tools/compatibility/audit.py --serial emulator-5554 --output /private/audit --phase scan
python3 tools/compatibility/audit.py --serial emulator-5554 --output /private/audit --phase boot
python3 tools/compatibility/audit.py --serial emulator-5554 --output /private/audit --phase rescue
python3 -m unittest discover -s tools/compatibility -v
```

Use `--game-id <archive-id>` to rerun one installed game or its rescue reports.
Use a new output directory for each APK revision. Interrupted boot sweeps resume
completed cases in the same directory. Never reuse one directory for two builds.
The boot harness force-stops the app between cases, uses disposable cache saves,
waits five seconds, sends `OK,DOWN,OK,1`, and settles for two seconds. Normal saves,
Quick Save, game archives and exported rescues are not used or modified. It ends
at the library. Do not interact with the game during the sweep.

The archive scan lists class/descriptor strings and pointer-backed native method
metadata candidates, compared with the checked-in LGT slot table. KTF `client.bin*`
files use their own encoded descriptor/name format and are compared with source
method declarations (including abstract interfaces), without borrowing LGT slot
assignments. This source inventory does not cover external runtime dependencies
or prove that a declared prototype is registered. Malformed nested archives are
reported per member; intact outer packages remain available for boot testing. A matching
name/descriptor does not establish a record's owner or call-site slot. Missing
explicit slot metadata does not establish an unsupported method: static/named
calls and inherited methods can work without that entry. Obfuscated guest
methods also appear in the scan. Never generate runtime mappings from these
candidates automatically. Inspect the call site and establish the contract first.

Each boot case retains phase status/paint counts, native logs, PID-filtered
Android logs and errors. Background log failures override a healthy-looking
paint counter. No paints or an unchanged paint counter means review is needed,
not a proven hang: titles, network dialogs and input gates can be static.
“Smoke-only-no-detected-error” is not gameplay compatibility or tutorial coverage.
Hard process failures/timeouts are separate from native errors. CPU/frame timing,
refresh caps and guest scheduling are unchanged.

Rescue batches retain the normal build/archive validation. A matching recorded
failure means reproduction succeeded, not that a fix succeeded. Reports without
an installed matching game are skipped explicitly. Old-build mismatch or tape
divergence must never be counted as a passed regression. For a confirmed fix,
batch the existing safe-prefix workflow (requires `compatibility-audit`):

```sh
# Originating/baseline build first; preserve the private oracle directory.
python3 tools/compatibility/audit.py --serial emulator-5554 --output /private/baseline --phase rescue-capture
# After installing the candidate diagnostic APK:
python3 tools/compatibility/audit.py --serial emulator-5554 --output /private/candidate --phase rescue-verify --reference /private/baseline
```

These explicit diagnostic commands allow cross-build comparison; they do not
change normal Quick Load. `prefix-oracle-matched` means exact safe-prefix frame
and saved-data equality followed by a three-second continuation without a native
error. It still does not prove gameplay compatibility; review logged background
errors separately. Longer scene-specific continuation can be tested
as described in [the component report](lgt-native-components-rescue.md). Never silently bypass normal Quick
Load build checks. An old recording may become incompatible with a correctness
fix and require a new recording.

Add each confirmed shared ABI mapping as data with native-SVC or JVM regression
tests. Unknown numeric slots stay unresolved until evidence identifies them.

Boot crashes are also retained as `<archive-id>/rescue.tar` before the next case
clears disposable saves. Pass `--rescue-fixtures /private/boot-baseline` to any
rescue phase to use those captures instead of the ordinary user rescue directory.
For example:

```sh
python3 tools/compatibility/audit.py --serial emulator-5554 --output /private/oracles --phase rescue-capture --rescue-fixtures /private/boot-baseline
python3 tools/compatibility/audit.py --serial emulator-5554 --output /private/fixed --phase rescue-verify --rescue-fixtures /private/boot-baseline --reference /private/oracles
```

Caught guest exceptions also appear in `logged-error-review`; investigate whether
they escape the callback before calling the title broken. Startup failures may
have no usable safe prefix. In that case the rescue replay remains blocked and a
fresh identical boot sequence is the available regression check, not a substitute
reported as a successful rescue replay.

Group a completed or partial sweep into a private Markdown/JSON report:

```sh
python3 tools/compatibility/summarize.py /private/audit/boot.json --output /private/audit/report
```

Groups may overlap; the summary keeps caught exceptions and static screens as
review items, and describes unimplemented stub names as reported labels rather
than verified numeric ABI identities.

For a deeper bounded probe, pass `--keys` with a comma-separated sequence. The
instrumentation uses the same 100 ms hold/400 ms gap per key; the runner records
the sequence and adjusts only its host-side command timeout. Guest timing and
scheduler policy do not change. Use a new output directory for a new sequence.

## Unattended blank-boot triage

The Android compatibility test now records framebuffer fingerprints, near-black
and near-white pixel fractions, PNG evidence, state and paint counts at each
sample. `--boot-samples` selects five-second startup observations. Three settled
samples follow a bounded key sequence. `--capture-rescue` retains a manual
capture in the isolated audit tree; normal player saves and Quick Saves are not
used. The native compatibility-audit feature remains required for guest-thread
warning/error evidence. This build is not a performance benchmark.

`tools/compatibility/overnight.py` consumes a frozen private plan of selected
installed archives, runs each in a fresh process with disposable saves, retains
per-case results, and retries high-priority suspects once with a longer startup
wait and alternative inputs. Per-game timeouts keep one hang from blocking the
batch. Already incompatible, overridden and user-deprioritized packages are
excluded when generating the plan. The runner restores an explicitly supplied
normal APK at completion or interruption and records restoration failures.

Three settled samples with at least 98% near-black or near-white pixels are
high-priority suspects even if paints increase. Missing framebuffers and
process/harness failures are separate high-priority review reasons. Native/logged
errors are lower priority under the user's triage policy. A static nonblank
screen is only a review item, not a proven crash; caught exceptions and intentional
first-run exits also require interpretation. No screen-based classification
establishes full gameplay compatibility or a proven infinite loop.

The runner writes `status.json`, `results.json`, `REPORT.md`, per-case logs,
framebuffer PNGs and available rescue archives to its private output directory.
Resume only after verifying the old process is no longer live and the same audit
APK is installed; reuse completed cases only with the same plan/build. A macOS
`caffeinate -i` wrapper can prevent idle sleep during the batch; the Mac must stay
powered and its lid open. All evidence and game-derived captures stay local.

The boot runner validates native key names before dispatch. LSOFT/RSOFT aliases
are normalized to L/R; unknown names now fail instead of silently doing nothing.
Earlier captures containing RSOFT did not exercise the native right soft key.
Preserve their recorded sequence and use a new directory for revised probes.

For the diagnostic RescueInstrumentation fresh-start/continuation path, keys
may also contain WAIT:milliseconds (0–30000 per step). These are host-side waits
between inputs, not changes to guest timers or scheduling. The boot CLI's keys
remain button-only. Use explicit waits to navigate documented first-run setup
without racing logos or dialogs, and retain the complete sequence with evidence.
