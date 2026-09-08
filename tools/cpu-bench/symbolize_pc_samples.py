#!/usr/bin/env python3
"""Attribute validated native Android PC samples using their exact debug ELF.

Each sample contributes once to the innermost source frame. Inline parents
remain attached to each PC for inspection, never added as disjoint CPU costs.
"""

import argparse
import collections
import hashlib
import json
from pathlib import Path
import subprocess


def symbol_records(symbolizer, binary, offsets):
    records = {}
    addresses = sorted(offsets)
    for start in range(0, len(addresses), 512):
        output = subprocess.check_output(
            [str(symbolizer), f"--obj={binary}", "--inlines", "--demangle",
             "--output-style=JSON", "--no-debuginfod"]
            + [hex(address) for address in addresses[start:start + 512]],
            text=True,
        )
        for item in json.loads(output):
            records[int(item["Address"], 16)] = item["Symbol"]
    if set(records) != offsets:
        raise ValueError("Symbolizer did not return exactly the requested addresses")
    return records


def read_capture(path):
    capture = json.loads(path.read_text())
    if capture.get("formatVersion") != 2 or capture.get("contextPcOffset") != 440:
        raise ValueError(f"{path}: unsupported or invalid signal-context ABI; version 2/offset 440 required")
    if capture.get("validated") is not True or capture.get("timerAndHandlerRestored") is not True:
        raise ValueError(f"{path}: architectural validation or signal restoration failed")
    if capture.get("invalidContexts", 0) or capture.get("droppedSamples", 0):
        raise ValueError(f"{path}: invalid contexts or dropped samples prevent clean attribution")
    if sum(sample["count"] for sample in capture["samples"]) != capture["recordedSamples"]:
        raise ValueError(f"{path}: grouped sample total does not match recordedSamples")
    for sample in capture["samples"]:
        if sample["count"] <= 0:
            raise ValueError(f"{path}: nonpositive sample count")
        if sample["inExecutable"]:
            base = int(capture["executableBase"], 16)
            if int(sample["moduleBase"], 16) != base or int(sample["pc"], 16) - base != int(sample["offset"], 16):
                raise ValueError(f"{path}: inconsistent ASLR-relative PC")
    return capture


def summarize(capture, records):
    executable = sum(sample["count"] for sample in capture["samples"] if sample["inExecutable"])
    function_counts = collections.Counter()
    source_counts = collections.Counter()
    outside_counts = collections.Counter()
    details = []
    for sample in capture["samples"]:
        row = dict(sample)
        if sample["inExecutable"]:
            frames = records[int(sample["offset"], 16)]
            frame = frames[0] if frames else {}
            function = frame.get("FunctionName") or "<unsymbolized>"
            source = (frame.get("FileName") or "<unknown>", frame.get("Line", 0), function)
            function_counts[function] += sample["count"]
            source_counts[source] += sample["count"]
            row["inlineFramesInnermostFirst"] = frames
        else:
            outside_counts[sample.get("module") or "<unknown>"] += sample["count"]
        details.append(row)

    def fraction(count):
        return {
            "samples": count,
            "percentOfExecutableSamples": 100 * count / executable if executable else None,
            "percentOfAllSamples": 100 * count / capture["recordedSamples"] if capture["recordedSamples"] else None,
        }

    result = {key: value for key, value in capture.items() if key != "samples"}
    result.update({
        "executableSamples": executable,
        "nonExecutableSamples": capture["recordedSamples"] - executable,
        "mipsWall": capture["guestInstructions"] / capture["elapsedMs"] / 1000,
        "mipsCpu": capture["guestInstructions"] / capture["elapsedCpuNs"] * 1000,
        "deliveredPerCpuSecond": capture["deliveredSignals"] / capture["elapsedCpuNs"] * 1e9,
        "innermostFunctions": [dict(function=function, **fraction(count)) for function, count in function_counts.most_common()],
        "innermostSourceLines": [dict(file=key[0], line=key[1], function=key[2], **fraction(count)) for key, count in source_counts.most_common()],
        "nonExecutableModules": [{"module": module, "samples": count} for module, count in outside_counts.most_common()],
        "samples": sorted(details, key=lambda row: row["count"], reverse=True),
    })
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--symbolizer", required=True, type=Path)
    parser.add_argument("--expected-sha256", required=True, help="Hash of the exact executable verified on the device")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("captures", type=Path, nargs="+")
    args = parser.parse_args()
    binary = args.binary.resolve()
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    if digest != args.expected_sha256.lower():
        parser.error("Local binary hash differs from the verified device binary")
    captures = [read_capture(path) for path in args.captures]
    offsets = {int(row["offset"], 16) for capture in captures for row in capture["samples"] if row["inExecutable"]}
    anchors = {int(capture["anchorOffset"], 16) for capture in captures}
    records = symbol_records(args.symbolizer, binary, offsets | anchors)
    for anchor in anchors:
        if not any("android::run_measured" in frame.get("FunctionName", "") for frame in records[anchor]):
            raise ValueError(f"Anchor {anchor:#x} does not identify this sampler's run_measured function")
    output = {
        "formatVersion": 1,
        "scope": "Native synthetic guest workloads; not a whole-game or WebAssembly CPU profile",
        "attribution": "One count per sampled PC using its innermost inline frame; inline parents are inclusive context only",
        "binary": str(binary), "binarySha256": digest,
        "symbolizer": str(args.symbolizer.resolve()),
        "captures": [dict(input=str(path.resolve()), **summarize(capture, records)) for path, capture in zip(args.captures, captures)],
    }
    args.output.write_text(json.dumps(output, indent=2) + "\n")
    print(f"Wrote {args.output}: {len(captures)} captures, {len(offsets)} executable PC offsets")


if __name__ == "__main__":
    main()
