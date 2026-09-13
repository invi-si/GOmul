#!/usr/bin/env python3
"""Loopback-only GOmul launcher: live DubiGame index and on-demand attachments."""
import argparse
from browser_sessions import BrowserSessions
import hashlib
import io
import html
from html.parser import HTMLParser
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import re
import threading
import time
import zipfile
from urllib.parse import urljoin, urlsplit, unquote
from urllib.request import Request, HTTPRedirectHandler, build_opener
from urllib.error import HTTPError

BASE = 'https://dubigame.tistory.com'
CARRIERS = ('LGT', 'KTF', 'SKT')
LIMIT = 64 * 1024 * 1024
NETWORK = threading.BoundedSemaphore(3)
THUMBNAIL_NETWORK = threading.BoundedSemaphore(2)
EXCLUDED_GAMES = {
    (entry['carrier'], entry['id'])
    for entry in json.loads(Path(__file__).with_name('excluded-games.json').read_text())
}


def eligible_games(entries):
    # Carrier/post identity avoids spelling aliases and keeps other versions
    # independent. Apply after reading raw caches, not only during scraping.
    return [entry for entry in entries if (entry['carrier'], entry['id']) not in EXCLUDED_GAMES
            and not any(word in entry['title'] for word in ('맞고', '올림픽'))]


def allowed_url(url):
    p = urlsplit(url)
    host = p.hostname or ''
    return p.scheme == 'https' and not p.username and not p.password and p.port in (None, 443) and (
        host == 'dubigame.tistory.com' or any(host == d or host.endswith('.' + d) for d in ('kakaocdn.net', 'daumcdn.net')))


