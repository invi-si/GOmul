import test from 'node:test';
import assert from 'node:assert/strict';
import {readLaunch} from '../src/ts/wasm-settings.ts';
const values=new Map();globalThis.localStorage={getItem:k=>values.get(k)??null};
test('game settings are separate while speed is global',()=>{
 values.set('gomul-wasm-game:a',JSON.stringify({namespace:'a:backup',settings:{phoneNumber:'01011112222',width:240,height:350},previous:['a']}));
 values.set('gomul-wasm-speed','2000');
 assert.equal(readLaunch('a').settings.width,240);assert.equal(readLaunch('b').settings.width,undefined);
 assert.equal(readLaunch('a').speed,2000);assert.equal(readLaunch('b').speed,2000);
 assert.equal(readLaunch('a').namespace,'a:backup');assert.equal(readLaunch('b').namespace,'b');
});
test('invalid saved speed falls back and extremes are bounded',()=>{
 values.set('gomul-wasm-speed','oops');assert.equal(readLaunch('c').speed,1000);
 values.set('gomul-wasm-speed','99999');assert.equal(readLaunch('c').speed,3000);
 values.set('gomul-wasm-speed','0');assert.equal(readLaunch('c').speed,250);
});
