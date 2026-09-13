import {AudioPlayer as PageAudioPlayer,setPcmVolume as pageVolume} from 'midi';
type Events=Parameters<PageAudioPlayer['play']>[2];
const onPage=typeof document!=='undefined';
function send(data:unknown):void {(globalThis as unknown as {postMessage(data:unknown):void}).postMessage(data);}
export class AudioPlayer {
  private page=onPage?new PageAudioPlayer():undefined;
  play(handle:number,duration:number,events:Events,repeat:boolean):void {
    if(this.page)this.page.play(handle,duration,events,repeat);
    else send({type:'audio',op:'play',handle,duration,events,repeat});
  }
  stop(handle:number):void {if(this.page)this.page.stop(handle);else send({type:'audio',op:'stop',handle});}
  dispose():void {this.page?.dispose();}
}
export function setPcmVolume(volume:number):void {if(onPage)pageVolume(volume);else send({type:'volume',volume});}
