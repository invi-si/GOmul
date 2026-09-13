#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
GOMUL_CHECKPOINT_BUILD=$(python3 - <<'PY'
import hashlib
from pathlib import Path
h=hashlib.sha256(b'gomul-local-native-v1-thumb-inline-table')
for p in sorted(Path('.').rglob('*.rs')):
 if any(x in p.parts for x in ('target','node_modules','.git')):continue
 h.update(str(p).encode());h.update(p.read_bytes())
h.update(Path('Cargo.lock').read_bytes())
print(h.hexdigest())
PY
)
export GOMUL_CHECKPOINT_BUILD
cargo build --locked --release -p wie-android --bin gomul-local --features local-web,experimental-thumb-inline,experimental-thumb-table
mkdir -p wie-web/native
# Replace the inode atomically: overwriting a running Mach-O can leave macOS
# using the previous code-signature cache and kill the next launch with SIGKILL.
staged=$(mktemp wie-web/native/.gomul-local.XXXXXX)
trap 'rm -f "$staged"' EXIT
cp "${CARGO_TARGET_DIR:-target}/release/gomul-local" "$staged"
chmod 755 "$staged"
mv -f "$staged" wie-web/native/gomul-local
