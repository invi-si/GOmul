import io
import zipfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch, Mock

import server

_test_zip=io.BytesIO()
with zipfile.ZipFile(_test_zip,"w") as z:z.writestr("game",b"test")
ZIP_BYTES=_test_zip.getvalue()


class CatalogTests(unittest.TestCase):
    def test_short_http_body_is_rejected_before_caching(self):
        response=Mock()
        response.headers.get.return_value='100'
        response.read.return_value=b'PK\x03\x04short'
        opener=Mock();opener.open.return_value.__enter__=Mock(return_value=response)
        opener.open.return_value.__exit__=Mock(return_value=False)
        with patch.object(server,'build_opener',return_value=opener):
            with self.assertRaisesRegex(ValueError,'중간에 끊겼습니다'):
                server.fetch('https://blog.kakaocdn.net/game.zip')

    def test_corrupt_rom_cache_is_refetched_and_prefixed_zip_is_accepted(self):
        with tempfile.TemporaryDirectory() as tmp:
            catalog=server.Catalog(Path(tmp))
            url='https://blog.kakaocdn.net/game.zip'
            dest=Path(tmp)/(server.hashlib.sha256(url.encode()).hexdigest()+'.rom')
            dest.write_bytes(b'PK\x03\x04truncated')
            valid=b'\xff\xd8image-prefix'+ZIP_BYTES
            with patch.object(server,'fetch',return_value=(valid,'application/octet-stream')) as fetch:
                self.assertEqual(catalog.cached(url,'rom'),valid)
                self.assertEqual(catalog.cached(url,'rom'),valid)
                fetch.assert_called_once()

    def test_index_decodes_titles_and_keeps_carriers_separate(self):
        body = '''<div class="post-item"><a href="/123"><img src="//i1.daumcdn.net/a.jpg"><span class="title">[LGT] 게임 &amp; 2</span></a></div>
        <div class="post-item"><a href="/124"><span class="title">[KTF] 게임</span></a></div>'''
        rows = server.parse_index(body, 'LGT')
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['title'], '게임 & 2')
        self.assertEqual(rows[0]['image'], 'https://i1.daumcdn.net/a.jpg')

    def test_attachments_only_direct_supported_files(self):
        body = '''<a href="https://blog.kakaocdn.net/a/%EA%B2%8C%EC%9E%84.zip?attach=1&amp;token=x">file</a>
        <a href="https://blog.kakaocdn.net/a/%EA%B2%8C%EC%9E%84.zip?attach=1&amp;token=x">duplicate</a>
        <a href="https://evil.test/game.jar">foreign</a><a href="https://blog.kakaocdn.net/a/save.dat">data</a>'''
        files = server.parse_attachments(body)
        self.assertEqual(len(files), 1)
        self.assertEqual(files[0]['name'], '게임.zip')
        self.assertIn('&token=x', files[0]['url'])

    def test_proxy_rejects_arbitrary_hosts_and_credentials(self):
        for url in ['http://127.0.0.1/private', 'https://kakaocdn.net.evil.test/a',
                    'file:///etc/passwd', 'https://user@blog.kakaocdn.net/a',
                    'https://blog.kakaocdn.net:444/a', 'https://evil.test/a']:
            self.assertFalse(server.allowed_url(url), url)
        self.assertTrue(server.allowed_url('https://blog.kakaocdn.net/a.zip'))

    def test_invalid_payload_never_cached_as_rom(self):
        with tempfile.TemporaryDirectory() as tmp:
            catalog = server.Catalog(Path(tmp))
            with patch.object(server, 'fetch', return_value=(b'<html>Error</html>', 'text/html')):
                with self.assertRaises(ValueError):
                    catalog.cached('https://blog.kakaocdn.net/a.zip', 'rom')
            self.assertEqual(list(Path(tmp).iterdir()), [])

    def test_cached_download_does_not_refetch(self):
        with tempfile.TemporaryDirectory() as tmp:
            catalog = server.Catalog(Path(tmp))
            with patch.object(server, 'fetch', return_value=(ZIP_BYTES, 'application/zip')) as fetch:
                self.assertEqual(catalog.cached('https://blog.kakaocdn.net/a.zip', 'rom'), ZIP_BYTES)
                catalog.cached('https://blog.kakaocdn.net/a.zip', 'rom')
                fetch.assert_called_once()

    def catalog_with_game(self, directory):
        catalog = server.Catalog(Path(directory))
        catalog.entries = [{'id': '123', 'carrier': 'KTF', 'title': 'Test', 'source': server.BASE + '/123'}]
        return catalog

    def test_attachment_lookup_reused_across_requests_and_server_restart(self):
        with tempfile.TemporaryDirectory() as tmp:
            body = b'<a href="https://blog.kakaocdn.net/a/game.zip">game</a>'
            catalog = self.catalog_with_game(tmp)
            with patch.object(server, 'fetch', return_value=(body, 'text/html')) as fetch:
                first = catalog.attachments('123')
                self.assertEqual(catalog.attachments('123'), first)
                restarted = self.catalog_with_game(tmp)
                self.assertEqual(restarted.attachments('123'), first)
                fetch.assert_called_once()

    def test_cached_rom_flow_needs_no_remote_requests(self):
        with tempfile.TemporaryDirectory() as tmp:
            catalog = self.catalog_with_game(tmp)
            body = b'<a href="https://blog.kakaocdn.net/a/game.zip">game</a>'
            with patch.object(server, 'fetch', side_effect=[(body, 'text/html'), (ZIP_BYTES, 'application/zip')]) as fetch:
                catalog.attachments('123')
                self.assertEqual(catalog.rom('123', 0), ZIP_BYTES)
                self.assertEqual(fetch.call_count, 2)
            with patch.object(server, 'fetch', side_effect=AssertionError('Unexpected network request')):
                catalog.attachments('123')
                self.assertEqual(catalog.rom('123', 0), ZIP_BYTES)

    def test_expired_link_refresh_preserves_filename_after_reordering(self):
        with tempfile.TemporaryDirectory() as tmp:
            catalog = self.catalog_with_game(tmp)
            files = [{'name': 'game.zip', 'url': 'old'}]
            refreshed = [{'name': 'other.zip', 'url': 'other'}, {'name': 'game.zip', 'url': 'new'}]
            expired = server.HTTPError('old', 403, 'Expired', {}, None)
            with patch.object(catalog, 'attachments', side_effect=[files, refreshed]) as attachments, patch.object(catalog, 'cached', side_effect=[expired, b'ok']) as cached:
                self.assertEqual(catalog.rom('123', 0), b'ok')
                attachments.assert_called_with('123', refresh=True)
                cached.assert_called_with('new', 'rom')

    def test_thumbnail_traffic_uses_separate_request_pool(self):
        self.assertIsNot(server.NETWORK, server.THUMBNAIL_NETWORK)
        with tempfile.TemporaryDirectory() as tmp:
            catalog = server.Catalog(Path(tmp))
            with patch.object(server, 'fetch', return_value=(b'image', 'image/jpeg')) as fetch:
                catalog.cached('https://blog.kakaocdn.net/a.jpg', 'image')
                self.assertTrue(fetch.call_args.kwargs['image'])

    def test_unknown_game_is_not_an_open_proxy(self):
        with tempfile.TemporaryDirectory() as tmp:
            catalog = server.Catalog(Path(tmp))
            catalog.entries = []
            with self.assertRaises(KeyError):
                catalog.attachments('99999')


if __name__ == '__main__':
    unittest.main()
