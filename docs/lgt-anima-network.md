# Anima network startup rescue

Rescue 1788999397148 failed at LGT WIPI C import 0x388. The native caller
at 0xed04 passes its IP string, then invokes htons on the adjacent port.
The SDK utility table places MC_utilInetAddrInt after the four endian helpers,
consistent with the existing 0x385 htons mapping.

Implemented dotted-decimal IPv4 conversion to a network-byte-order ARM word.
Malformed strings return 0xffffffff; guest memory errors propagate. This is
local conversion only, with no DNS lookup or network request.

Replay then reached 0x7d0: the caller supplies domain 2 and type 1, tests the
returned descriptor for negativity, and calls the socket-connect path only
on success. This matches the SDK MC_netSocket(domain, type) declaration.
The offline backend now returns -1 and lets the existing guest failure path
run, without allocating a fake descriptor or claiming connection success.

SVC regression tests cover conversion byte order, invalid strings and socket
failure for stream, datagram and invalid inputs. No game-specific branches,
authentication bypass, networking activation, or timer changes were added.

Validation: 92 targeted LGT/WIPI C tests passed. Diagnostic replay of the
rescue safe prefix followed by five seconds of continuation passed with
`Running; paints=280`. The captured frame is the loading screen; this does
not establish full gameplay or successful completion of network-dependent
startup. The installed user APK has rescue-replay disabled.

## Immediate offline failure validation

MC_netConnect now returns -1 synchronously instead of accepting the request
and scheduling a delayed failure callback. A rejected request has no pending
callback. This is shared offline-backend behavior, not an Anima-only rule.

The old rescue diverges when replayed across this behavior change; it cannot
serve as a compatibility oracle for the altered callback sequence. The Android
instrumentation runner now accepts `mode=fresh-start` to boot from an isolated
copy of the report's initial saves without loading its old trace. Optional
`keys` and `settleMs` let it dismiss startup notices and inspect later frames.

112 LGT/KTF/WIPI C tests pass, including immediate failure without a System,
spawn, or guest callback, plus an ARM SVC check of the return value. This tests
the implemented failure contract; broad game compatibility is not established.

Fresh-start Android validation completed the notices and selected Yes at the
network-authentication prompt (keys OK x6, LEFT, OK). After 15 seconds the
captured guest screen read `인증에 성공하였습니다` (authentication succeeded),
with `Running; paints=347`. No real network request or emulator-generated
success response was made: this is the supplied game's own response to the
offline path. The loading loop did not persist in this test. Full gameplay
and unrelated titles remain outside this validation.
