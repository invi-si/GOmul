import type {CatalogGame} from './catalog';
import {exportBrowserBackup,importBrowserBackup} from './wasm-save-backup';

export function installSaveBackup(getGames:()=>CatalogGame[],canOpen:()=>boolean):void {
  const dialog=document.createElement('dialog');dialog.className='app-dialog native-settings save-backup';
  dialog.innerHTML=`<form method="dialog"><header class="dialog-header"><h2>브라우저 저장 백업</h2><button aria-label="닫기">×</button></header>
    <p>게임을 종료한 뒤 이용하세요. 게임 안에서 저장한 데이터와 화면·전화번호 설정을 백업합니다. 현재 장면을 저장하는 퀵세이브는 아닙니다.</p>
    <label>게임 <select name="game"></select></label>
    <div class="native-actions"><button type="button" data-export>백업 파일 저장</button><button type="button" data-import>백업 파일 불러오기</button><input type="file" accept=".json,application/json" hidden></div>
    <p>불러온 저장은 새 저장 공간에 적용합니다. 기존 진행 상황은 유지되며 게임 설정의 ‘이전 저장으로 되돌리기’로 돌아갈 수 있습니다. 파일은 서버로 전송하지 않습니다.</p>
    <p role="status"></p></form>`;
  document.body.append(dialog);
  const select=dialog.querySelector('select')!,file=dialog.querySelector<HTMLInputElement>('input[type=file]')!;
  const button=dialog.querySelector<HTMLButtonElement>('[data-export]')!,status=dialog.querySelector('[role=status]')!;
  const importButton=dialog.querySelector<HTMLButtonElement>('[data-import]')!;
  importButton.onclick=()=>file.click();
  let working=false;
  async function run(task:(game:string)=>Promise<void>){
    if(working||!select.value)return;
    working=true;select.disabled=true;button.disabled=true;file.disabled=true;importButton.disabled=true;status.textContent='처리 중…';
    try{await task(select.value);}catch(error){status.textContent=error instanceof Error?error.message:String(error);}
    finally{working=false;select.disabled=false;button.disabled=false;file.disabled=false;importButton.disabled=false;file.value='';}
  }
  button.onclick=()=>void run(async game=>{
    const blob=await exportBrowserBackup(game),url=URL.createObjectURL(blob),link=document.createElement('a');
    link.href=url;link.download='GOmul-'+game+'.save.json';document.body.append(link);link.click();link.remove();
    setTimeout(()=>URL.revokeObjectURL(url),30000);status.textContent='백업 파일을 저장했습니다.';
  });
  file.onchange=()=>{const selected=file.files?.[0];if(selected)void run(async game=>{
    await importBrowserBackup(game,selected);status.textContent='저장을 불러왔습니다. 게임을 열면 적용됩니다.';
  });};
  document.getElementById('catalog-save-backup')!.onclick=()=>{
    if(!canOpen()||working)return;
    select.replaceChildren(...getGames().map(game=>{const option=document.createElement('option');option.value=game.carrier+'-'+game.id;option.textContent=game.title+' · '+game.carrier;return option;}));
    status.textContent='';dialog.showModal();
  };
}
