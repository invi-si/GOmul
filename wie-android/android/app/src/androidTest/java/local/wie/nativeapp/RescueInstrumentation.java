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
 @Override public void onStart(){if(arguments!=null&&arguments.containsKey("rescue")){reproduce();return;}Bundle result=new Bundle();File root=new File(getTargetContext().getCacheDir(),"rescue-instrumentation");
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
 private void reproduce(){Bundle result=new Bundle();File root=new File(getTargetContext().getCacheDir(),"rescue-reproduction");
  try{
   remove(root);File report=new File(arguments.getString("rescue"));File game=new File(arguments.getString("game"));
   byte[] expected=Files.readAllBytes(new File(report,"archive-id").toPath());
   if(!java.util.Arrays.equals(expected,java.security.MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(game.toPath()))))throw new IOException("Game digest mismatch");
   File save=new File(root,"saves/case");copy(new File(report,"initial"),save);
   File slot=new File(root,"checkpoints/case/quick");slot.mkdirs();copy(new File(report,"initial"),new File(slot,"initial"));new File(slot,"expected").mkdirs();
   for(String name:new String[]{"trace","frame","archive-id","build-id"})Files.copy(new File(report,name).toPath(),new File(slot,name).toPath());
   Files.write(new File(slot,"format").toPath(),"GOmul replay alpha4 v1\0".getBytes(java.nio.charset.StandardCharsets.UTF_8));
   NativeBridge.start(game.getAbsolutePath(),save.getAbsolutePath());NativeBridge.pause(true);
   String expectedError=new String(Files.readAllBytes(new File(report,"error.txt").toPath()),java.nio.charset.StandardCharsets.UTF_8);
   // Strip only UI wrapping; compare the complete native error including registers and stack.
   if(expectedError.startsWith("Error: "))expectedError=expectedError.substring(7);
   String suffix=". Quick Load available.";
   if(expectedError.endsWith(suffix))expectedError=expectedError.substring(0,expectedError.length()-suffix.length());
   String outcome;
   boolean reproduced=false;
   try{outcome=NativeBridge.checkpoint("load");}catch(Exception failure){outcome=failure.getMessage();reproduced=expectedError.equals(outcome);}
   Files.write(new File(root,"outcome.txt").toPath(),outcome.getBytes(java.nio.charset.StandardCharsets.UTF_8));
   result.putString("stream",(reproduced?"Exact recorded failure reproduced: PASS\n":"Recorded failure did not match: FAIL\n")+outcome+"\n");finish(reproduced?0:1,result);
  }catch(Throwable e){result.putString("stream",android.util.Log.getStackTraceString(e));finish(1,result);}finally{NativeBridge.stop();}
 }
 private static void copy(File source,File destination)throws IOException{
  destination.mkdirs();File[] files=source.listFiles();if(files==null)throw new IOException("Missing fixture data");for(File f:files){File d=new File(destination,f.getName());if(f.isDirectory())copy(f,d);else Files.copy(f.toPath(),d.toPath());}
 }
 private static void remove(File f){File[] children=f.listFiles();if(children!=null)for(File c:children)remove(c);f.delete();}
}
