import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import ts from 'typescript';

// Substitute only the external AudioWorklet implementation. Exercise the actual
// initialization function, with controlled HTTP and sound-bank outcomes.
const source = readFileSync(new URL('../src/ts/midi.ts', import.meta.url), 'utf8')
  .replace('import { WorkletSynthesizer } from "spessasynth_lib";',
    'const WorkletSynthesizer = globalThis.TestSynth;');
let created = 0, destroyed = 0, loaded = 0, connected = 0;
let badBank = false;
globalThis.TestSynth = class {
  constructor() { created++; }
  soundBankManager = {addSoundBank: async () => { loaded++; if (badBank) throw Error('Invalid bank'); }};
  isReady = Promise.resolve();
  connect() { connected++; }
  destroy() { destroyed++; }
};
const {initAudio} = await import('data:text/javascript;base64,' + Buffer.from(
  ts.transpileModule(source, {compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022}}).outputText
).toString('base64'));

test('MIDI failures release partial state and leave PCM connected', async () => {
  const oldFetch = globalThis.fetch, oldWarn = console.warn;
  const gains = [];
  const ctx = {currentTime: 0, destination: {}, audioWorklet: {addModule: async () => {}},
    createGain() { const g = {gain: {value: 0}, connect() { g.connected = true; }}; gains.push(g); return g; }};
  console.warn = () => {};
  try {
    globalThis.fetch = async () => ({ok: false, status: 404, arrayBuffer() { throw Error('Must not decode error page'); }});
    const failedFetch = await initAudio(ctx);
    assert.equal(failedFetch.synth, null);
    assert.equal(created, 0);
    assert.equal(failedFetch.pcmGain.connected, true);

    globalThis.fetch = async () => ({ok: true, arrayBuffer: async () => new ArrayBuffer(8)});
    badBank = true;
    const failedBank = await initAudio(ctx);
    assert.equal(failedBank.synth, null);
    assert.equal(destroyed, 1);
    assert.equal(connected, 0);
    assert.equal(failedBank.pcmGain.connected, true);

    badBank = false;
    const ready = await initAudio(ctx);
    assert.notEqual(ready.synth, null);
    assert.equal(created, 2);
    assert.equal(loaded, 2);
    assert.equal(connected, 1);
    assert.equal(destroyed, 1);
  } finally { globalThis.fetch = oldFetch; console.warn = oldWarn; delete globalThis.TestSynth; }
});
