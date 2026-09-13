import type {SettingsController} from './settings';
export interface LaunchSettings {phoneNumber?:string;width?:number;height?:number;fullFramebuffer?:boolean;}
export interface LaunchProfile {phoneNumber?:string;numericDirections?:boolean;}
export interface BrowserLaunch {namespace:string;settings:LaunchSettings;speed:number;profile?:LaunchProfile;}
interface Preferences {namespace:string;settings:LaunchSettings;previous:string[];}
function preferences(id:string):Preferences {
  try {const saved=JSON.parse(localStorage.getItem('gomul-wasm-game:'+id)||'null');if(saved&&typeof saved.namespace==='string'&&saved.settings)return {...saved,previous:Array.isArray(saved.previous)?saved.previous:[]};}catch{}
  return {namespace:id,settings:{},previous:[]};
}
export function readLaunch(id:string):BrowserLaunch {
  const p=preferences(id);const rate=Number(localStorage.getItem('gomul-wasm-speed')||1000);
  return {namespace:p.namespace,settings:p.settings,speed:Number.isFinite(rate)?Math.min(3000,Math.max(250,rate)):1000};
}
export function activateImportedSave(id:string,namespace:string,settings:LaunchSettings):void {
  const p=preferences(id);p.previous.push(p.namespace);p.namespace=namespace;p.settings=settings;
  localStorage.setItem('gomul-wasm-game:'+id,JSON.stringify(p));
}
export function installWasmSettings(id:string,launch:BrowserLaunch,volume:SettingsController,release:()=>void,command:(type:'speed'|'korean',value:number|boolean)=>void,restart:()=>void){
  const dialog=document.createElement('dialog');dialog.className='app-dialog native-settings';
  dialog.innerHTML=`<form method="dialog"><header class="dialog-header"><h2>게임 설정</h2><button aria-label="닫기">×</button></header>
    <fieldset><legend>화면 크기 · 이 게임</legend><label><input name="custom" type="checkbox"> 직접 설정</label><div class="native-dimensions"><label>가로 <input name="width" type="number" min="64" max="1024" value="240"></label><label>세로 <input name="height" type="number" min="64" max="1024" value="320"></label></div><label><input name="full" type="checkbox"> 전체 프레임버퍼 표시 (LGT)</label></fieldset>
    <fieldset><legend>실행 속도 <output></output></legend><input name="speed" type="range" min="250" max="3000" step="250"><p>모든 게임에 적용 · 2배 초과는 실험적 기능입니다. 실제 속도는 기기 성능에 따라 달라집니다.</p></fieldset>
    <fieldset><legend>초기 데이터 설정 · 이 게임</legend><label>가상 전화번호 <input name="phone" type="tel" maxlength="11" placeholder="자동 설정 또는 11자리"></label><p>함께 제공된 데이터에 맞는 번호를 사용합니다.</p></fieldset>
    <p class="settings-error" role="status"></p><button type="button" data-action="apply">설정 적용 · 게임 재시작</button>
    <div class="native-actions"><button type="button" data-action="korean">한글 입력</button><button type="button" data-action="audio">소리 설정</button></div>
    <div class="native-actions"><button type="button" data-action="fresh">새 저장으로 시작</button><button type="button" data-action="previous">이전 저장으로 되돌리기</button></div><p>새 저장을 만들면 초기 데이터를 다시 설치합니다. 기존 저장은 이 브라우저에 보관하며 되돌릴 수 있습니다.</p></form>`;
  document.body.append(dialog);
  const field=(name:string)=>dialog.querySelector<HTMLInputElement>(`[name="${name}"]`)!;
  const action=(name:string)=>dialog.querySelector<HTMLButtonElement>(`[data-action="${name}"]`)!;
  const abort=new AbortController(),opts={signal:abort.signal};let korean=false;
  const toggle=document.getElementById('native-korean') as HTMLButtonElement|null;
  const labels:Record<string,string>={'1':'1 ㅣ','2':'2 ㆍ','3':'3 ㅡ','4':'4 ㄱㅋ','5':'5 ㄴㄹ','6':'6 ㄷㅌ','7':'7 ㅂㅍ','8':'8 ㅅㅎ','9':'9 ㅈㅊ','0':'0 ㅇㅁ'};
  const setKorean=()=>{
    release();korean=!korean;command('korean',korean);
    for(const button of document.querySelectorAll<HTMLButtonElement>('#player-view button[data-key]')){const key=button.dataset.key!;if(labels[key])button.textContent=korean?labels[key]:key;}
    for(const button of [toggle,action('korean')])if(button){button.textContent=korean?'숫자 입력':'한글 입력';button.setAttribute('aria-pressed',String(korean));}
  };
  if(toggle){toggle.disabled=false;toggle.title='';toggle.textContent='한글';toggle.addEventListener('click',setKorean,opts);}
  action('korean').addEventListener('click',setKorean,opts);
  const save=(p:Preferences)=>localStorage.setItem('gomul-wasm-game:'+id,JSON.stringify(p));
  const apply=()=>{
    const p=preferences(id);const phone=field('phone').value.trim();
    const width=Number(field('width').value),height=Number(field('height').value);
    if(phone&&!/^\d{11}$/.test(phone)){dialog.querySelector('.settings-error')!.textContent='전화번호는 숫자 11자리여야 합니다.';return;}
    if(field('custom').checked&&(!Number.isInteger(width)||!Number.isInteger(height)||width<64||width>1024||height<64||height>1024)){dialog.querySelector('.settings-error')!.textContent='화면 크기는 64~1024 정수로 입력하세요.';return;}
    p.settings={phoneNumber:phone||undefined,...(field('custom').checked?{width,height,fullFramebuffer:field('full').checked}:{})};save(p);dialog.close();restart();
  };
  action('apply').addEventListener('click',apply,opts);
  field('speed').addEventListener('input',()=>{const value=Number(field('speed').value);localStorage.setItem('gomul-wasm-speed',String(value));dialog.querySelector('output')!.value=(value/1000)+'×';command('speed',value);},opts);
  action('audio').addEventListener('click',()=>{dialog.close();volume.open();},opts);
  action('fresh').addEventListener('click',()=>{const p=preferences(id);p.previous.push(p.namespace);p.namespace=id+':save:'+crypto.randomUUID();save(p);dialog.close();restart();},opts);
  action('previous').addEventListener('click',()=>{const p=preferences(id),old=p.previous.pop();if(!old)return;p.previous.push(p.namespace);p.namespace=old;save(p);dialog.close();restart();},opts);
  const open=()=>{release();field('custom').checked=launch.settings.width!==undefined;field('width').value=String(launch.settings.width??240);field('height').value=String(launch.settings.height??320);field('full').checked=!!launch.settings.fullFramebuffer;field('phone').value=launch.settings.phoneNumber||launch.profile?.phoneNumber||'';field('speed').value=String(readLaunch(id).speed);dialog.querySelector('output')!.value=(Number(field('speed').value)/1000)+'×';action('previous').disabled=!preferences(id).previous.length;dialog.showModal();};
  return {open,dispose(){abort.abort();dialog.close();dialog.remove();for(const button of document.querySelectorAll<HTMLButtonElement>('#player-view button[data-key]'))if(labels[button.dataset.key!])button.textContent=button.dataset.key!;}};
}
