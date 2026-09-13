#!/usr/bin/env python3
"""Serve a private copy of the Android library for client-side WASM development.

No emulator processes, session endpoints, or save uploads are provided.
Game archives stay outside the source tree.
"""
import argparse
import json
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit


def library(directory):
    result = {}
    for folder in sorted(directory.iterdir()):
        if not folder.is_dir():
            continue
        metadata = folder / 'library.json'
        info = json.loads(metadata.read_text()) if metadata.exists() else {}
        archives = sorted(p for p in folder.iterdir() if p.suffix.lower() in ('.jar', '.zip'))
        if not archives:
            continue
        result[folder.name] = {
            'entry': {'id': folder.name, 'title': info.get('title', archives[0].stem),
                      'carrier': info.get('carrier', ''),
                      'source': 'https://dubigame.tistory.com/' + str(info.get('sourceId', '')),
                      'thumbnail': f'/api/game/{folder.name}/thumbnail' if (folder / 'thumbnail.jpg').is_file() else None},
            'archives': archives, 'thumbnail': folder / 'thumbnail.jpg',
        }
    return result


class Handler(SimpleHTTPRequestHandler):
    def send_bytes(self, data, kind):
        self.send_response(200)
        self.send_header('Content-Type', kind)
        self.send_header('Content-Length', str(len(data)))
        self.send_header('Cache-Control', 'no-store')
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):
        self.send_error(405, 'Client-side player: no server sessions or save uploads')

    def do_GET(self):
        path = urlsplit(self.path).path
        if path == '/':
            self.path = '/catalog.html'
            return super().do_GET()
        if path == '/api/catalog':
            return self.send_bytes(json.dumps([g['entry'] for g in self.server.games.values()]).encode(), 'application/json')
        if path.startswith('/api/'):
            parts = path.strip('/').split('/')
            if len(parts) < 4 or parts[:2] != ['api', 'game'] or parts[2] not in self.server.games:
                return self.send_error(404)
            game = self.server.games[parts[2]]
            if parts[3:] == ['files']:
                return self.send_bytes(json.dumps([{'name': p.name, 'index': i} for i, p in enumerate(game['archives'])]).encode(), 'application/json')
            if parts[3:] == ['thumbnail'] and game['thumbnail'].is_file():
                return self.send_bytes(game['thumbnail'].read_bytes(), 'image/jpeg')
            if len(parts) == 5 and parts[3] == 'rom' and parts[4].isdigit():
                index = int(parts[4])
                if index < len(game['archives']):
                    return self.send_bytes(game['archives'][index].read_bytes(), 'application/octet-stream')
            return self.send_error(404)
        super().do_GET()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--games', required=True, type=Path)
    parser.add_argument('--port', type=int, default=8791)
    args = parser.parse_args()
    dist = Path(__file__).resolve().parents[2] / 'wie-web/dist'
    server = ThreadingHTTPServer(('127.0.0.1', args.port), partial(Handler, directory=str(dist)))
    server.games = library(args.games)
    print(f'WASM library: http://localhost:{args.port}/ ({len(server.games)} versions)', flush=True)
    try:
        server.serve_forever()
    finally:
        server.server_close()


if __name__ == '__main__':
    main()
