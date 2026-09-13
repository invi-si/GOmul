package local.wie.nativeapp;

import android.app.Instrumentation;
import android.media.AudioAttributes;
import android.media.MediaPlayer;
import android.os.Bundle;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import org.json.JSONObject;

/** Decoder smoke test for a privately captured MIDI/WAV file, not an audibility claim. */
final class AudioPlaybackInstrumentation {
 static void run(Instrumentation instrumentation,Bundle arguments){
  Bundle result=new Bundle();MediaPlayer player=new MediaPlayer();
  try{
   CountDownLatch ready=new CountDownLatch(1);AtomicReference<String> error=new AtomicReference<>();
   player.setAudioAttributes(new AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_GAME).setContentType(AudioAttributes.CONTENT_TYPE_MUSIC).build());
   player.setOnPreparedListener(p->ready.countDown());
   player.setOnErrorListener((p,what,extra)->{error.set(what+":"+extra);ready.countDown();return true;});
   player.setDataSource(arguments.getString("audioFile"));player.prepareAsync();
   if(!ready.await(10,TimeUnit.SECONDS))throw new IllegalStateException("Audio prepare timed out");
   if(error.get()!=null)throw new IllegalStateException("Audio decoder error "+error.get());
   player.setVolume(1f,1f);player.start();Thread.sleep(2000);
   JSONObject report=new JSONObject().put("durationMs",player.getDuration()).put("positionMs",player.getCurrentPosition()).put("playing",player.isPlaying());
   if(error.get()!=null)throw new IllegalStateException("Audio playback error "+error.get());
   result.putString("stream",report.toString()+"\n");instrumentation.finish(0,result);
  }catch(Exception error){result.putString("stream",error.toString()+"\n");instrumentation.finish(1,result);}
  finally{player.release();}
 }
}
