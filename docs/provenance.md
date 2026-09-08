# GOmul provenance and attribution

GOmul is MIT-licensed work based on WIE. These identifiers support honest
attribution and comparison; they impose no new license restrictions and do not
claim ownership of upstream contributions. Closed-source reuse is permitted by
MIT subject to its terms, including preserving applicable copyright/permission
notices. See LICENSE and https://opensource.org/license/mit.

## Independent evidence layers

1. **Copyright and license:** the original WIE notice remains; an additional
   notice identifies invi-si's GOmul contributions only. Selected GOmul component
   files carry scoped notices and MIT SPDX identifiers.
2. **Project and component identifiers:** PROVENANCE.json records public random
   IDs and their source locations. Component IDs are also in comments in the
   timer fix, transcript engine, attribution, input diagnostics, launcher and
   research index. They are descriptive metadata, never guest instructions/data.
3. **Native binary identifier:** the Android library exports GOMUL_ORIGIN_ID as
   an immutable byte array containing the project ID. It has no execution path,
   network operation, allocation or guest-state interaction. It can survive source
   comment removal, but deliberate relinking/editing can still remove it.
4. **Packaged metadata:** Android notice generation embeds the registry in the
   existing Licences page and writes assets/GOMUL-PROVENANCE.json. That manifest
   records source commit, tracked-file modification status, every tracked file's
   SHA-256 and a canonical aggregate hash. It never reads saves or untracked files
   and excludes absolute paths, user names, devices, clock times and credentials.
5. **Public history:** GitHub commits retain publication history. Component changes,
   regression tests and dated reports can be compared independently of IDs. Commit
   author timestamps alone are not trusted third-party timestamp evidence.
6. **Release identity:** release SHA256SUMS and GitHub asset digests identify exact
   APK bytes. Android signing-certificate fingerprints establish continuity of a
   signing key, not sole authorship of every file. Retain hashes before replacing
   any release asset.

New markers enter binaries only when rebuilt. The version-code-7 APK published
before this provenance change does not contain the new IDs. Its evidence record:

- Source: aca38494 (see the release notes for the full commit).
- APK SHA-256: 4a148f8462ed2c8f74e17f8077b3c05ff283735479c42ebffc37893902a8a6c8.
- Android certificate SHA-256: c0a230abf44e737b43104d2df3cd135bb50e912c019b4d19a18637fee2cce537.
- Repository: https://github.com/invi-si/GOmul ; release: v0.1.0.

## Offline commands

From the repository root, stage intended new source before generating a manifest:

```sh
python3 tools/provenance/provenance.py manifest > release-artifacts/source-provenance.json
python3 tools/provenance/provenance.py inspect /path/to/an.apk
python3 tools/provenance/provenance.py inspect /path/to/source-copy
python3 -m unittest discover -s tools/provenance
```

Create the ignored release-artifacts directory first if needed. Inspection reads
files locally; it neither executes nor extracts APK entries. A ZIP entry larger
than 256 MiB is rejected; inspect only reasonable-sized, locally obtained inputs.
Use a clean checkout for release manifests. Hashes describe tracked working bytes,
so a dirty build is explicitly marked and must not be presented as the exact
committed source. Compiler flags, dependency downloads and untracked inputs are
not attested by this source manifest. Native builds still need their feature list
and build settings recorded separately.

## Interpreting matches

A matching ID, unusual test, algorithm structure or substantial code match is a
lead to investigate. Shared WIE ancestry also creates legitimate similarities.
A marker can be copied independently, forged, stripped, or carried through a
lawful fork. Absence does not exclude copying and presence does not establish a
license violation. Preserve the obtained artifact, its hash, URL and acquisition
date before comparing it; do not accuse someone based solely on a marker.

These are transparent identifiers: no telemetry, callback URLs, device IDs,
tracking pixels, executable traps, deliberate bugs, altered guest timing, poisoned
test vectors, hidden save data or invented historical claims. Signed commits,
signed release manifests and hosted build attestations could add authentication
later, but require a deliberate signing/key-management setup; this change does
not pretend that unsigned hashes are signed attestations.
