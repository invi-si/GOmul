import json
import threading
import time
import unittest
import urllib.request
import urllib.error
from http.server import ThreadingHTTPServer
from unittest.mock import patch
import bridge


class BridgeTests(unittest.TestCase):
    def test_auth_validation_deduplication_and_completion(self):
        servers=[]
        ready=threading.Event()
        def server(address, handler):
            instance=ThreadingHTTPServer(('127.0.0.1',0),handler)
            servers.append(instance);ready.set()
            return instance
        with patch.object(bridge,'ThreadingHTTPServer',side_effect=server), patch.object(bridge,'Checkpoints') as checkpoints:
            checkpoints.return_value.perform.return_value={'message':'Saved'}
            threading.Thread(target=bridge.serve,args=({'token':'test','port':0,'adb':'unused','avd':'test','stateDirectory':'unused'},),daemon=True).start()
            self.assertTrue(ready.wait(2))
            host='http://127.0.0.1:'+str(servers[0].server_port)+'/'
            def request(body=None,token='test',path=''):
                req=urllib.request.Request(host+path,data=None if body is None else json.dumps(body).encode(),headers={'Authorization':'Bearer '+token})
                with urllib.request.urlopen(req,timeout=2) as response:return json.load(response)
            try:
                with self.assertRaises(urllib.error.HTTPError) as error:request({},token='wrong')
                self.assertEqual(error.exception.code,403)
                with self.assertRaises(urllib.error.HTTPError):request({'id':'x','action':'delete'})
                job={'id':'12345678-1234-1234-1234-123456789abc','action':'save','game':'a'*64}
                self.assertTrue(request(job)['pending'])
                self.assertTrue(request(job)['pending'])
                self.assertIn('error',request(dict(job,id='22345678-1234-1234-1234-123456789abc')))
                result={}
                for _ in range(30):
                    time.sleep(.05);result=request(path=job['id'])
                    if not result.get('pending'):break
                self.assertEqual(result,{'message':'Saved'})
                self.assertEqual(request(job),result)
                checkpoints.return_value.perform.assert_called_once_with('save')
            finally:servers[0].shutdown();servers[0].server_close()
