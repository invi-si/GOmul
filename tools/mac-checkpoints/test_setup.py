import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import unittest
import zipfile
spec=importlib.util.spec_from_file_location('gomul_setup',Path(__file__).with_name('setup.py'))
setup=importlib.util.module_from_spec(spec);spec.loader.exec_module(setup)

class SetupTests(unittest.TestCase):
    def test_user_data_mapping_and_identity(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);game=root/'test.zip';data=root/'data';data.mkdir()
            with zipfile.ZipFile(game,'w') as z:z.writestr('app_info','PID:PDTEST\n');z.writestr('test.jar',b'synthetic fixture')
            (data/'savedata').write_bytes(b'user data')
            game_id,result=setup.build_import(game,data,'01000000000')
            with tarfile.open(fileobj=io.BytesIO(result)) as z:
                self.assertEqual(z.extractfile('files/saves/'+game_id+'/PDTEST/db/savedata/1').read(),b'user data')
                self.assertEqual(z.extractfile('files/saves/'+game_id+'/phone-number.txt').read(),b'01000000000\n')
            with self.assertRaises(ValueError):setup.build_import(game,data,'not a number')
            (data/'nested').mkdir()
            with self.assertRaises(ValueError):setup.build_import(game,data,'01000000000')
