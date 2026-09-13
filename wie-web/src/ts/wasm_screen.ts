/** Presentation bridge: DOM on the page, copied frame messages in the CPU worker. */
export class ScreenTarget {
  private w = 240;
  private h = 320;
  private context?: CanvasRenderingContext2D;
  constructor(private canvas?: HTMLCanvasElement | null) {
    if (canvas) { this.w=canvas.width; this.h=canvas.height; this.context=canvas.getContext('2d')!; }
  }
  width():number { return this.w; }
  height():number { return this.h; }
  resize(width:number,height:number):void {
    this.w=width;this.h=height;
    if(this.canvas){this.canvas.width=width;this.canvas.height=height;}
  }
  paint(bytes:Uint8Array):void {
    const rgba=new Uint8ClampedArray(bytes);
    if(this.context)this.context.putImageData(new ImageData(rgba,this.w,this.h),0,0);
    else (globalThis as unknown as {postMessage(data:unknown,transfer:Transferable[]):void}).postMessage(
      {type:'frame',width:this.w,height:this.h,rgba},[rgba.buffer]);
  }
}
