package local.wie.nativeapp;

import android.app.Instrumentation;
import android.os.Bundle;

/** Verify conversion between audio's guest timeline and Handler deadlines. */
final class AudioTimingInstrumentation {
 private static Object field(Object object,String name)throws Exception{var f=object.getClass().getDeclaredField(name);f.setAccessible(true);return f.get(object);}
 private static int[] state(AudioOutput output)throws Exception{
  var ready=new java.util.concurrent.CountDownLatch(1);var result=new java.util.concurrent.atomic.AtomicReference<int[]>();var error=new java.util.concurrent.atomic.AtomicReference<Throwable>();
  ((android.os.Handler)field(output,"handler")).post(()->{try{
   var groups=(java.util.Map<?,?>)field(output,"groups");int position=-1,playing=0;
   if(!groups.isEmpty()){
    var players=(java.util.List<?>)field(groups.values().iterator().next(),"players");
    if(!players.isEmpty()){var p=(android.media.MediaPlayer)players.get(0);try{position=p.getCurrentPosition();playing=p.isPlaying()?1:0;}catch(IllegalStateException preparing){}}
   }
   result.set(new int[]{groups.size(),position,playing});
  }catch(Throwable e){error.set(e);}finally{ready.countDown();}});
  if(!ready.await(5,java.util.concurrent.TimeUnit.SECONDS))throw new AssertionError("Audio worker unresponsive");
  if(error.get()!=null)throw new AssertionError(error.get());return result.get();
 }
 private static void playbackLifecycle(Instrumentation runner)throws Exception{
  var root=new java.io.File(runner.getTargetContext().getCacheDir(),"audio-lifecycle-test");root.mkdirs();var output=new AudioOutput(root);boolean closed=false;
  try{
   // Silent PCM verifies decoder/worker state without playing a test tone.
   int data=160000;var wav=java.nio.ByteBuffer.allocate(44+data).order(java.nio.ByteOrder.LITTLE_ENDIAN);
   wav.put("RIFF".getBytes(java.nio.charset.StandardCharsets.US_ASCII)).putInt(36+data).put("WAVEfmt ".getBytes(java.nio.charset.StandardCharsets.US_ASCII));
   wav.putInt(16).putShort((short)1).putShort((short)1).putInt(8000).putInt(16000).putShort((short)2).putShort((short)16).put("data".getBytes(java.nio.charset.StandardCharsets.US_ASCII)).putInt(data);
   var packet=java.nio.ByteBuffer.allocate(31+wav.capacity()).order(java.nio.ByteOrder.LITTLE_ENDIAN);
   packet.put((byte)1).putInt(7).putLong(10000).put((byte)0).putInt(1).putLong(0).put((byte)1).putInt(wav.capacity()).put(wav.array());output.submit(packet.array());
   long deadline=android.os.SystemClock.elapsedRealtime()+10000;int[] current;
   do{Thread.sleep(40);current=state(output);}while((current[1]<100||current[2]!=1)&&android.os.SystemClock.elapsedRealtime()<deadline);
   if(current[1]<100||current[2]!=1)throw new AssertionError("PCM did not start");
   output.pause(true);int before=state(output)[1];Thread.sleep(300);current=state(output);
   if(current[2]!=0||Math.abs(current[1]-before)>40)throw new AssertionError("Audio advanced during pause");
   output.speed(2000);before=state(output)[1];Thread.sleep(300);current=state(output);
   if(current[2]!=0||Math.abs(current[1]-before)>40)throw new AssertionError("Changing speed resumed paused audio");
   output.pause(false);Thread.sleep(300);current=state(output);
   if(current[2]!=1||current[1]<=before+100)throw new AssertionError("Audio did not resume");
   output.stopAll();if(state(output)[0]!=0)throw new AssertionError("Audio groups survived stop");
   java.io.File[] files=new java.io.File(root,"native-audio").listFiles();if(files==null||files.length!=0)throw new AssertionError("Audio temporary files leaked");
   output.close();closed=true;var thread=(Thread)field(output,"thread");thread.join(5000);if(thread.isAlive())throw new AssertionError("Audio thread survived close");
  }finally{if(!closed)output.close();GameDataImport.removeTree(root);}
 }
 static void run(Instrumentation runner){Bundle result=new Bundle();try{
  for(int rate:new int[]{250,500,1000,1250,1500,1750,2000,2250,2500,2750,3000}){
   if(AudioOutput.guestElapsed(1000,rate)!=rate)throw new AssertionError("elapsed "+rate);
   if(AudioOutput.hostDelay(rate,rate)!=1000)throw new AssertionError("deadline "+rate);
   if(AudioOutput.hostDelay(0,rate)!=0)throw new AssertionError("zero deadline");
   for(long delay:new long[]{1,17,999,2000}){
    long host=AudioOutput.hostDelay(delay,rate);
    if(AudioOutput.guestElapsed(host,rate)<delay)throw new AssertionError("early audio deadline");
   }
  }
  playbackLifecycle(runner);
  result.putString("stream","Audio timing, PCM playback, pause/resume, paused speed change, stop and resource cleanup: PASS\n");runner.finish(0,result);
 }catch(Throwable e){result.putString("stream",android.util.Log.getStackTraceString(e));runner.finish(1,result);}}
}
