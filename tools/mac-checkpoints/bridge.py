#!/usr/bin/env python3
"""Authenticated loopback bridge for the Mac AVD's in-game checkpoint buttons."""
import argparse
import json
import re
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from checkpoints import Checkpoints


def serve(config):
    def maintain_tunnel():
        while True:
            try:
                controller = Checkpoints(config['adb'], config['avd'], config['stateDirectory'])
                controller.connect()
                port = 'tcp:' + str(config['port'])
                listing = controller.command('-s', controller.serial, 'reverse', '--list')
                if not any(line.split()[-2:] == [port, port] for line in listing.splitlines()):
                    controller.command('-s', controller.serial, 'reverse', port, port)
            except Exception:
                pass  # The configured AVD may be closed; retry when it opens.
            time.sleep(10)
    threading.Thread(target=maintain_tunnel, daemon=True).start()
    results = {}
    lock = threading.Lock()
    busy = False

    def execute(request_id, action, game):
        nonlocal busy
        # Let Android finish its button gesture and receive acceptance before capture.
        time.sleep(0.5)
        try:
            result = Checkpoints(config['adb'], config['avd'], config['stateDirectory'], game=game).perform(action)
        except Exception as error:
            result = {'error': str(error)}
        with lock:
            results[request_id] = result
            busy = False

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def reply(self, code, value):
            payload = json.dumps(value).encode()
            self.send_response(code)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

        def authorized(self):
            if self.headers.get('Authorization') != 'Bearer ' + config['token']:
                self.reply(403, {'error': 'Unauthorized checkpoint request.'})
                return False
            return True

        def do_GET(self):
            if not self.authorized():
                return
            with lock:
                result = results.get(self.path[1:], {'message': 'Checkpoint session restored.'})
            self.reply(200, result)

        def do_POST(self):
            nonlocal busy
            if not self.authorized():
                return
            try:
                length = int(self.headers.get('Content-Length', '0'))
                if not 0 < length <= 1024:
                    raise ValueError()
                data = json.loads(self.rfile.read(length))
                request_id, action, game = data['id'], data['action'], data['game']
                if not isinstance(game, str) or not re.fullmatch(r'[0-9a-f]{64}', game):
                    raise ValueError()
                if action not in ('save', 'load', 'recover', 'boot') or not isinstance(request_id, str) or len(request_id) != 36:
                    raise ValueError()
            except (ValueError, KeyError, TypeError):
                self.reply(400, {'error': 'Invalid checkpoint request.'})
                return
            with lock:
                if request_id in results:
                    result = results[request_id]
                elif busy:
                    result = {'error': 'Another checkpoint operation is running.'}
                else:
                    busy = True
                    result = {'pending': True}
                    results[request_id] = result
                    threading.Thread(target=execute, args=(request_id, action, game), daemon=True).start()
            self.reply(200, result)

    ThreadingHTTPServer(('127.0.0.1', config['port']), Handler).serve_forever()


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--config', required=True)
    serve(json.loads(Path(parser.parse_args().config).read_text()))
