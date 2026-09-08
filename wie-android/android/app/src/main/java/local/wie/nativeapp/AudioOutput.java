package local.wie.nativeapp;
import android.media.MediaPlayer;
import android.media.AudioAttributes;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.SystemClock;
import android.util.Log;
import java.io.*;
import java.nio.*;
import java.util.*;

final class AudioOutput {
 private final HandlerThread thread=new HandlerThread("WIE audio");
 private final Handler handler;
 private final File directory;
 private final Map<Integer,Group> groups=new HashMap<>();
 private boolean paused;
 private static class Part {long delay;int kind;byte[] data;}
 private static class Group {int id;long duration,start,elapsed;boolean repeat;List<Part> parts=new ArrayList<>();List<MediaPlayer> players=new ArrayList<>();List<File> files=new ArrayList<>();}
 AudioOutput(File cache) {directory=new File(cache,"native-audio");directory.mkdirs();File[] old=directory.listFiles();if(old!=null)for(File f:old)f.delete();thread.start();handler=new Handler(thread.getLooper());}
 void submit(byte[] packet) {handler.post(()->{
  try {
   ByteBuffer b=ByteBuffer.wrap(packet).order(ByteOrder.LITTLE_ENDIAN);int command=b.get();int id=b.getInt();stop(id);if(command==0)return;
   Group g=new Group();g.id=id;g.duration=b.getLong();g.repeat=b.get()!=0;int count=b.getInt();
   for(int i=0;i<count;i++){Part p=new Part();p.delay=b.getLong();p.kind=b.get();p.data=new byte[b.getInt()];b.get(p.data);g.parts.add(p);}
   groups.put(id,g);schedule(g);
  }catch(Exception e){Log.e("WIE-Native","Audio command failed",e);}
 });}
 private void schedule(Group g) {
  long start=SystemClock.uptimeMillis()-g.elapsed;g.start=start;
  for(Part part:g.parts)if(part.delay>=g.elapsed)handler.postAtTime(()->{if(groups.get(g.id)!=g)return;play(g,part);},g,start+part.delay);
  if(g.duration>0)handler.postAtTime(()->{if(groups.get(g.id)==g){if(g.repeat){releasePlayers(g);g.elapsed=0;schedule(g);}else stop(g.id);}},g,start+g.duration);
 }
 private void play(Group g,Part part) {
  try {
   File f=File.createTempFile("sound-",part.kind==0?".mid":".wav",directory);try(FileOutputStream out=new FileOutputStream(f)){out.write(part.data);}g.files.add(f);
   MediaPlayer player=new MediaPlayer();g.players.add(player);player.setAudioAttributes(new AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_GAME).setContentType(AudioAttributes.CONTENT_TYPE_MUSIC).build());
   player.setDataSource(f.getAbsolutePath());player.setOnPreparedListener(p->{if(groups.get(g.id)==g&&!paused)p.start();});player.setOnErrorListener((p,a,b)->{Log.e("WIE-Native","Audio playback error "+a+":"+b);return true;});player.prepareAsync();
  }catch(Exception e){Log.e("WIE-Native","Audio playback failed",e);}
 }
 private void releasePlayers(Group g){for(MediaPlayer p:g.players)p.release();g.players.clear();for(File f:g.files)f.delete();g.files.clear();}
 private void stop(int id){Group g=groups.remove(id);if(g!=null){handler.removeCallbacksAndMessages(g);releasePlayers(g);}}
 void pause(boolean value){handler.post(()->{if(paused==value)return;paused=value;for(Group g:groups.values()){if(value){g.elapsed=SystemClock.uptimeMillis()-g.start;handler.removeCallbacksAndMessages(g);}else schedule(g);for(MediaPlayer p:g.players){try{if(value&&p.isPlaying())p.pause();else if(!value)p.start();}catch(IllegalStateException ignored){}}}});}
 void stopAll(){handler.post(()->{for(int id:new ArrayList<>(groups.keySet()))stop(id);});}
 void close(){handler.post(()->{for(int id:new ArrayList<>(groups.keySet()))stop(id);thread.quitSafely();});}
}
