import { clientPlatform } from './client_platform';
import { initializeCatalog,type CatalogGame } from './catalog';
import {installSaveBackup} from './wasm-save-backup-ui';
import { initializeSettings } from './settings';
import { runWasmApp } from './wasm-player';
async function main() {
  document.body.dataset.client=clientPlatform(navigator);
  const settings=initializeSettings();
  document.getElementById('catalog-settings')!.onclick=()=>settings.open();
  const help=document.getElementById('help-dialog') as HTMLDialogElement;
  document.getElementById('catalog-help')!.onclick=()=>help.showModal();
  const report=document.getElementById('bug-report-dialog') as HTMLDialogElement;
  document.getElementById('catalog-bug-report')!.onclick=()=>report.showModal();
  let games:CatalogGame[]=[];
  installSaveBackup(()=>games,()=>document.getElementById('player-view')!.hidden===true&&document.getElementById('catalog-loading')!.hidden===true);
  await initializeCatalog((app,selection)=>runWasmApp(app,selection,settings),loaded=>{games=loaded;});
}
void main().catch(error=>{const status=document.getElementById('catalog-status')!;status.textContent=String(error);status.classList.add('error');});
