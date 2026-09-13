import assert from 'node:assert/strict';
import test from 'node:test';
import {encodeSave,decodeSave,withGameSaveLock} from '../src/ts/browser_save_store.ts';

test('save bundles retain every byte through browser transport',async()=>{
  const bytes=Uint8Array.from({length:100000},(_,i)=>i%256);
  const result=decodeSave(await encodeSave(new Blob([bytes])));
  assert.deepEqual(new Uint8Array(await result.arrayBuffer()),bytes);
});

test('same-game tabs cannot run concurrently; different games are independent',async()=>{
  const held=new Set();
  const original=Object.getOwnPropertyDescriptor(globalThis,'navigator');
  Object.defineProperty(globalThis,'navigator',{configurable:true,value:{locks:{
    request:async(name,options,callback)=>{
      if(held.has(name))return callback(null);
      held.add(name);try{return await callback({name});}finally{held.delete(name);}
    }
  }}});
  try{
    await withGameSaveLock('game-a',async()=>{
      await assert.rejects(withGameSaveLock('game-a',async()=>assert.fail('must not enter')),/다른 탭/);
      await withGameSaveLock('game-b',async()=>{});
    });
    await withGameSaveLock('game-a',async()=>{});
    await assert.rejects(withGameSaveLock('game-a',async()=>{throw new Error('launch failed');}),/launch failed/);
    await withGameSaveLock('game-a',async()=>{});
  }finally{
    if(original)Object.defineProperty(globalThis,'navigator',original);else delete globalThis.navigator;
  }
});

test('browser writes await transaction commit and surface quota aborts',async()=>{
  const {writeBrowserSave}=await import('../src/ts/browser_save_store.ts');
  let tx;
  const previous=globalThis.indexedDB;
  globalThis.indexedDB={open:()=>{
    const request={};
    queueMicrotask(()=>{request.result={transaction:()=>{
      tx={objectStore:()=>({put:()=>({})})};return tx;
    }};request.onsuccess();});
    return request;
  }};
  try{
    let finished=false;
    const first=writeBrowserSave('a',new Blob(['a'])).then(()=>{finished=true;});
    await new Promise(resolve=>setImmediate(resolve));
    assert.equal(finished,false);
    tx.oncomplete();await first;assert.equal(finished,true);
    const second=writeBrowserSave('a',new Blob(['b']));
    const rejected=assert.rejects(second,/quota/);
    await new Promise(resolve=>setImmediate(resolve));
    tx.error=new Error('quota');tx.onabort();await rejected;
  }finally{globalThis.indexedDB=previous;}
});
