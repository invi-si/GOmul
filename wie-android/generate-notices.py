"""Reuse the repository's notice collector for the native Android dependency graph."""
from pathlib import Path
import importlib.util
root = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("wie_notices", root / "scripts/generate-rust-notices.py")
notices = importlib.util.module_from_spec(spec)
spec.loader.exec_module(notices)
notices.TARGETS = {"aarch64-linux-android": "wie-android"}
notices.OUTPUT = root / "wie-android/android/app/src/main/assets/LICENSES.txt"
notices.main()
with notices.OUTPUT.open("a") as output:
    output.write("\nNeoDunggeunmo font\n")
    output.write((root / "wie-web/public/licenses/NeoDunggeunmo-OFL.txt").read_text())
