import {WieWeb,prepareLaunch} from '@pkg';
import {flushBrowserWrites} from './indexed_db_store';
let emulator:WieWeb|undefined;
let timer:ReturnType<typeof setTimeout>|undefined;
let paused=false;
function send(message:unknown):void {(globalThis as unknown as {postMessage(message:unknown):void}).postMessage(message);}
function fail(error:unknown):void {if(timer!==undefined)clearTimeout(timer);send({type:'error',error:String(error)});}
function tick():void {
  if(!emulator||paused)return;
  try {
    emulator.update();
    if(emulator.has_exited()){send({type:'exit'});return;}
    // Host event-loop yield only; the backend retains its existing 8 ms slice.
    timer=setTimeout(tick,0);
  }catch(error){fail(error);}
}
globalThis.onmessage=async(event:MessageEvent)=>{
  const m=event.data;
  try {
    if(m.type==='start'){
      const profile=JSON.parse(await prepareLaunch(m.filename,m.archive,m.launch.namespace,JSON.stringify(m.launch.settings)));
      const settings={...m.launch.settings,phoneNumber:profile.phoneNumber};
      emulator=new WieWeb(m.filename,m.archive,undefined,m.font,m.launch.namespace,JSON.stringify(settings));
      emulator.set_speed(m.launch.speed);
      send({type:'ready',profile});tick();
    }else if(m.type==='key'&&emulator){
      if(m.down)emulator.key_down(m.key);else emulator.key_up(m.key);
      send({type:'input-exposed',id:m.id,time:performance.timeOrigin+performance.now()});
    }else if(m.type==='speed'&&emulator){emulator.set_speed(m.value);
    }else if(m.type==='korean'&&emulator){emulator.set_text_input_mode(m.value);
    }else if(m.type==='pause'){
      paused=m.paused;if(timer!==undefined)clearTimeout(timer);if(!paused)tick();
    }else if(m.type==='stop'){
      if(timer!==undefined)clearTimeout(timer);emulator?.free();emulator=undefined;await flushBrowserWrites();send({type:'stopped'});
    }
  }catch(error){fail(error);}
};
// Webpack initializes WASM asynchronously. The page must not send the archive
// until this handler exists, or the first message can be lost during startup.
send({type:'initialized'});

globalThis.onunhandledrejection=event=>{event.preventDefault();fail(event.reason);};
