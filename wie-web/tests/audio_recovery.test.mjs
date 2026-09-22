import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import ts from 'typescript';

const originalSynth = globalThis.TestSynth;
globalThis.TestSynth = class {
  soundBankManager = {addSoundBank: async () => {}};
  isReady = Promise.resolve();
  connect() {}
  destroy() {}
};
const source = readFileSync(new URL('../src/ts/midi.ts', import.meta.url), 'utf8')
  .replace('import { WorkletSynthesizer } from "spessasynth_lib";', 'const WorkletSynthesizer = globalThis.TestSynth;')
  .replace('new URL("./audio-worker.ts", import.meta.url)', '"audio-worker.ts"');
const {AudioPlayer} = await import('data:text/javascript;base64,' + Buffer.from(
  ts.transpileModule(source, {compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022}}).outputText
).toString('base64'));

test('interrupted audio resumes once on a gesture and queued playback uses the rebased clock', async () => {
  const originals = {
    AudioContext: globalThis.AudioContext,
    Worker: globalThis.Worker,
    document: globalThis.document,
    fetch: globalThis.fetch,
    TestSynth: originalSynth,
  };
  const listeners = new Map();
  const posted = [];
  let resumeCalls = 0;
  let finishResume;
  const context = {
    state: 'interrupted', currentTime: 2, destination: {},
    audioWorklet: {addModule: async () => {}},
    createGain: () => ({gain: {value: 0}, connect() {}}),
    resume() {
      resumeCalls++;
      return new Promise(resolve => {
        finishResume = () => { this.state = 'running'; this.currentTime = 5; resolve(); };
      });
    },
    close: async () => {},
  };
  globalThis.AudioContext = class {constructor() { return context; }};
  globalThis.Worker = class {postMessage(message) { posted.push(message); } terminate() {}};
  globalThis.document = {addEventListener(name, listener) { listeners.set(name, listener); }};
  globalThis.fetch = async () => ({ok: true, arrayBuffer: async () => new ArrayBuffer(1)});
  let player;
  try {
    player = new AudioPlayer();
    const state = await player.audioReady;
    assert.ok(state.synth);
    assert.equal(state.clockAudioTime, 2);

    listeners.get('pointerdown')();
    listeners.get('keydown')();
    player.play(1, 100, [], false);
    await Promise.resolve();
    assert.equal(resumeCalls, 1);
    assert.equal(posted.length, 0);

    finishResume();
    await player.commands;
    assert.equal(state.clockAudioTime, 5);
    assert.ok(state.clockPerformanceTime >= performance.timeOrigin);
    assert.deepEqual(posted.map(message => message.type), ['play']);
    listeners.get('pointerdown')();
    assert.equal(resumeCalls, 1);
  } finally {
    player?.dispose();
    Object.assign(globalThis, originals);
  }
});
