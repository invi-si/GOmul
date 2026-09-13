import {withGameSaveLock,encodeSave,decodeSave} from './browser_save_store';
import {readLaunch,activateImportedSave,type LaunchSettings} from './wasm-settings';

type Entry={key:string|[string,string];data:string};
type SavedStore={kind:'files'|'records';appId:string;entries:Entry[]};
export interface SaveBackup {format:'gomul-browser-save';version:1;game:string;settings:LaunchSettings;stores:SavedStore[];}
const MAX_BYTES=64*1024*1024;
const prefix=(namespace:string)=>`gomul_wasm_${new TextEncoder().encode(namespace).length}:${namespace}`;
function names(namespace:string,store:Pick<SavedStore,'kind'|'appId'>):[string,string] {
  const name=store.kind==='files'?`gomul_wasm_fs_${namespace}`:prefix(namespace)+store.appId;
  return [name,store.kind==='files'?'files':name];
}
function open(name:string,store:string,create=false):Promise<IDBDatabase> {
  return new Promise((resolve,reject)=>{
    const request=indexedDB.open(name);
    request.onupgradeneeded=()=>{if(create)request.result.createObjectStore(store);else request.transaction!.abort();};
    request.onsuccess=()=>resolve(request.result);
    request.onerror=()=>reject(request.error);
  });
}
async function read(name:string,store:string):Promise<Entry[]> {
  const db=await open(name,store);
  try {
    const entries=await new Promise<{key:Entry['key'];data:Uint8Array}[]>((resolve,reject)=>{
      const tx=db.transaction(store),values:{key:Entry['key'];data:Uint8Array}[]=[];
      const cursor=tx.objectStore(store).openCursor();
      cursor.onsuccess=()=>{const c=cursor.result;if(c){values.push({key:c.key as Entry['key'],data:c.value});c.continue();}};
      tx.oncomplete=()=>resolve(values);tx.onabort=()=>reject(tx.error||new Error('브라우저 저장 작업이 취소되었습니다.'));
    });
    return Promise.all(entries.map(async entry=>({key:entry.key,data:await encodeSave(new Blob([new Uint8Array(entry.data)]))})));
  }finally{db.close();}
}
export function validateBackup(value:unknown,game:string):SaveBackup {
  const bad=()=>{throw new Error('이 게임의 GOmul 브라우저 저장 파일이 아닙니다.');};
  if(!value||typeof value!=='object')return bad();
  const b=value as SaveBackup;
  if(b.format!=='gomul-browser-save'||b.version!==1||b.game!==game||!Array.isArray(b.stores)||!b.stores.length||!b.settings||typeof b.settings!=='object')return bad();
  const s=b.settings;
  if(s.phoneNumber!==undefined&&(typeof s.phoneNumber!=='string'||!/^\d{11}$/.test(s.phoneNumber)))return bad();
  if((s.width!==undefined||s.height!==undefined)&&![s.width,s.height].every(n=>typeof n==='number'&&Number.isInteger(n)&&n>=64&&n<=1024))return bad();
  if(s.fullFramebuffer!==undefined&&typeof s.fullFramebuffer!=='boolean')return bad();
  const stores=new Set<string>();let bytes=0;
  for(const store of b.stores){
    if(!store||!['files','records'].includes(store.kind)||typeof store.appId!=='string'||store.appId.length>1024||!Array.isArray(store.entries)||(store.kind==='files'&&store.appId!==''))return bad();
    const identity=JSON.stringify([store.kind,store.appId]);if(stores.has(identity))return bad();stores.add(identity);
    const keys=new Set<string>();
    for(const entry of store.entries){
      if(!entry||(typeof entry.key!=='string'&&!(Array.isArray(entry.key)&&entry.key.length===2&&entry.key.every(k=>typeof k==='string')))||typeof entry.data!=='string')return bad();
      const key=JSON.stringify(entry.key);if(keys.has(key))return bad();keys.add(key);
      // atob rejects malformed base64; this checks all data before any writes.
      try{bytes+=atob(entry.data).length;}catch{return bad();}
      if(bytes>MAX_BYTES)throw new Error('저장 데이터가 너무 큽니다 (최대 64MB).');
    }
  }
  return b;
}
export async function exportBrowserBackup(game:string):Promise<Blob> {
  let result:Blob|undefined;
  await withGameSaveLock(game,async()=>{
    const launch=readLaunch(game),stores:SavedStore[]=[];
    for(const database of await indexedDB.databases()){
      const name=database.name;if(!name)continue;
      if(name===`gomul_wasm_fs_${launch.namespace}`)stores.push({kind:'files',appId:'',entries:await read(name,'files')});
      else if(name.startsWith(prefix(launch.namespace)))stores.push({kind:'records',appId:name.slice(prefix(launch.namespace).length),entries:await read(name,name)});
    }
    if(!stores.length)throw new Error('이 게임의 브라우저 저장 데이터가 없습니다.');
    stores.sort((a,b)=>a.kind.localeCompare(b.kind)||a.appId.localeCompare(b.appId));
    const backup:SaveBackup={format:'gomul-browser-save',version:1,game,settings:launch.settings,stores};
    validateBackup(backup,game);
    result=new Blob([JSON.stringify(backup)],{type:'application/json'});
  });
  return result!;
}
export async function importBrowserBackup(game:string,file:Blob):Promise<void> {
  if(file.size>128*1024*1024)throw new Error('저장 파일이 너무 큽니다.');
  const backup=validateBackup(JSON.parse(await file.text()),game);
  await withGameSaveLock(game,async()=>{
    // Install into an unused namespace, then switch one preference only after
    // every database commits. Existing progress remains available for undo.
    const namespace=game+':restore:'+crypto.randomUUID(),created:string[]=[];
    try {
      for(const store of backup.stores){
        const [name,table]=names(namespace,store);
        const entries=await Promise.all(store.entries.map(async entry=>({key:entry.key,data:new Uint8Array(await decodeSave(entry.data).arrayBuffer())})));
        created.push(name);const db=await open(name,table,true);
        try{await new Promise<void>((resolve,reject)=>{
          const tx=db.transaction(table,'readwrite'),target=tx.objectStore(table);
          for(const entry of entries)target.put(entry.data,entry.key);
          tx.oncomplete=()=>resolve();tx.onabort=()=>reject(tx.error||new Error('브라우저 저장 작업이 취소되었습니다.'));
        });}finally{db.close();}
      }
      activateImportedSave(game,namespace,backup.settings);
    }catch(error){
      for(const name of created)indexedDB.deleteDatabase(name);
      throw error;
    }
  });
}
