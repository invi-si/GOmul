package local.wie.nativeapp;

import android.app.Instrumentation;
import android.os.Bundle;
import android.graphics.BitmapFactory;
import java.io.*;
import java.nio.*;
import java.nio.file.Files;
import java.util.zip.*;

/** Synthetic export checks, or explicit developer replay using isolated copies of rescue saves. */
public final class RescueInstrumentation extends Instrumentation {
 private Bundle arguments;
 @Override public void onCreate(Bundle args){super.onCreate(args);arguments=args;start();}
 @Override public void onStart(){if(arguments!=null&&"audio-playback".equals(arguments.getString("mode"))){AudioPlaybackInstrumentation.run(this,arguments);return;}if(arguments!=null&&"launch-data".equals(arguments.getString("mode"))){LaunchDataInstrumentation.run(this);return;}if(arguments!=null&&"manual-rescue".equals(arguments.getString("mode"))){manualRescue();return;}if(arguments!=null&&"thor-controls".equals(arguments.getString("mode"))){ThorInstrumentation.run(this);return;}if(arguments!=null&&"compatibility".equals(arguments.getString("mode"))){CompatibilityInstrumentation.run(this,arguments);return;}if(arguments!=null&&"audio-timing".equals(arguments.getString("mode"))){AudioTimingInstrumentation.run(this);return;}if(arguments!=null&&arguments.containsKey("rescue")){reproduce();return;}Bundle result=new Bundle();File root=new File(getTargetContext().getCacheDir(),"rescue-instrumentation");
  try{
   remove(root);root.mkdirs();File fixture=new File(root,"fixture");fixture.mkdirs();
   Files.write(new File(fixture,"report.txt").toPath(),"synthetic rescue".getBytes());
   ByteBuffer frame=ByteBuffer.allocate(25).order(ByteOrder.LITTLE_ENDIAN);frame.putInt(2).putInt(1).putLong(1).put((byte)0).putInt(0xff336699).putInt(0xffcc2200);
   Files.write(new File(fixture,"frame").toPath(),frame.array());File exported=new File(root,"report.zip");
   RescueReports.export(fixture,new FileOutputStream(exported));
   try(ZipFile zip=new ZipFile(exported)){
    if(zip.getEntry("report.txt")==null)throw new AssertionError("Missing report");
    try(InputStream png=zip.getInputStream(zip.getEntry("screenshot.png"))){android.graphics.Bitmap b=BitmapFactory.decodeStream(png);if(b==null||b.getWidth()!=2||b.getHeight()!=1||b.getPixel(0,0)!=0xff336699||b.getPixel(1,0)!=0xffcc2200)throw new AssertionError("Incorrect rescue screenshot");b.recycle();}
   }
   // Real native worker failure from a synthetic malformed JAR.
   File bad=new File(root,"broken.jar");Files.write(bad.toPath(),new byte[]{1,2,3});
   File save=new File(root,"saves/synthetic");save.mkdirs();NativeBridge.start(bad.getAbsolutePath(),save.getAbsolutePath());
   long deadline=android.os.SystemClock.elapsedRealtime()+15000;String status;
   do{status=NativeBridge.status();if(status.startsWith("Error:"))break;Thread.sleep(50);}while(android.os.SystemClock.elapsedRealtime()<deadline);
   if(!status.startsWith("Error:"))throw new AssertionError("Expected failure: "+status);
   File captured=new File(root,"rescues/synthetic/latest");
   if(!new File(captured,"error.txt").isFile()||!new File(captured,"safe-trace").isFile())throw new AssertionError("Native failure was not rescued");
   if(new File(captured,"broken.jar").exists())throw new AssertionError("Game archive leaked into report");
   NativeBridge.stop();
   // A deliberate stop must not create another report.
   if(new File(root,"rescues/synthetic/previous").exists())throw new AssertionError("Stop generated duplicate rescue");
   result.putString("stream","Rescue export PNG, native failure capture, archive exclusion and intentional stop: PASS\n");finish(0,result);
  }catch(Throwable e){result.putString("stream",android.util.Log.getStackTraceString(e));finish(1,result);}
  finally{NativeBridge.stop();remove(root);}
 }
 private void manualRescue(){
  Bundle result=new Bundle();File root=new File(getTargetContext().getCacheDir(),"manual-rescue-instrumentation");
  try{
   remove(root);File save=new File(root,"saves/test-game");save.mkdirs();
   NativeBridge.start(arguments.getString("game"),save.getAbsolutePath());Thread.sleep(5000);
   if(!"Running".equals(NativeBridge.status()))throw new AssertionError("Expected non-crashed game: "+NativeBridge.status());
   NativeBridge.pause(true);Thread.sleep(200);
   NativeBridge.checkpoint("save");File quick=new File(root,"checkpoints/test-game/quick/trace");byte[] originalQuick=Files.readAllBytes(quick.toPath());
   String status=NativeBridge.status();long paints=NativeBridge.paints();
   File captured=new File(NativeBridge.manualRescue());
   if(!captured.getCanonicalPath().startsWith(new File(root,"rescues/test-game/manual").getCanonicalPath()+File.separator))throw new AssertionError("Wrong game/report path");
   for(String name:new String[]{"report.txt","trace","safe-trace","frame","archive-id","build-id","recording-status.txt"})if(!new File(captured,name).isFile())throw new AssertionError("Missing "+name);
   if(!new String(Files.readAllBytes(new File(captured,"error.txt").toPath()),java.nio.charset.StandardCharsets.UTF_8).contains("Manual"))throw new AssertionError("Manual capture not labeled");
   if(!status.equals(NativeBridge.status())||paints!=NativeBridge.paints())throw new AssertionError("Manual rescue changed paused game");
   if(!java.util.Arrays.equals(originalQuick,Files.readAllBytes(quick.toPath())))throw new AssertionError("Quick Save changed");
   File exported=new File(root,"manual.zip");RescueReports.export(captured,new FileOutputStream(exported));
   try(ZipFile zip=new ZipFile(exported)){
    for(String name:new String[]{"report.txt","trace","safe-trace","archive-id","screenshot.png"})if(zip.getEntry(name)==null)throw new AssertionError("ZIP missing "+name);
    try(InputStream png=zip.getInputStream(zip.getEntry("screenshot.png"))){android.graphics.Bitmap b=BitmapFactory.decodeStream(png);if(b==null||b.getWidth()<1||b.getHeight()<1)throw new AssertionError("Invalid screenshot");b.recycle();}
   }
   NativeBridge.manualRescue();
   if(!new File(captured.getParentFile(),"previous/report.txt").isFile())throw new AssertionError("Repeat capture missing");
   if(new File(root,"rescues/test-game/latest").exists())throw new AssertionError("Manual capture masqueraded as automatic failure");
   if(!captured.equals(RescueReports.latest(root,"test-game")))throw new AssertionError("Library cannot find manual capture");
   result.putString("stream","Manual rescue without crash, paused session unchanged, per-game report, Quick Save preserved, repeat capture and ZIP screenshot: PASS\n");finish(0,result);
  }catch(Throwable e){result.putString("stream",android.util.Log.getStackTraceString(e));finish(1,result);}
  finally{NativeBridge.stop();remove(root);}
 }
 private void reproduce(){Bundle result=new Bundle();File root=new File(getTargetContext().getCacheDir(),"rescue-reproduction");
  try{
   if(arguments.containsKey("diagnosticFilter"))android.system.Os.setenv("GOMUL_DIAGNOSTIC_FILTER",arguments.getString("diagnosticFilter"),true);
   remove(root);File report=new File(arguments.getString("rescue"));File game=new File(arguments.getString("game"));
   byte[] expected=Files.readAllBytes(new File(report,"archive-id").toPath());
   if(!java.util.Arrays.equals(expected,java.security.MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(game.toPath()))))throw new IOException("Game digest mismatch");
   File save=new File(root,"saves/case");copy(new File(report,"initial"),save);
   File slot=new File(root,"checkpoints/case/quick");slot.mkdirs();copy(new File(report,"initial"),new File(slot,"initial"));new File(slot,"expected").mkdirs();
   String mode=arguments.getString("mode", "load");
   boolean fresh=mode.equals("fresh-start");
   boolean diagnostic=fresh||mode.equals("rescue-capture")||mode.equals("rescue-verify");
   for(String name:new String[]{"trace","frame","archive-id","build-id"})Files.copy(new File(report,diagnostic&&name.equals("trace")?"safe-trace":name).toPath(),new File(slot,name).toPath());
   if(mode.equals("rescue-verify")){
    File reference=new File(arguments.getString("reference"));
    Files.copy(new File(reference,"frame").toPath(),new File(slot,"frame").toPath(),java.nio.file.StandardCopyOption.REPLACE_EXISTING);
    copy(new File(reference,"expected"),new File(slot,"expected"));
   }
   Files.write(new File(slot,"format").toPath(),"GOmul replay alpha4 v1\0".getBytes(java.nio.charset.StandardCharsets.UTF_8));
   NativeBridge.start(game.getAbsolutePath(),save.getAbsolutePath());NativeBridge.pause(true);
   String expectedError=new String(Files.readAllBytes(new File(report,"error.txt").toPath()),java.nio.charset.StandardCharsets.UTF_8);
   // Strip only UI wrapping; compare the complete native error including registers and stack.
   if(expectedError.startsWith("Error: "))expectedError=expectedError.substring(7);
   String suffix=". Quick Load available.";
   if(expectedError.endsWith(suffix))expectedError=expectedError.substring(0,expectedError.length()-suffix.length());
   String outcome;
   boolean reproduced=false;
   try{outcome=fresh?"Fresh start from isolated initial saves":NativeBridge.checkpoint(mode);reproduced=diagnostic;}catch(Exception failure){outcome=failure.getMessage();reproduced=!diagnostic&&expectedError.equals(outcome);}
   if(diagnostic&&reproduced&&arguments.containsKey("runMs")){
    boolean inputTrace="true".equals(arguments.getString("inputTrace"));
    if(inputTrace)NativeBridge.traceControl(true,"");
    NativeBridge.speed(Integer.parseInt(arguments.getString("speed", "1000")));
    NativeBridge.pause(false);Thread.sleep(Long.parseLong(arguments.getString("runMs")));
    if(arguments.containsKey("keys")){long traceId=0;for(String key:arguments.getString("keys").split(",")){if(key.startsWith("WAIT:")){long delay=Long.parseLong(key.substring(5));if(delay<0||delay>30000)throw new IllegalArgumentException("Wait must be 0–30000 ms");Thread.sleep(delay);continue;}long now=System.nanoTime();NativeBridge.key(key,true,++traceId,now,now);Thread.sleep(100);now=System.nanoTime();NativeBridge.key(key,false,++traceId,now,now);Thread.sleep(300);}Thread.sleep(Long.parseLong(arguments.getString("settleMs", "1000")));}
    NativeBridge.pause(true);
    if(inputTrace)NativeBridge.traceControl(false,new File(root,"input-trace.csv").getAbsolutePath());
    int[] pixels=new int[1024*1024];long shape=NativeBridge.frame(pixels,new long[4]);
    if(shape!=0){int width=(int)(shape>>>32),height=(int)shape;android.graphics.Bitmap bitmap=android.graphics.Bitmap.createBitmap(pixels,width,height,android.graphics.Bitmap.Config.ARGB_8888);try(OutputStream png=new FileOutputStream(new File(root,"continuation.png"))){bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG,100,png);}bitmap.recycle();}
    outcome += "\nContinuation: "+NativeBridge.status()+"; paints="+NativeBridge.paints();
    reproduced = !NativeBridge.status().startsWith("Error:");
    if(reproduced&&"true".equals(arguments.getString("checkpointRoundTrip"))){
     NativeBridge.checkpoint("save");
     NativeBridge.speed(1000);
     outcome += "\nCheckpoint round trip after speed change: "+NativeBridge.checkpoint("load");
    }
   }
   Files.write(new File(root,"outcome.txt").toPath(),outcome.getBytes(java.nio.charset.StandardCharsets.UTF_8));
   result.putString("stream",(reproduced?(diagnostic?"Rescue diagnostic replay: PASS\n":"Exact recorded failure reproduced: PASS\n"):"Rescue reproduction/validation: FAIL\n")+outcome+"\n");finish(reproduced?0:1,result);
  }catch(Throwable e){result.putString("stream",android.util.Log.getStackTraceString(e));finish(1,result);}finally{NativeBridge.stop();}
 }
 private static void copy(File source,File destination)throws IOException{
  destination.mkdirs();File[] files=source.listFiles();if(files==null)throw new IOException("Missing fixture data");for(File f:files){File d=new File(destination,f.getName());if(f.isDirectory())copy(f,d);else Files.copy(f.toPath(),d.toPath());}
 }
 private static void remove(File f){File[] children=f.listFiles();if(children!=null)for(File c:children)remove(c);f.delete();}
}
