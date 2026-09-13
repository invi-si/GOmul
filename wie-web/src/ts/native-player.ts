import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Settings, createIcons } from 'lucide';
import type { AppMetadata } from './app_library_store';
import type { SettingsController } from './settings';
import { bindGameInput } from './game_input';
import { AudioPlayer, setPcmVolume } from './midi';
import { readBrowserSave, writeBrowserSave, encodeSave, decodeSave, withGameSaveLock } from './browser_save_store';

interface GameSettings { speed:number; width:number; height:number; custom:boolean; full:boolean; phone:string; autoStartup?:boolean }
type NativeAudio = {stop:number} | {handle:number; duration:number; repeat:boolean; events: ([number,'midi',number[]] | [number,'wave',number,number,number[]])[]};
interface Reply {identity?:string; storage?:string; revision?:string; session?:string; settings?:GameSettings; error?:string; message?:string; setup?:{needsPhone?:boolean}; width?:number; height?:number; pixels?:string; status?:string; korean?:boolean; quick?:boolean; recovery?:boolean; startup?:boolean; rescue?:boolean; startupError?:string; audio?:NativeAudio[]}

async function post(path:string, payload:object):Promise<Response> {
  const response=await fetch(path,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(payload)});
  if(!response.ok){const result=await response.json() as Reply;throw new Error(result.error || '로컬 실행기 요청 실패');}
  return response;
}

export async function runNativeApp(app:AppMetadata, selection:{game:string;index:number}, volumes:SettingsController):Promise<void> {
  const descriptor=await (await post('/api/native/identity',selection)).json() as Reply;
  await withGameSaveLock(descriptor.identity!,async()=>{
    const stored=await readBrowserSave(descriptor.identity!);
    await runBrowserSession(app,selection,volumes,descriptor.identity!,stored);
  });
}

