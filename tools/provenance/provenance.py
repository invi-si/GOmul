#!/usr/bin/env python3
"""Offline source manifest generation and exact-marker inspection. No telemetry."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[2]


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT)


def manifest():
    provenance = json.loads((ROOT / "PROVENANCE.json").read_text())
    # Index plus working bytes: deleted files are explicit and untracked/private
    # files are never read. Stage intended new source before generating a build.
    files = {}
    for raw in sorted(set(git("ls-files", "-z").split(b"\0"))):
        if not raw:
            continue
        name = raw.decode()
        path = ROOT / name
        if path.is_symlink():
            raise ValueError(f"Refusing symlink: {name}")
        files[name] = hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else None
    serialized = json.dumps(files, sort_keys=True, separators=(",", ":")).encode()
    return {
        "schema": 1,
        "provenance": provenance,
        "source_commit": git("rev-parse", "HEAD").decode().strip(),
        "tracked_source_modified": bool(git("status", "--porcelain", "--untracked-files=no")),
        "source_tree_sha256": hashlib.sha256(serialized).hexdigest(),
        "files_sha256": files,
        "limits": "Hashes describe tracked working files, not authorship, compiler settings or a signed attestation. Untracked files are excluded.",
    }


def inspect(path):
    provenance = json.loads((ROOT / "PROVENANCE.json").read_text())
    markers = [provenance["project_id"]] + [c["id"] for c in provenance["components"]]
    matches = []

    def scan(name, data):
        found = [marker for marker in markers if marker.encode() in data]
        if found:
            matches.append({"file": name, "markers": found, "sha256": hashlib.sha256(data).hexdigest()})

    if path.is_dir():
        for file in sorted(path.rglob("*")):
            if file.is_file() and not file.is_symlink() and ".git" not in file.relative_to(path).parts:
                scan(file.relative_to(path).as_posix(), file.read_bytes())
    elif zipfile.is_zipfile(path):
        with zipfile.ZipFile(path) as archive:
            for item in archive.infolist():
                # Do not extract or execute any content. Bound decompression.
                if item.file_size > 256 * 1024 * 1024:
                    raise ValueError(f"Entry too large for inspection: {item.filename}")
                if not item.is_dir():
                    scan(item.filename, archive.read(item))
    else:
        scan(path.name, path.read_bytes())
    return {"matches": matches, "interpretation": "A match is a provenance clue, not proof of infringement. Absence does not rule out reuse."}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("manifest")
    checker = sub.add_parser("inspect")
    checker.add_argument("path", type=Path)
    args = parser.parse_args()
    print(json.dumps(manifest() if args.command == "manifest" else inspect(args.path), indent=2))
