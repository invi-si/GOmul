import base64
import hashlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch
import zipfile
from browser_sessions import BrowserSessions, restore_bundle, export_bundle
from native import Native, validated_settings
from server import eligible_games


def bundle(files):
    out=io.BytesIO()
    with zipfile.ZipFile(out,'w') as z:
        for name,value in files.items():z.writestr(name,value)
    return base64.b64encode(out.getvalue()).decode()


class BrowserStorageTests(unittest.TestCase):
    def test_imported_settings_are_validated_before_boot(self):
        for values in ([], {'speed':'1000'}, {'width':0}, {'phone':'not a number'}):
            with self.assertRaises(ValueError):validated_settings(values)
        self.assertEqual(validated_settings({})['speed'],1000)

    def test_bundle_rejects_traversal_other_game_and_unrelated_files(self):
        for name in ('../escape','saves/id/../../escape','saves/other/record','games/id/game.jar',
                     '/saves/id/record','saves/id/./record','checkpoints/id/rollback/file'):
            with tempfile.TemporaryDirectory() as root:
                with self.assertRaises(ValueError):restore_bundle(Path(root),'id',bundle({name:b'x'}))
        with tempfile.TemporaryDirectory() as root:
            restore_bundle(Path(root),'id',bundle({'saves/id/db/1':b'earned','checkpoints/id/quick/trace':b'tape','settings/id.json':b'{}'}))
            self.assertEqual((Path(root)/'saves/id/db/1').read_bytes(),b'earned')

    def test_bundle_rejects_links(self):
        out=io.BytesIO()
        with zipfile.ZipFile(out,'w') as z:
            info=zipfile.ZipInfo('saves/id/link');info.external_attr=0o120777<<16;z.writestr(info,'/outside')
        with tempfile.TemporaryDirectory() as root:
            with self.assertRaises(ValueError):restore_bundle(Path(root),'id',base64.b64encode(out.getvalue()).decode())

    def test_export_roundtrip_keeps_slots_and_empty_guest_directories(self):
        with tempfile.TemporaryDirectory() as root,tempfile.TemporaryDirectory() as other:
            native=Native(Mock(),Path(root));native.game='id';native.rpc=Mock()
            for name in ('saves/id/db/1','checkpoints/id/quick/trace','checkpoints/id/startup/trace'):
                path=Path(root)/'browser-export'/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(name.encode())
            (Path(root)/'browser-export/saves/id/empty').mkdir()
            settings=Path(root)/'settings/id.json';settings.parent.mkdir();settings.write_text('{}')
            data=export_bundle(native)
            restore_bundle(Path(other),'id',base64.b64encode(data).decode())
            self.assertTrue((Path(other)/'saves/id/empty').is_dir())
            self.assertTrue((Path(other)/'checkpoints/id/startup/trace').is_file())
            native.rpc.assert_called_once_with({'op':'browser-storage'})

    def test_sessions_have_independent_roots_and_no_implicit_mac_save_import(self):
        catalog=Mock();catalog.rom.return_value=b'game';identity=hashlib.sha256(b'game').hexdigest()
        def launch(native,game,index,expected):
            native.game=expected;native.token=native.root.name
            return {'session':native.token}
        manager=BrowserSessions(catalog)
        payload={'game':'1','index':0,'identity':identity}
        try:
            with patch.object(Native,'launch',launch):
                a=manager.launch(dict(payload,storage=bundle({f'saves/{identity}/progress':b'A'})))['session']
                b=manager.launch(payload)['session']
            first=manager.sessions[a]['native'].root;second=manager.sessions[b]['native'].root
            self.assertNotEqual(first,second)
            self.assertEqual((first/f'saves/{identity}/progress').read_bytes(),b'A')
            self.assertFalse((second/f'saves/{identity}/progress').exists())
            with self.assertRaises(ValueError):manager.command({'session':'unknown','op':'storage'})
            manager.command({'session':a,'op':'stop'})
            self.assertFalse(first.exists());self.assertTrue(second.exists())
            self.assertIn(b,manager.sessions)
        finally:manager.close()
        self.assertFalse(second.exists())

    def test_wrong_archive_and_worker_limit_rejected(self):
        catalog=Mock();catalog.rom.return_value=b'game'
        manager=BrowserSessions(catalog,max_sessions=0)
        try:
            with self.assertRaises(ValueError):manager.launch({'game':'1','index':0,'identity':'wrong'})
            with self.assertRaises(ValueError):manager.launch({'game':'1','index':0,'identity':hashlib.sha256(b'game').hexdigest()})
        finally:manager.close()

    def test_genre_exclusions_apply_across_all_carriers(self):
        entries=[{'id':str(i),'carrier':c,'title':title} for i,c in enumerate(('LGT','KTF','SKT')) for title in ('새 맞고','2010 올림픽','리듬스타2')]
        self.assertEqual([g['title'] for g in eligible_games(entries)],['리듬스타2']*3)
