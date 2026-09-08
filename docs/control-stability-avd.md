# Frozen-control stability on the Mac AVD — 2026-09-09

**Outcome:** a repeatable fast regime was restored by explicitly activating the
macOS AVD host before each benchmark process. No CPU candidate, emulator code,
guest timing, refresh cap, CPU affinity, governor, scheduling configuration, or
power setting was changed. macOS's automatic priority response to application
activation was the controlled host variable.

The SHA-256 of the frozen control on both Mac and Android remains:
`1c7dee8deabccb6944de11d63c70d45289be4d07f57723e7ac181a951cbabcfc`.
Both fixture hashes were recorded on-device. All runs used exactly 5,940,941 normal
steps or 9,072,858 skill steps, with unchanged CPU segments and budgets. All emitted
`timing_eligible=true` and passed their CPU/exit and full final-memory validations.

## Procedure and observations

The Android gameplay app was backgrounded using its existing pause behavior,
retaining its session. The only executable benchmarked was the original frozen
control. Each phase alternated normal then skill for six pairs. Each process ran
two warmups and three measured replays: 18 measured results per workload/phase.

| Host condition | Normal median | Skill median | Normal / skill CV |
|---|---:|---:|---:|
| Initial inactive host state | 143.637 ms | 219.925 ms | 3.90% / 3.10% |
| One explicit host activation | 28.227 ms | 41.645 ms | 1.54% / 1.31% |
| Activation before every process | **28.059 ms** | **41.470 ms** | **1.11% / 1.17%** |

These are medians of the six process medians. CV is sample standard deviation /
mean of process medians; it is not a confidence interval or a guarantee about
future runs. The final phase added ten-second gaps between pairs, while activating
the host again before each subprocess.

Final process medians:

- Normal: 27.801, 28.049, 27.936, 28.683, 28.069, 28.277 ms.
- Skill: 40.767, 42.280, 41.482, 41.338, 41.458, 41.532 ms.

macOS `ps -M` initially reported priority 4 for the AVD host threads. Activation
changed the main thread to 46 and other inspected threads to 31. A later inactive
interval returned to priority 4 after about 55 seconds; reactivation restored the
higher priorities. The `T` suffix printed by ps is retained in raw records and is
**not** interpreted as proof of App Nap. An attempted hide invocation reported an
error, so that interval is not presented as a successfully controlled hide test.

The immediate timing recovery and sustained activation series identify host
application activity/priority as a practical way to control this timing regime.
This is consistent with macOS background deprioritization; we did not inspect
private OS policy state and therefore do not assert a particular App Nap mechanism.
Low Power Mode was off, and the Mac remained on battery; no power settings were
changed. No emulator or app restart was required.

An activation request does not guarantee the AVD stays frontmost. In fact, the
recorded frontmost PID during the final phase was Codex, while the AVD retained
normal thread priorities. The procedure relies on the observed activation/priority
state, not on incorrectly equating Launch Services' `Foreground` application type
with actual window focus. Raw host thread state and actual frontmost PID are saved.

## Repeatable tooling

`tools/cpu-bench/avd_host.py` makes a reversible activation/unhide request through
NSRunningApplication, waits 250 ms outside timing, then records thread priorities
and the actual frontmost PID. It neither changes process priority directly nor
writes persistent defaults.

`tools/cpu-bench/control_stability.py` runs **control only** with fixed fixtures,
normal/skill alternation, two warmups and three measured rounds per process. It
records hashes, host activation state, all results, and process-level variation.
Supply the PID of the already-running AVD host:

```sh
python3 tools/cpu-bench/control_stability.py \
  --adb "$ANDROID_HOME/platform-tools/adb" --avd-host-pid AVD_PID \
  --control FROZEN_CONTROL --normal NORMAL_CAPTURE --skill SKILL_CAPTURE \
  --out NEW_LOCAL_OUTPUT_DIRECTORY
```

The permanent paired `transcript_gate.py` now requires `--avd-host-pid` for macOS
AVDs and uses the same host activation before each verification/timing subprocess.
It preserves both-workloads-first verification and the 5%/consistency screen.
This follow-up did not run any candidate through that gate.

Raw logs and manifests are private under
`release-artifacts/cpu-transcript/control-stability/`. Do not publish the fixtures
or machine metadata. All three phases and every measured result are retained;
no slow samples were removed to improve reported consistency.

## Conclusion and limits

The frozen control was correct and unchanged. Mixing macOS host activity regimes
made small optimization comparisons unreliable. We now have a reproducible
procedure returning the original fast baseline with about 1.1% process-level
variation in this session. Continue checking local stationarity in future runs;
activation is not a substitute for examining variance.

No gameplay FPS improvement is claimed. The archived register-indexing candidate
remains rejected for promotion until it passes a new paired run under the now
controlled conditions. No further manual scene positioning is needed.