async function runBrowserSession(app:AppMetadata,selection:{game:string;index:number},volumes:SettingsController,identity:string,stored:Blob|undefined):Promise<void> {
  const started=await (await post('/api/native/start',{...selection,identity,storage:stored?await encodeSave(stored):undefined})).json() as Reply;
  const session=started.session!;
  let prefs=started.settings!;
  const library=document.getElementById('library-view')!;
  const player=document.getElementById('player-view')!;
  const canvas=document.getElementById('canvas') as HTMLCanvasElement;
  const context=canvas.getContext('2d')!;
  const dialog=document.getElementById('native-settings') as HTMLDialogElement;
  const notice=document.getElementById('native-notice')!;
  const status=document.getElementById('native-status')!;
  const abort=new AbortController();
  const options={signal:abort.signal};
  let active=true,busy=false,timer:number|undefined,lastStatus='';
  let audio=new AudioPlayer();
  let queue:Promise<unknown>=Promise.resolve();
  const rpc=(command:object):Promise<Reply>=>{
    const result=queue.then(async()=> (await post('/api/native/command',{session,...command})).json() as Promise<Reply>);
    queue=result.catch(()=>{});return result;
  };
  const tell=(text:string,error=false)=>{
    for(const target of [notice,document.getElementById('native-dialog-notice')!]){target.textContent=text;target.classList.toggle('error',error);}
  };
  let revision:string|undefined;
  let storageQueue:Promise<void>=Promise.resolve();
  let syncing=false;
  const persist=():Promise<void>=>{
    const job=storageQueue.catch(()=>{}).then(async()=>{
      syncing=true;
      try {
        const result=await rpc({op:'storage',revision});
        if(result.storage!==undefined)await writeBrowserSave(identity,decodeSave(result.storage));
        revision=result.revision;
      } finally {syncing=false;}
    });
    storageQueue=job;return job;
  };
  const input=bindGameInput(player,{
    key_down:key=>{if(!busy)void rpc({op:'key',key,down:true}).catch(error=>tell(String(error),true));},
    key_up:key=>{void rpc({op:'key',key,down:false}).catch(()=>{});},
  });
  const clearAudio=()=>{audio.dispose();audio=new AudioPlayer();};
  const unsubscribe=volumes.onPcmVolumeChange(setPcmVolume);
  setPcmVolume(volumes.pcmVolume);
  const setKeypad=(korean:boolean)=>{
    const labels:Record<string,string>={'1':'1 ㅣ','2':'2 ㆍ','3':'3 ㅡ','4':'4 ㄱㅋ','5':'5 ㄴㄹ','6':'6 ㄷㅌ','7':'7 ㅂㅍ','8':'8 ㅅㅎ','9':'9 ㅈㅊ','0':'0 ㅇㅁ'};
    for(const button of player.querySelectorAll<HTMLButtonElement>('button[data-key]')){
      const key=button.dataset.key!;
      if(labels[key]){button.textContent=korean?labels[key]:key;button.setAttribute('aria-label',button.textContent);}
    }
    for(const id of ['native-korean','native-korean-settings']){
      const button=document.getElementById(id)!;button.textContent=korean?'ABC':'한글';button.setAttribute('aria-pressed',String(korean));
    }
  };
  const field=(id:string)=>document.getElementById(id) as HTMLInputElement;
  const fill=()=>{
    field('native-width').value=String(prefs.width);field('native-height').value=String(prefs.height);
    field('native-custom').checked=prefs.custom;field('native-full').checked=prefs.full;
    field('native-phone').value=prefs.phone||'';field('native-speed').value=String(prefs.speed);
    field('native-autostart').checked=!!prefs.autoStartup;
    document.getElementById('native-speed-value')!.textContent=`${(prefs.speed/1000).toFixed(2)}×`;
  };
  const action=async(op:string,extra:object={})=>{
    if(busy)return;
    input.releaseAll();busy=true;
    tell(op==='checkpoint'?'저장 상태 처리 중…':'처리 중…');
    try{
      const result=await rpc({op,...extra});
      if(result.settings){prefs=result.settings;fill();}
      if(op==='reset'||op==='settings'||(op==='checkpoint'&&'action' in extra&&extra.action!=='save'))clearAudio();
      await persist();
      tell(result.message?.startsWith('Quick Save')?'빠른 저장을 완료했습니다.':result.message?.startsWith('Quick Load')?'불러오기를 완료했습니다.':result.message||'적용했습니다.');
    }catch(error){tell(String(error),true);}finally{busy=false;}
  };
  const save=()=>void action('checkpoint',{action:'save'});
  const load=()=>void action('checkpoint',{action:'load'});
  for(const id of ['native-save','settings-save'])document.getElementById(id)!.addEventListener('click',save,options);
  for(const id of ['native-load','settings-load'])document.getElementById(id)!.addEventListener('click',load,options);
  document.getElementById('settings-recover')!.addEventListener('click',()=>void action('checkpoint',{action:'recover'}),options);
  document.getElementById('app-settings')!.addEventListener('click',()=>{input.releaseAll();fill();dialog.showModal();},options);
  document.getElementById('native-close')!.addEventListener('click',()=>dialog.close(),options);
  document.getElementById('native-audio')!.addEventListener('click',()=>volumes.open(),options);
  for(const id of ['native-korean','native-korean-settings'])document.getElementById(id)!.addEventListener('click',()=>{
    input.releaseAll();const enabled=document.getElementById('native-korean')!.getAttribute('aria-pressed')!=='true';
    void rpc({op:'korean',enabled}).then(()=>setKeypad(enabled)).catch(error=>tell(String(error),true));
  },options);
  field('native-full').addEventListener('change',()=>{if(field('native-full').checked)field('native-custom').checked=true;},options);
  field('native-speed').addEventListener('input',()=>document.getElementById('native-speed-value')!.textContent=`${(Number(field('native-speed').value)/1000).toFixed(2)}×`,options);
  document.getElementById('native-apply')!.addEventListener('click',()=>{
    const values:GameSettings={speed:Number(field('native-speed').value),width:Number(field('native-width').value),height:Number(field('native-height').value),custom:field('native-custom').checked,full:field('native-full').checked,phone:field('native-phone').value.trim(),autoStartup:field('native-autostart').checked};
    const restart=values.width!==prefs.width||values.height!==prefs.height||values.custom!==prefs.custom||values.full!==prefs.full||values.phone!==prefs.phone;
    if(restart&&!confirm('화면·전화번호 변경 시 게임이 재시작됩니다. 게임 안에서 저장하셨나요?'))return;
    void action('settings',{values});
  },options);
  document.getElementById('native-startup')!.addEventListener('click',()=>{
    if(confirm('현재 장면을 이 게임의 보호된 시작 상태로 저장할까요? 다음 실행부터 자동으로 불러옵니다. 빠른 저장은 유지됩니다.'))void action('startup-save');
  },options);
  document.getElementById('native-reset')!.addEventListener('click',()=>{
    if(confirm('이 게임의 진행 상황과 빠른 저장·시작 상태를 초기화하고 재시작할까요? 브라우저의 이전 저장 상태도 교체됩니다.'))void action('reset');
  },options);
  const exportRescue=async(latest:boolean)=>{
    if(busy)return;input.releaseAll();busy=true;tell('오류 보고서를 만드는 중…');
    try{
      await queue;
      const response=await post('/api/native/command',{session,op:latest?'rescue-latest':'rescue'});
      const url=URL.createObjectURL(await response.blob());
      const link=document.createElement('a');link.href=url;link.download=`GOmul-${app.title}-rescue.zip`;link.click();
      window.setTimeout(()=>URL.revokeObjectURL(url),30000);tell('오류 보고서 ZIP을 내보냈습니다.');
    }catch(error){tell(String(error),true);}finally{busy=false;}
  };
  document.getElementById('native-rescue')!.addEventListener('click',()=>void exportRescue(false),options);
  document.getElementById('native-latest-rescue')!.addEventListener('click',()=>void exportRescue(true),options);
  const poll=async()=>{
    if(!active)return;
    try{
      if(!busy&&!document.hidden){
        const result=await rpc({op:'poll'});
        if(!active)return;
        if(result.pixels){
          const bytes=Uint8Array.from(atob(result.pixels),c=>c.charCodeAt(0));
          if(canvas.width!==result.width||canvas.height!==result.height){canvas.width=result.width!;canvas.height=result.height!;player.style.setProperty('--screen-ratio',String(canvas.width/canvas.height));}
          context.putImageData(new ImageData(new Uint8ClampedArray(bytes.buffer),canvas.width,canvas.height),0,0);
        }
        setKeypad(!!result.korean);
        for(const id of ['native-load','settings-load'])(document.getElementById(id) as HTMLButtonElement).disabled=!result.quick;
        (document.getElementById('settings-recover') as HTMLButtonElement).disabled=!result.recovery;
        (document.getElementById('native-latest-rescue') as HTMLButtonElement).disabled=!result.rescue;
        status.textContent=result.status==='Running'?'실행 중':result.status==='Loading…'?'게임 시작 중…':result.status?.startsWith('Game stopped')?'게임 중지 · 빠른 불러오기 가능':result.status==='Stopped'?'종료됨':result.status||'';
        if(result.status!==lastStatus&&result.status?.startsWith('Error:'))tell('게임 오류가 발생했습니다. 설정에서 오류 보고서를 내보내거나 빠른 불러오기를 사용하세요.',true);
        lastStatus=result.status||'';
        if(result.startupError)tell('시작 상태를 불러오지 못했습니다. '+result.startupError,true);
        const rate=prefs.speed/1000;
        for(const command of result.audio||[]){
          if('stop' in command)audio.stop(command.stop);
          else audio.play(command.handle,command.duration/rate,command.events.map(event=>event[1]==='midi'?[event[0]/rate,'midi',new Uint8Array(event[2])]:[event[0]/rate,'wave',event[2],Math.round(event[3]*rate),new Int16Array(event[4])]),command.repeat);
        }
      }
    }catch(error){tell(String(error),true);active=false;input.releaseAll();audio.dispose();}
    finally{if(active)timer=window.setTimeout(()=>void poll(),33);}
  };
  const syncVisibility=()=>{
    if(!active)return;
    input.releaseAll();
    if(document.hidden)audio.dispose();else clearAudio();
    if(document.hidden)void persist().catch(error=>tell('브라우저 저장 실패: '+String(error),true));
    void rpc({op:'pause',paused:document.hidden}).then(()=>{if(active&&!document.hidden)return rpc({op:'speed',value:prefs.speed});}).catch(()=>{});
  };
  document.addEventListener('visibilitychange',syncVisibility,options);
  window.addEventListener('pageshow',syncVisibility,options);
  // A download may finish after the user has already hidden this tab.
  if(document.hidden)syncVisibility();
  window.addEventListener('pagehide',()=>{
    void fetch('/api/native/command',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({session,op:'pause',paused:true}),keepalive:true});
  },options);
  library.hidden=true;player.hidden=false;
  document.getElementById('player-title')!.textContent=app.title;
  canvas.width=240;canvas.height=320;context.fillStyle='#000';context.fillRect(0,0,240,320);
  createIcons({icons:{ArrowDown,ArrowLeft,ArrowRight,ArrowUp,Settings},root:player});
  fill();tell(started.setup?.needsPhone?'초기 데이터에 필요한 가상 전화번호를 설정에 입력하세요.':'');
  void poll();
  const autoSave=window.setInterval(()=>{
    if(!busy&&!syncing)void persist().catch(error=>tell('브라우저 저장 실패: '+String(error),true));
  },15000);
  void persist().catch(error=>tell('브라우저 저장 실패: '+String(error),true));
  return new Promise(resolve=>{
    const finish=()=>{
      input.dispose();active=false;window.clearTimeout(timer);window.clearInterval(autoSave);audio.dispose();unsubscribe();abort.abort();dialog.close();
      void rpc({op:'stop'}).catch(()=>{}).finally(()=>{player.hidden=true;library.hidden=false;resolve();});
    };
    document.getElementById('back-to-library')!.addEventListener('click',()=>{
    if(busy){tell('진행 중인 저장 작업을 기다려 주세요.');return;}
    busy=true;input.releaseAll();tell('이 브라우저에 저장하는 중…');
    void persist().then(finish).catch(error=>{
      busy=false;tell('브라우저 저장 실패: '+String(error)+' 저장 공간을 확인하고 다시 시도하세요.',true);
      if(confirm('최신 진행 상황을 브라우저에 저장하지 못했습니다. 마지막으로 저장된 데이터만 유지하고 목록으로 돌아갈까요?'))finish();
    });
    },options);
  });
}
