import http.client
import json
import tempfile
import threading
import unittest
from pathlib import Path
from unittest.mock import Mock, patch
from http.server import ThreadingHTTPServer
from server import Handler, Catalog, EXCLUDED_GAMES


class HttpTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)/'dist'
        self.root.mkdir()
        (self.root/'catalog.html').write_text('catalog')
        (self.root/'assets').mkdir()
        (self.root/'assets/.private').write_text('private')
        outside = Path(self.temp.name)/'secret'; outside.write_text('outside')
        (self.root/'outside.txt').symlink_to(outside)
        self.server = ThreadingHTTPServer(('127.0.0.1',0), lambda *a,**kw:Handler(*a,directory=str(self.root),**kw))
        self.host = '127.0.0.1:' + str(self.server.server_port)
        self.server.hosts = {self.host}
        self.server.native = Mock()
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()

    def tearDown(self):
        self.server.shutdown();self.thread.join();self.server.server_close();self.temp.cleanup()

    def request(self, method, path='/', headers=None, body=None):
        connection = http.client.HTTPConnection(self.host, timeout=2)
        connection.request(method,path,body,headers or {})
        response = connection.getresponse()
        result = (response.status,dict(response.getheaders()),response.read())
        connection.close()
        return result

    def test_get_and_head_require_trusted_origin_and_host(self):
        for method in ('GET','HEAD'):
            self.assertEqual(self.request(method,headers={'Host':'evil.test'})[0],403)
            self.assertEqual(self.request(method,headers={'Origin':'https://evil.test'})[0],403)
            status,headers,_ = self.request(method)
            self.assertEqual(status,200)
            self.assertEqual(headers['X-Frame-Options'],'DENY')
            self.assertEqual(headers['X-Content-Type-Options'],'nosniff')

    def test_no_directory_listings_hidden_files_or_outside_symlinks(self):
        for method in ('GET','HEAD'):
            for path in ('/assets/','/outside.txt','/assets/%2eprivate','/%2e%2e/secret'):
                self.assertEqual(self.request(method,path)[0],404,(method,path))

    def test_rejects_invalid_post_before_native_dispatch(self):
        self.assertEqual(self.request('POST','/api/native/start',{'Content-Type':'application/json'},'[]')[0],400)
        self.assertEqual(self.request('POST','/api/native/start',{'Content-Type':'text/plain'},'{}')[0],415)
        self.assertEqual(self.request('POST','/api/native/start',{'Content-Type':'application/json','Origin':'https://evil.test'},'{}')[0],403)
        self.server.native.launch.assert_not_called()

    def test_corrupt_catalog_cache_is_refetched(self):
        cache=Path(self.temp.name)/'cache';cache.mkdir()
        (cache/'catalog.json').write_text('[broken')
        body=b'<div class="post-item"><a href="/123"><span class="title">[LGT] Test</span></a></div>'
        with patch('server.fetch',return_value=(body,'text/html')):
            result=Catalog(cache).index()
        self.assertEqual(result[0]['id'],'123')
        self.assertEqual(json.loads((cache/'catalog.json').read_text()),result)
        self.assertFalse((cache/'catalog.part').exists())

    def test_exclusions_apply_to_disk_and_memory_cache_and_game_lookup(self):
        cache=Path(self.temp.name)/'cache';cache.mkdir()
        excluded=[{'id':game_id,'carrier':carrier,'title':'Spelling may change'}
                  for carrier,game_id in sorted(EXCLUDED_GAMES)]
        retained=[{'id':'434','carrier':'LGT','title':'하이브리드2'},
                  {'id':'261','carrier':'KTF','title':'훼미리마트 타이쿤'}]
        (cache/'catalog.json').write_text(json.dumps(excluded+retained))
        catalog=Catalog(cache)
        with patch('server.fetch') as network:
            self.assertEqual(catalog.index(),retained)
            self.assertEqual(catalog.index(),retained)
            # Attachments/download/launch all resolve through game().
            with self.assertRaises(KeyError):catalog.game('258')
            network.assert_not_called()

    def test_exclusions_apply_to_fresh_catalog_without_deleting_raw_cache(self):
        cache=Path(self.temp.name)/'cache'
        body=('<div class="post-item"><a href="/261"><span class="title">[LGT] 훼미리마트 타이쿤</span></a></div>'
              '<div class="post-item"><a href="/434"><span class="title">[LGT] 하이브리드2</span></a></div>').encode()
        with patch('server.fetch',return_value=(body,'text/html')):
            result=Catalog(cache).index()
        self.assertEqual([entry['id'] for entry in result],['434'])
        self.assertEqual(len(json.loads((cache/'catalog.json').read_text())),2)
