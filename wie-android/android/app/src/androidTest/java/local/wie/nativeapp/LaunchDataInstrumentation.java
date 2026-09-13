package local.wie.nativeapp;

import android.app.Instrumentation;
import android.content.Intent;
import android.os.Bundle;
import android.os.SystemClock;
import java.io.File;
import java.io.FileOutputStream;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.Map;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.zip.ZipEntry;
import java.util.zip.ZipOutputStream;

/** Exercise the real launcher against an unmarked legacy save, using only synthetic data. */
final class LaunchDataInstrumentation {
 static void run(Instrumentation test){
  Bundle result=new Bundle();MainActivity activity=null;
  String id="f".repeat(63)+"0";
  File root=new File(test.getTargetContext().getCacheDir(),"launch-data-test");
  File saves=new File(test.getTargetContext().getFilesDir(),"saves/"+id);
  File checkpoints=new File(test.getTargetContext().getFilesDir(),"checkpoints/"+id);
  File rescues=new File(test.getTargetContext().getFilesDir(),"rescues/"+id);
  boolean owned=false;
  try{
   if(saves.exists()||checkpoints.exists()||rescues.exists())throw new AssertionError("Synthetic test identity is already in use");
   owned=true;GameDataImport.removeTree(root);File game=new File(root,id+"/synthetic.zip");game.getParentFile().mkdirs();
   try(ZipOutputStream zip=new ZipOutputStream(new FileOutputStream(game))){
    for(Map.Entry<String,String> e:Map.of("app_info","PID:PDTEST\nAID:TEST\n","data/savedata","bundled start","gomul.properties","phoneNumber=01000000000\n").entrySet()){
     zip.putNextEntry(new ZipEntry(e.getKey()));zip.write(e.getValue().getBytes(StandardCharsets.UTF_8));zip.closeEntry();
    }
   }
   File record=new File(saves,"PDTEST/db/savedata/1");record.getParentFile().mkdirs();Files.write(record.toPath(),"earned progress".getBytes(StandardCharsets.UTF_8));
   activity=(MainActivity)test.startActivitySync(new Intent(test.getTargetContext(),MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
   MainActivity current=activity;Method launch=MainActivity.class.getDeclaredMethod("launch",File.class);launch.setAccessible(true);
   test.runOnMainSync(()->{try{launch.invoke(current,game);}catch(Exception e){throw new RuntimeException(e);}});
   var starting=MainActivity.class.getDeclaredField("starting");starting.setAccessible(true);AtomicBoolean busy=new AtomicBoolean(true);
   long deadline=SystemClock.elapsedRealtime()+15000;
   do{test.runOnMainSync(()->{try{busy.set(starting.getBoolean(current));}catch(Exception e){throw new RuntimeException(e);}});if(busy.get())Thread.sleep(50);}while(busy.get()&&SystemClock.elapsedRealtime()<deadline);
   if(busy.get())throw new AssertionError("Launcher did not finish startup");
   if(!new String(Files.readAllBytes(record.toPath()),StandardCharsets.UTF_8).equals("earned progress"))throw new AssertionError("Opening the game replaced existing progress");
   if(new File(saves,"companion-imported").exists())throw new AssertionError("Launch unexpectedly reimported companion data");
   result.putString("stream","Real launcher preserves unmarked existing progress when the archive contains startup data: PASS\n");
  }catch(Throwable e){result.putString("stream",android.util.Log.getStackTraceString(e));result.putBoolean("failed",true);}
  finally{
   if(activity!=null){MainActivity current=activity;test.runOnMainSync(current::finish);test.waitForIdleSync();}
   NativeBridge.stop();GameDataImport.removeTree(root);
   if(owned){GameDataImport.removeTree(saves);GameDataImport.removeTree(checkpoints);GameDataImport.removeTree(rescues);}
  }
  test.finish(result.getBoolean("failed")?1:0,result);
 }
}
