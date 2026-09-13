/** Durable saves belong to this browser origin/profile, never a server account. */
let database:Promise<IDBDatabase>|undefined;
function open():Promise<IDBDatabase> {
  return database??=new Promise((resolve,reject)=>{
    const request=indexedDB.open('gomul_browser_saves',1);
    request.onupgradeneeded=()=>request.result.createObjectStore('games');
    request.onsuccess=()=>resolve(request.result);
    request.onerror=()=>{database=undefined;reject(request.error);};
  });
}
export async function readBrowserSave(identity:string):Promise<Blob|undefined> {
  const db=await open();
  return new Promise((resolve,reject)=>{
    const tx=db.transaction('games','readonly');
    const request=tx.objectStore('games').get(identity);
    tx.oncomplete=()=>resolve(request.result as Blob|undefined);
    tx.onabort=()=>reject(tx.error);
    tx.onerror=()=>reject(tx.error);
  });
}
export async function writeBrowserSave(identity:string,data:Blob):Promise<void> {
  const db=await open();
  return new Promise((resolve,reject)=>{
    const tx=db.transaction('games','readwrite');
    tx.objectStore('games').put(data,identity);
    // A successful request alone does not mean the transaction committed.
    tx.oncomplete=()=>resolve();
    tx.onabort=()=>reject(tx.error||new Error('브라우저 저장 공간을 확인하세요.'));
    tx.onerror=()=>reject(tx.error);
  });
}
export async function encodeSave(data:Blob):Promise<string> {
  const bytes=new Uint8Array(await data.arrayBuffer());
  let binary='';
  for(let i=0;i<bytes.length;i+=32768)binary+=String.fromCharCode(...bytes.subarray(i,i+32768));
  return btoa(binary);
}
export function decodeSave(data:string):Blob {
  const bytes=Uint8Array.from(atob(data),char=>char.charCodeAt(0));
  return new Blob([bytes],{type:'application/zip'});
}
export async function withGameSaveLock(identity:string,run:()=>Promise<void>):Promise<void> {
  if(!navigator.locks)throw new Error('브라우저 저장을 안전하게 사용하려면 최신 브라우저의 localhost 또는 HTTPS에서 실행하세요.');
  await navigator.locks.request('gomul-save-'+identity,{ifAvailable:true},async lock=>{
    if(!lock)throw new Error('이 브라우저의 다른 탭에서 같은 게임을 실행 중입니다. 먼저 종료하세요.');
    await run();
  });
}
