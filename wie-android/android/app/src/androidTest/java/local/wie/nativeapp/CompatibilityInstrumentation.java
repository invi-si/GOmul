package local.wie.nativeapp;

import android.app.Instrumentation;
import android.os.Bundle;
import android.os.SystemClock;
import org.json.JSONArray;
import org.json.JSONObject;
import java.io.File;
import java.nio.file.Files;

/** Bounded smoke sequences. Only app-cache saves are used; no gameplay-pass claim. */
public final class CompatibilityInstrumentation {
 private static void step(String key,int holdMs)throws Exception {
  if(key.startsWith("WAIT:")){
   int delay=Integer.parseInt(key.substring(5));
   if(delay<0||delay>30000)throw new IllegalArgumentException("Wait must be 0–30000 ms");
   Thread.sleep(delay);
  }else if(key.startsWith("SPEED:")){
   int speed=Integer.parseInt(key.substring(6));
   if(speed<250||speed>2000)throw new IllegalArgumentException("Speed must be 250–2000");
   NativeBridge.speed(speed);
  }else{
   String[] chord=key.split("\\+");
   for(String button:chord)NativeBridge.key(button,true,0,0,0);
   Thread.sleep(holdMs);
   for(String button:chord)NativeBridge.key(button,false,0,0,0);
   Thread.sleep(400);
  }
 }
 private static int sampleNumber,frameWidth,frameHeight,audioNumber;
 private static boolean collectAudio;
 private static int[] framePixels;
 private static void copySave(java.nio.file.Path sourceRoot,File save)throws Exception {
  try(java.util.stream.Stream<java.nio.file.Path> paths=Files.walk(sourceRoot)){
   for(java.nio.file.Path source:(Iterable<java.nio.file.Path>)paths::iterator){
    java.nio.file.Path target=save.toPath().resolve(sourceRoot.relativize(source));
    if(Files.isDirectory(source))Files.createDirectories(target);
    else Files.copy(source,target,java.nio.file.StandardCopyOption.REPLACE_EXISTING);
   }
  }
 }
 private static JSONObject sample(File root,String phase)throws Exception {
  JSONObject row=new JSONObject().put("phase",phase).put("elapsedMs",SystemClock.elapsedRealtime())
   .put("processCpuMs",android.os.Process.getElapsedCpuTime())
   .put("status",NativeBridge.status()).put("paints",NativeBridge.paints());
  long dimensions=NativeBridge.frame(framePixels,new long[2]);
  if(dimensions!=0){frameWidth=(int)(dimensions>>>32);frameHeight=(int)dimensions;}
  if(frameWidth>0&&frameHeight>0){
   int count=frameWidth*frameHeight,black=0,white=0;long hash=0xcbf29ce484222325L;
   for(int i=0;i<count;i++){int rgb=framePixels[i]&0xffffff,r=rgb>>>16,g=(rgb>>>8)&255,b=rgb&255;
    if(r<16&&g<16&&b<16)black++;if(r>239&&g>239&&b>239)white++;
    hash=(hash^rgb)*0x100000001b3L;
   }
   String name=String.format(java.util.Locale.ROOT,"sample-%03d.png",sampleNumber++);
   row.put("blackFraction",(double)black/count).put("whiteFraction",(double)white/count)
    .put("frameHash",Long.toHexString(hash)).put("image",name).put("width",frameWidth).put("height",frameHeight);
   android.graphics.Bitmap bitmap=android.graphics.Bitmap.createBitmap(framePixels,0,frameWidth,frameWidth,frameHeight,android.graphics.Bitmap.Config.ARGB_8888);
   try(java.io.FileOutputStream png=new java.io.FileOutputStream(new File(root,name))){bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG,100,png);}finally{bitmap.recycle();}
  }
  if(collectAudio){
   byte[] packet;int count=0;
   while((packet=NativeBridge.audio())!=null){
    Files.write(new File(root,String.format(java.util.Locale.ROOT,"audio-%03d.bin",audioNumber++)).toPath(),packet);count++;
   }
   row.put("audioPackets",count);
  }
  return row;
 }
 static void run(Instrumentation instrumentation,Bundle arguments){
  Bundle result=new Bundle();File root=new File(instrumentation.getTargetContext().getCacheDir(),"compatibility-audit");
  try{
   if(arguments.containsKey("diagnosticFilter"))android.system.Os.setenv("GOMUL_DIAGNOSTIC_FILTER",arguments.getString("diagnosticFilter"),true);
   GameDataImport.removeTree(root);root.mkdirs();
   collectAudio="true".equals(arguments.getString("collectAudio"));audioNumber=0;
   sampleNumber=frameWidth=frameHeight=0;framePixels=new int[1024*1024];
   Files.write(new File(root,"process-id").toPath(),Integer.toString(android.os.Process.myPid()).getBytes(java.nio.charset.StandardCharsets.US_ASCII));
   File save=new File(root,"saves/case");save.mkdirs();
   if(arguments.containsKey("initialSave"))copySave(new File(arguments.getString("initialSave")).toPath(),save);
   JSONArray samples=new JSONArray();
   if(arguments.containsKey("lifecycleRounds")){
    for(int round=0;round<Integer.parseInt(arguments.getString("lifecycleRounds"));round++){
     NativeBridge.start(arguments.getString("game"),save.getAbsolutePath());Thread.sleep(2500);
     int checkpointRounds=Integer.parseInt(arguments.getString("checkpointRounds","0"));
     if(checkpointRounds>0){NativeBridge.pause(true);NativeBridge.checkpoint("save");for(int i=0;i<checkpointRounds;i++)NativeBridge.checkpoint("load");}
     String running=NativeBridge.status();long paints=NativeBridge.paints();NativeBridge.stop();
     String memory=new String(Files.readAllBytes(new File("/proc/self/status").toPath()),java.nio.charset.StandardCharsets.US_ASCII);
     long maps;try(java.util.stream.Stream<String> lines=Files.lines(new File("/proc/self/maps").toPath())){maps=lines.count();}
     JSONObject row=new JSONObject().put("round",round).put("status",running).put("paints",paints).put("maps",maps).put("memory",memory);
     samples.put(row);Files.write(new File(root,"lifecycle.json").toPath(),samples.toString(2).getBytes(java.nio.charset.StandardCharsets.UTF_8));
    }
    result.putString("stream",samples.toString()+"\n");instrumentation.finish(0,result);return;
   }
   NativeBridge.start(arguments.getString("initialGame",arguments.getString("game")),save.getAbsolutePath());
   int bootSamples=Integer.parseInt(arguments.getString("bootSamples","1"));
   for(int i=0;i<bootSamples;i++){Thread.sleep(5000);samples.put(sample(root,"boot:"+i));}
   // Reproduce documented phone installs that copy companion data after first boot.
   // Everything remains confined to this test's isolated cache save.
   if(arguments.containsKey("stagedData")){
    for(String key:arguments.getString("initialKeys", "").split(",")){
     if(key.isEmpty())continue;step(key,100);samples.put(sample(root,"before-data-key:"+key));
    }
    NativeBridge.stop();
    java.nio.file.Path staged=new File(arguments.getString("stagedData")).toPath();
    copySave(staged,save);
    NativeBridge.start(arguments.getString("game"),save.getAbsolutePath());
    Thread.sleep(5000);samples.put(sample(root,"reopened-after-data-install"));
   }
   if("true".equals(arguments.getString("restartAfterStop"))){
    Thread.sleep(2500);samples.put(sample(root,"before-restart"));
    if(NativeBridge.status().startsWith("Game stopped")){
     NativeBridge.stop();
     File firstLog=new File(save.getParentFile(),"case.frames.log");
     if(firstLog.isFile())Files.copy(firstLog.toPath(),new File(root,"first-start.frames.log").toPath());
     NativeBridge.start(arguments.getString("game"),save.getAbsolutePath());
     Thread.sleep(5000);samples.put(sample(root,"restarted-with-same-save"));
    }
   }
   int[] pixels=new int[1024*1024];long dimensions=NativeBridge.frame(pixels,new long[2]);
   if(dimensions!=0){
    int width=(int)(dimensions>>>32),height=(int)dimensions;
    android.graphics.Bitmap bitmap=android.graphics.Bitmap.createBitmap(pixels,0,width,width,height,android.graphics.Bitmap.Config.ARGB_8888);
    try(java.io.FileOutputStream png=new java.io.FileOutputStream(new File(root,"boot.png"))){bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG,100,png);}finally{bitmap.recycle();}
   }
   int keyHoldMs=Integer.parseInt(arguments.getString("keyHoldMs","100"));
   if(keyHoldMs<1||keyHoldMs>5000)throw new IllegalArgumentException("keyHoldMs must be 1–5000 ms");
   for(String key:arguments.getString("keys","OK,DOWN,OK,1").split(",")){
    if(key.isEmpty())continue;
    if(NativeBridge.status().startsWith("Error:"))break;
    step(key,keyHoldMs);
    samples.put(sample(root,"key:"+key));
   }
   for(int i=0;i<3;i++){Thread.sleep(2000);samples.put(sample(root,"settled:"+i));}
   // Some games ask for confirmation before their first-run initialization exit.
   if("true".equals(arguments.getString("restartAfterStop"))&&NativeBridge.status().startsWith("Game stopped")){
    NativeBridge.stop();NativeBridge.start(arguments.getString("game"),save.getAbsolutePath());
    Thread.sleep(5000);samples.put(sample(root,"reopened-after-input-exit"));
    for(String key:arguments.getString("reopenedKeys",arguments.getString("keys","OK,DOWN,OK,1")).split(",")){
     if(key.isEmpty())continue;if(NativeBridge.status().startsWith("Error:"))break;
     step(key,keyHoldMs);
     samples.put(sample(root,"reopened-key:"+key));
    }
    for(int i=0;i<3;i++){Thread.sleep(2000);samples.put(sample(root,"settled:"+i));}
   }
   NativeBridge.pause(true);
   JSONObject report=new JSONObject().put("pid",android.os.Process.myPid()).put("samples",samples);
   // Persist measurement evidence even if optional rescue capture is unavailable.
   Files.write(new File(root,"result.json").toPath(),report.toString(2).getBytes(java.nio.charset.StandardCharsets.UTF_8));
   if("true".equals(arguments.getString("captureRescue"))){
    for(int attempt=0;attempt<3;attempt++){
     try{Thread.sleep(100);report.put("rescuePath",NativeBridge.manualRescue());report.remove("rescueCaptureError");break;}
     catch(Exception error){report.put("rescueCaptureError",error.toString());}
    }
   }
   Files.write(new File(root,"result.json").toPath(),report.toString(2).getBytes(java.nio.charset.StandardCharsets.UTF_8));
   result.putString("stream",report.toString()+"\n");instrumentation.finish(0,result);
  }catch(Throwable e){result.putString("stream",android.util.Log.getStackTraceString(e));instrumentation.finish(1,result);}
  finally{NativeBridge.stop();}
 }
}
