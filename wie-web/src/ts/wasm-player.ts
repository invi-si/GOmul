import {readLaunch,type LaunchProfile} from './wasm-settings';
import type { AppMetadata } from './app_library_store';
import type { SettingsController } from './settings';
import { runWorkerApp } from './worker-app';
import { withGameSaveLock } from './browser_save_store';

let font: Promise<Uint8Array> | undefined;
export async function runWasmApp(app: AppMetadata, selection: {game:string; index:number}, settings: SettingsController): Promise<void> {
  const response = await fetch(`/api/game/${selection.game}/rom/${selection.index}`);
  if (!response.ok) throw new Error(`게임 파일을 읽을 수 없습니다 (${response.status}).`);
  const archive = new Uint8Array(await response.arrayBuffer());
  font ??= fetch(new URL('../../../assets/neodgm.ttf', import.meta.url)).then(async response => {
    if (!response.ok) throw new Error('글꼴을 읽을 수 없습니다.');
    return new Uint8Array(await response.arrayBuffer());
  }).catch(error => { font = undefined; throw error; });
  const fontData = await font;
  const run = new URLSearchParams(location.search).get('runtime') === 'main'
    ? (await import('./app')).runApp : runWorkerApp;
  let restarting:boolean;
  do {
  restarting=false;
  await withGameSaveLock(app.id, async () => {
  const launch=readLaunch(app.id);
  if(run!==runWorkerApp){const {prepareLaunch}=await import('@pkg');const profile:LaunchProfile=JSON.parse(await prepareLaunch(app.filename,archive,launch.namespace,JSON.stringify(launch.settings)));launch.profile=profile;launch.settings.phoneNumber=profile.phoneNumber;}
  return new Promise<void>((resolve, reject) => {
    const library = document.getElementById('library-view')!;
    const player = document.getElementById('player-view')!;
    const loading = document.getElementById('catalog-loading')!;
    const showPlayer=()=>{
      library.hidden=true;player.hidden=false;loading.hidden=true;
      const canvas=document.getElementById('canvas')!;
      canvas.tabIndex=0;canvas.focus({preventScroll:true});
    };
    // Keep unfinished native-only actions explicit during the WASM baseline stage.
    const unavailable = ['native-save','native-load'];
    for (const id of unavailable) {
      const button = document.getElementById(id) as HTMLButtonElement | null;
      if (button) { button.disabled = true; button.title = '브라우저 버전 준비 중'; }
    }
    const status = document.getElementById('native-status');
    if (status) status.textContent = '';
    let dispose: (() => void | Promise<void>) | undefined;
    let closing=false;
    const exit = (error?: unknown) => {
      if(closing)return;closing=true;
      void Promise.resolve().then(()=>dispose?.()).then(()=>{
        player.hidden=true;library.hidden=false;
        if(error!==undefined)reject(error);else resolve();
      },stopError=>{player.hidden=true;library.hidden=false;reject(error??stopError);});
    };
    try {
      dispose = run(app, archive, fontData, settings, exit, showPlayer, launch, ()=>{restarting=true;exit();});
      if(run!==runWorkerApp)showPlayer();
    }
    catch (error) { exit(error); }
  });
  });
  }while(restarting);
}
