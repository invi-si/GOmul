import assert from 'node:assert/strict';
import test from 'node:test';
import {IndexedDBStore,flushBrowserWrites} from '../src/ts/indexed_db_store.ts';
function fixture(){
 const request={};
 const transaction={objectStore:()=>({put:()=>request,delete:()=>request})};
 const store=new IndexedDBStore({transaction:()=>transaction},'records');
 return {store,transaction,request};
}
test('worker shutdown waits for every outstanding write',async()=>{
 const first=fixture(),second=fixture();
 const a=first.store.set('a',new Uint8Array([1]));
 const b=second.store.set('b',new Uint8Array([2]));
 let flushed=false;
 const flush=flushBrowserWrites().then(()=>{flushed=true;});
 first.transaction.oncomplete();await a;await Promise.resolve();
 assert.equal(flushed,false);
 second.transaction.oncomplete();await b;await flush;
 assert.equal(flushed,true);
});
for(const op of ['set','delete']){
 test(`${op} resolves only after the IndexedDB transaction commits`,async()=>{
  const {store,transaction,request}=fixture();let complete=false;
  const promise=(op==='set'?store.set('save',new Uint8Array([1])):store.delete('save')).then(()=>{complete=true;});
  request.onsuccess?.();await Promise.resolve();assert.equal(complete,false);
  transaction.oncomplete();await promise;assert.equal(complete,true);
 });
 test(`${op} rejects if the transaction aborts after a successful request`,async()=>{
  const {store,transaction,request}=fixture();
  const promise=op==='set'?store.set('save',new Uint8Array([1])):store.delete('save');
  request.onsuccess?.();transaction.error=new Error('disk full');transaction.onabort();
  await assert.rejects(promise,/disk full/);
 });
}