class Redirects(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        if not allowed_url(newurl):
            raise ValueError('허용되지 않은 다운로드 주소입니다.')
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def fetch(url, limit=LIMIT, *, image=False):
    if not allowed_url(url):
        raise ValueError('허용되지 않은 다운로드 주소입니다.')
    with (THUMBNAIL_NETWORK if image else NETWORK), build_opener(Redirects()).open(Request(url, headers={
        'User-Agent': 'Mozilla/5.0 GOmul-local', 'Referer': BASE + '/',
    }), timeout=30) as response:
        length = response.headers.get('Content-Length')
        expected = int(length) if length is not None else None
        if expected is not None and expected > limit:
            raise ValueError('다운로드 크기 제한을 초과했습니다.')
        data = response.read(limit + 1)
        if len(data) > limit:
            raise ValueError('다운로드 크기 제한을 초과했습니다.')
        if expected is not None and len(data) != expected:
            raise ValueError('다운로드가 중간에 끊겼습니다. 다시 실행해 주세요.')
        return data, response.headers.get_content_type()


class Links(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links = []
        self.meta = {}

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'a' and attrs.get('href'):
            self.links.append(urljoin(BASE, attrs['href']))
        if tag == 'meta':
            self.meta[attrs.get('property', attrs.get('name'))] = attrs.get('content', '')


def parse_index(body, carrier):
    entries = []
    for block in re.findall(r'<div class="post-item">(.*?)</div>', body, re.S):
        link = re.search(r'<a href="/(\d+)"', block)
        title = re.search(r'<span class="title">(.*?)</span>', block, re.S)
        image = re.search(r'<img[^>]*src="([^"]+)"', block)
        if not link or not title:
            continue
        name = html.unescape(re.sub('<[^>]+>', '', title[1])).strip()
        if not name.startswith('[' + carrier + ']'):
            continue
        entries.append({'id': link[1], 'carrier': carrier, 'title': name.split(']', 1)[1].strip(),
                        'source': BASE + '/' + link[1],
                        'image': urljoin(BASE, html.unescape(image[1])) if image else None})
    return entries


def parse_attachments(body):
    parser = Links()
    parser.feed(body)
    files = []
    for url in dict.fromkeys(parser.links):
        name = unquote(urlsplit(url).path.rsplit('/', 1)[-1])
        if allowed_url(url) and name.lower().endswith(('.jar', '.zip', '.alz')):
            files.append({'name': name, 'url': url})
    return files


class Catalog:
    def __init__(self, cache):
        self.cache = cache
        cache.mkdir(parents=True, exist_ok=True)
        self.lock = threading.Lock()
        self.entries = None
        self.attachment_locks = {}

    def index(self):
        with self.lock:
            if self.entries is not None:
                return eligible_games(self.entries)
            saved = self.cache / 'catalog.json'
            if saved.exists() and time.time() - saved.stat().st_mtime < 86400:
                try:
                    self.entries = json.loads(saved.read_text())
                    return eligible_games(self.entries)
                except (OSError, json.JSONDecodeError):
                    pass
            entries = {}
            for carrier in CARRIERS:
                first = fetch(BASE + '/category/' + carrier, 4 * 1024 * 1024)[0].decode()
                pages = max([int(n) for n in re.findall(r'/category/' + carrier + r'\?page=(\d+)', first)] + [1])
                for page in range(1, min(pages, 100) + 1):
                    body = first if page == 1 else fetch(BASE + '/category/' + carrier + '?page=' + str(page), 4 * 1024 * 1024)[0].decode()
                    for entry in parse_index(body, carrier):
                        entries[entry['id']] = entry
            if not entries:
                raise ValueError('사이트에서 게임 목록을 찾지 못했습니다. 잠시 후 다시 시도하세요.')
            self.entries = sorted(entries.values(), key=lambda e: (e['title'], e['carrier']))
            temporary = saved.with_suffix('.part')
            temporary.write_text(json.dumps(self.entries, ensure_ascii=False))
            temporary.replace(saved)
            return eligible_games(self.entries)

    def game(self, game_id):
        for game in self.index():
            if game['id'] == game_id:
                return game
        raise KeyError('게임을 찾을 수 없습니다.')

    def attachments(self, game_id, *, refresh=False):
        game = self.game(game_id)
        with self.lock:
            lock = self.attachment_locks.setdefault(game_id, threading.Lock())
        with lock:
            manifest = self.cache / (game_id + '.attachments.json')
            if not refresh and manifest.exists() and time.time() - manifest.stat().st_mtime < 3600:
                try:
                    return json.loads(manifest.read_text())
                except (OSError, json.JSONDecodeError):
                    pass
            files = parse_attachments(fetch(game['source'], 4 * 1024 * 1024)[0].decode())
            temporary = manifest.with_suffix('.part')
            temporary.write_text(json.dumps(files, ensure_ascii=False))
            temporary.replace(manifest)
            return files

    def rom(self, game_id, index):
        files = self.attachments(game_id)
        if index >= len(files) or index < 0:
            raise KeyError('첨부파일을 찾을 수 없습니다.')
        selected = files[index]
        try:
            return self.cached(selected['url'], 'rom')
        except HTTPError as error:
            if error.code not in (401, 403):
                raise
            # Expired signed link: refresh once and keep the selected filename,
            # even if the source post has reordered its attachments.
            refreshed = self.attachments(game_id, refresh=True)
            replacement = next((file for file in refreshed if file['name'] == selected['name']), None)
            if replacement is None:
                raise KeyError('선택한 첨부파일이 변경되었습니다. 게임을 다시 선택하세요.')
            return self.cached(replacement['url'], 'rom')

    def cached(self, url, kind):
        dest = self.cache / (hashlib.sha256(url.encode()).hexdigest() + '.' + kind)
        if dest.exists():
            existing = dest.read_bytes()
            if kind != 'rom' or existing.startswith(b'ALZ\x01') or zipfile.is_zipfile(io.BytesIO(existing)):
                return existing
            # Older builds accepted only the first four ZIP bytes, so an
            # interrupted download could become a permanently broken cache hit.
            dest.unlink(missing_ok=True)
        data, mime = fetch(url, 8 * 1024 * 1024 if kind == 'image' else LIMIT, image=kind == 'image')
        if kind == 'image' and not mime.startswith('image/'):
            raise ValueError('썸네일을 불러올 수 없습니다.')
        if kind == 'rom' and not data.startswith(b'ALZ\x01') and not zipfile.is_zipfile(io.BytesIO(data)):
            raise ValueError('첨부파일이 지원되는 ZIP/JAR/ALZ 형식이 아닙니다.')
        temporary = dest.with_suffix('.' + str(threading.get_ident()) + '.part')
        temporary.write_bytes(data)
        temporary.replace(dest)
        return data


class Handler(SimpleHTTPRequestHandler):
    def trusted_request(self):
        return self.headers.get('Host') in self.server.hosts and self.headers.get('Origin') in (
            None, *('http://' + host for host in self.server.hosts))

    def end_headers(self):
        self.send_header('X-Content-Type-Options', 'nosniff')
        self.send_header('X-Frame-Options', 'DENY')
        self.send_header('Content-Security-Policy', "frame-ancestors 'none'; object-src 'none'; base-uri 'self'")
        self.send_header('Referrer-Policy', 'no-referrer')
        super().end_headers()

    def send_head(self):
        # Both GET and HEAD pass here, including translated/percent-encoded
        # paths. Never list directories or serve symlinks outside the build.
        if not self.trusted_request():
            self.send_error(403); return None
        path = unquote(urlsplit(self.path).path)
        if path == '/':
            self.path = '/catalog.html'
            path = self.path
        if any(part.startswith('.') for part in path.split('/') if part):
            self.send_error(404); return None
        target = Path(self.translate_path(self.path)).resolve()
        root = Path(self.directory).resolve()
        if root not in target.parents or not target.is_file():
            self.send_error(404); return None
        return super().send_head()

    def log_message(self, format, *args):
        pass  # Do not log CDN tokens or private filesystem paths.

    def send_data(self, data, mime, code=200, cache_control='no-store'):
        self.send_response(code)
        self.send_header('Content-Type', mime)
        self.send_header('Content-Length', str(len(data)))
        self.send_header('Cache-Control', cache_control)
        self.end_headers()
        self.wfile.write(data)

    def json(self, value, code=200):
        self.send_data(json.dumps(value, ensure_ascii=False).encode(), 'application/json; charset=utf-8', code)

    def do_POST(self):
        if not self.trusted_request():
            self.send_error(403); return
        if self.headers.get('Content-Type') != 'application/json':
            self.send_error(415); return
        try:
            size=int(self.headers.get('Content-Length','0'))
            limit = 180 * 1024 * 1024 if self.path == '/api/native/start' else 65536
            if not 0 < size < limit:raise ValueError('잘못된 요청 크기입니다.')
            payload=json.loads(self.rfile.read(size))
            if not isinstance(payload,dict):raise ValueError('JSON 객체가 필요합니다.')
            if self.path=='/api/native/start':
                result=self.server.native.launch(payload)
            elif self.path=='/api/native/identity':
                result=self.server.native.identity(payload)
            elif self.path=='/api/native/command':
                result=self.server.native.command(payload)
            else:
                self.send_error(404); return
            if isinstance(result,bytes):return self.send_data(result,'application/zip')
            self.json(result)
        except (BrokenPipeError,ConnectionResetError):pass
        except Exception as error:self.json({'error':str(error) if isinstance(error,ValueError) else '로컬 실행기 요청에 실패했습니다.'},400)

    def do_GET(self):
        # Reject foreign Host/Origin values even though only loopback is bound.
        if self.headers.get('Host') not in self.server.hosts:
            self.send_error(403)
            return
        origin = self.headers.get('Origin')
        if origin and origin not in {'http://' + h for h in self.server.hosts}:
            self.send_error(403)
            return
        path = urlsplit(self.path).path
        if not path.startswith('/api/'):
            if path == '/':
                self.path = '/catalog.html'
            if path.startswith('/.') or '..' in unquote(path).split('/'):
                self.send_error(404)
                return
            return super().do_GET()
        try:
            catalog = self.server.catalog
            if path == '/api/catalog':
                return self.json([dict(id=e['id'], title=e['title'], carrier=e['carrier'], source=e['source'],
                                       thumbnail='/api/game/' + e['id'] + '/thumbnail' if e['image'] else None) for e in catalog.index()])
            match = re.fullmatch(r'/api/game/(\d+)/(files|thumbnail|rom/\d+)', path)
            if not match:
                return self.json({'error': '잘못된 요청입니다.'}, 404)
            game_id, action = match.groups()
            if action == 'files':
                return self.json([{'name': f['name'], 'index': i} for i, f in enumerate(catalog.attachments(game_id))])
            if action == 'thumbnail':
                url = catalog.game(game_id)['image']
                if not url:
                    return self.send_error(404)
                data = catalog.cached(url, 'image')
                mime = 'image/png' if data.startswith(b'\x89PNG') else 'image/gif' if data.startswith(b'GIF') else 'image/webp' if data[8:12] == b'WEBP' else 'image/jpeg'
                return self.send_data(data, mime, cache_control='private, max-age=86400')
            self.send_data(catalog.rom(game_id, int(action.split('/')[1])), 'application/octet-stream')
        except (BrokenPipeError, ConnectionResetError):
            pass
        except Exception as error:
            # No remote URLs/signatures in browser errors.
            self.json({'error': str(error) if isinstance(error, (ValueError, KeyError)) else '원본 사이트 요청에 실패했습니다. 잠시 후 다시 시도하세요.'}, 502)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port', type=int, default=8790)
    parser.add_argument('--cache', type=Path, default=Path(os.environ.get('XDG_CACHE_HOME', Path.home() / '.cache')) / 'gomul-web')
    args = parser.parse_args()
    dist = Path(__file__).resolve().parents[2] / 'wie-web' / 'dist'
    if not (dist / 'index.html').exists():
        parser.error('Run npm run build:prod first.')
    server = ThreadingHTTPServer(('127.0.0.1', args.port), lambda *a, **kw: Handler(*a, directory=str(dist), **kw))
    server.hosts = {f'localhost:{args.port}', f'127.0.0.1:{args.port}'}
    server.catalog = Catalog(args.cache)
    server.native = BrowserSessions(server.catalog)
    print(f'GOmul: http://localhost:{args.port}', flush=True)
    try:
        server.serve_forever()
    finally:
        server.native.close()
        server.server_close()


if __name__ == '__main__':
    main()
