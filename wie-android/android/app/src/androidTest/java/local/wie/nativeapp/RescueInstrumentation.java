package local.wie.nativeapp;

import android.app.Instrumentation;
import android.os.Bundle;
import android.graphics.BitmapFactory;
import java.io.*;
import java.nio.*;
import java.nio.file.Files;
import java.util.zip.*;

/** Uses synthetic data only; never opens or modifies the user's games or saves. */
public final class RescueInstrumentation extends Instrumentation {
 @Override public void onCreate(Bundle args){super.onCreate(args);start();}
 @Override public void onStart(){Bundle result=new Bundle();File root=new File(getTargetContext().getCacheDir(),"rescue-instrumentation");
  try{
   root.mkdirs();File fixture=new File(root,"fixture");fixture.mkdirs();
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
 private static void remove(File f){File[] children=f.listFiles();if(children!=null)for(File c:children)remove(c);f.delete();}
}
