"""Reuse the repository's notice collector for the native Android dependency graph."""
from pathlib import Path
import importlib.util
import subprocess
root = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("wie_notices", root / "scripts/generate-rust-notices.py")
notices = importlib.util.module_from_spec(spec)
spec.loader.exec_module(notices)
notices.TARGETS = {"aarch64-linux-android": "wie-android"}
notices.OUTPUT = root / "wie-android/android/app/src/main/assets/LICENSES.txt"
notices.main()
with notices.OUTPUT.open("a") as output:
    output.write("\nGOmul provenance (offline build metadata; not telemetry)\n")
    output.write((root / "PROVENANCE.json").read_text())
    output.write("\nNeoDunggeunmo font\n")
    output.write((root / "wie-web/public/licenses/NeoDunggeunmo-OFL.txt").read_text())
provenance = subprocess.check_output(["python3", str(root / "tools/provenance/provenance.py"), "manifest"], cwd=root)
notices.OUTPUT.with_name("GOMUL-PROVENANCE.json").write_bytes(provenance)
