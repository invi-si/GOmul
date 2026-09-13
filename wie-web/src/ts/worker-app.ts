import {installWasmSettings,readLaunch,type BrowserLaunch} from './wasm-settings';
import {ArrowDown,ArrowLeft,ArrowRight,ArrowUp,Settings,createIcons} from 'lucide';
import type {AppMetadata} from './app_library_store';
import type {SettingsController} from './settings';
import {bindGameInput,HeldGameKeys,directionalGameKey} from './game_input';
import {AudioPlayer,setPcmVolume} from './midi';
export function runWorkerApp(app:AppMetadata,archive:Uint8Array,font:Uint8Array,settings:SettingsController,exit:(error?:unknown)=>void,onReady:()=>void=()=>{},launch:BrowserLaunch=readLaunch(app.id),restart:()=>void=()=>exit()):()=>Promise<void> {
  const worker=new Worker(new URL('./cpu-worker.ts',import.meta.url),{type:'module'});
  const audio=new AudioPlayer();
  const player=document.getElementById('player-view')!;
  const canvas=document.getElementById('canvas') as HTMLCanvasElement;
  canvas.width=240;canvas.height=320;
  player.style.setProperty('--screen-ratio','0.75');
  const ctx=canvas.getContext('2d')!;
  const abort=new AbortController();
  const options={signal:abort.signal};
  let active=true,ready=false,id=0;
  let numericDirections=false;
  const startup=setTimeout(()=>exit(new Error('브라우저 실행 준비 시간이 초과되었습니다. 다시 시도해 주세요.')),60000);
  let pending:ImageData|undefined,raf=0;
  const metrics={inputs:[] as number[],paints:0};
  const sent=new Map<number,number>();
  if(new URLSearchParams(location.search).has('profile'))(window as unknown as {gomulWorkerMetrics:typeof metrics}).gomulWorkerMetrics=metrics;
  document.getElementById('player-title')!.textContent=app.title;
  createIcons({icons:{ArrowDown,ArrowLeft,ArrowRight,ArrowUp,Settings},root:player});
  const key=(key:string,down:boolean)=>{if(!ready)return;const n=++id;sent.set(n,performance.timeOrigin+performance.now());worker.postMessage({type:'key',key,down,id:n});};
  const guestKeys=new HeldGameKeys({key_down:k=>key(k,true),key_up:k=>key(k,false)});
  const input=bindGameInput(player,{key_down:k=>guestKeys.press(k,directionalGameKey(k,numericDirections)),key_up:k=>{guestKeys.release(k);}});
  const controls=installWasmSettings(app.id,launch,settings,input.releaseAll,(type,value)=>worker.postMessage({type,value}),restart);
  setPcmVolume(settings.pcmVolume);
  const unsubscribe=settings.onPcmVolumeChange(setPcmVolume);
  document.getElementById('back-to-library')!.addEventListener('click',()=>exit(),options);
  document.getElementById('app-settings')!.addEventListener('click',()=>{controls.open();},options);
  document.addEventListener('visibilitychange',()=>{input.releaseAll();worker.postMessage({type:'pause',paused:document.hidden});},options);
  worker.onmessage=(event:MessageEvent)=>{
    if(!active)return;
    const m=event.data;
    if(m.type==='initialized')worker.postMessage({type:'start',filename:app.filename,archive,font,launch});
    else if(m.type==='ready'){clearTimeout(startup);launch.profile=m.profile;numericDirections=!!m.profile.numericDirections;ready=true;onReady();worker.postMessage({type:'pause',paused:document.hidden});}
    else if(m.type==='frame'){
      pending=new ImageData(m.rgba,m.width,m.height);
      if(!raf)raf=requestAnimationFrame(()=>{
        raf=0;if(!pending||!active)return;
        if(canvas.width!==pending.width||canvas.height!==pending.height){canvas.width=pending.width;canvas.height=pending.height;}
        player.style.setProperty('--screen-ratio',String(canvas.width/canvas.height));
        ctx.putImageData(pending,0,0);metrics.paints++;pending=undefined;
      });
    }else if(m.type==='audio'){
      if(m.op==='play')audio.play(m.handle,m.duration,m.events,m.repeat);else audio.stop(m.handle);
    }else if(m.type==='input-exposed'){
      const start=sent.get(m.id);sent.delete(m.id);if(start!==undefined){metrics.inputs.push(m.time-start);if(metrics.inputs.length>1000)metrics.inputs.shift();}
    }else if(m.type==='exit')exit();
    else if(m.type==='error')exit(new Error(m.error));
  };
  worker.onerror=e=>exit(new Error(e.message));
  return ()=>{
    if(!active)return Promise.resolve();
    clearTimeout(startup);
    input.dispose();controls.dispose();active=false;abort.abort();unsubscribe();audio.dispose();if(raf)cancelAnimationFrame(raf);
    return new Promise<void>((resolve,reject)=>{
      const shutdown=setTimeout(()=>{worker.terminate();reject(new Error("브라우저 실행 종료 시간이 초과되었습니다."));},5000);
      worker.onmessage=e=>{if(e.data.type==='stopped'||e.data.type==='error'){clearTimeout(shutdown);worker.terminate();if(e.data.type==='error')reject(new Error(e.data.error));else resolve();}};
      worker.postMessage({type:'stop'});
    });
  };
}
