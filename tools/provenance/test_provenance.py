import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location("provenance", Path(__file__).with_name("provenance.py"))
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)


class ProvenanceTest(unittest.TestCase):
    def test_source_markers_match_registry(self):
        data = json.loads((p.ROOT / "PROVENANCE.json").read_text())
        self.assertIn(data["project_id"], (p.ROOT / "wie-android/src/lib.rs").read_text())
        for component in data["components"]:
            for file in component["files"]:
                self.assertIn(component["id"], (p.ROOT / file).read_text())

    def test_zip_inspection_does_not_extract(self):
        marker = json.loads((p.ROOT / "PROVENANCE.json").read_text())["project_id"]
        with tempfile.TemporaryDirectory() as tmp:
            archive = Path(tmp) / "fixture.apk"
            with zipfile.ZipFile(archive, "w") as z:
                z.writestr("../escape", marker)
                z.writestr("unrelated", "ordinary content")
            result = p.inspect(archive)
            self.assertEqual(len(result["matches"]), 1)
            self.assertEqual(result["matches"][0]["markers"], [marker])
            self.assertEqual(list(Path(tmp).iterdir()), [archive])


if __name__ == "__main__":
    unittest.main()
