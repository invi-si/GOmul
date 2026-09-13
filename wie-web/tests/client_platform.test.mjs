import assert from 'node:assert/strict';
import test from 'node:test';
import { clientPlatform } from '../src/ts/client_platform.ts';

test('phones and tablets use the mobile layout, including desktop-mode iPad', () => {
  for (const device of [
    { userAgent:'Mozilla/5.0 (Linux; Android 16) Mobile', platform:'Linux aarch64' },
    { userAgent:'Mozilla/5.0 (iPhone; CPU iPhone OS)', platform:'iPhone' },
    { userAgent:'Mozilla/5.0 (Macintosh; Intel Mac OS X)', platform:'MacIntel', maxTouchPoints:5 },
  ]) assert.equal(clientPlatform(device), 'mobile');
});

test('touch-enabled Windows computers still use Windows controls', () => {
  assert.equal(clientPlatform({ userAgent:'Mozilla/5.0 (Windows NT 10.0; Win64; x64)', platform:'Win32', maxTouchPoints:10 }), 'windows');
});

test('Mac and Linux are detected as desktop', () => {
  assert.equal(clientPlatform({ userAgent:'Mozilla/5.0 (Macintosh)', platform:'MacIntel', maxTouchPoints:0 }), 'desktop');
  assert.equal(clientPlatform({ userAgent:'Mozilla/5.0 (X11; Linux x86_64)', platform:'Linux x86_64' }), 'desktop');
});
