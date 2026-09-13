package local.wie.nativeapp;

import android.graphics.Bitmap;
import java.io.*;
import java.nio.*;
import java.nio.file.Files;
import java.util.zip.*;

/** Offline exports only. Reports never include the imported game archive. */
final class RescueReports {
 static File latest(File files, String game) {
  File root=new File(files,"rescues");
  if(game!=null){File folder=new File(root,game);File automatic=new File(folder,"latest"),manual=new File(folder,"manual/latest");return newest(automatic,manual);}
  File result=null;File[] games=root.listFiles();if(games!=null)for(File folder:games){File candidate=latest(files,folder.getName());result=newest(result,candidate);}return result;
 }
 private static File newest(File a,File b){
  if(a!=null&&!new File(a,"report.txt").isFile())a=null;
  if(b!=null&&!new File(b,"report.txt").isFile())b=null;
  return a==null?b:b==null?a:a.lastModified()>=b.lastModified()?a:b;
 }
 static void export(File report, OutputStream output)throws IOException {
  try(ZipOutputStream zip=new ZipOutputStream(output)){
   append(zip,report,"");
   File frame=new File(report,"frame");
   if(frame.isFile()){
    // Native checkpoint frame: LE width, height, paint count, redraw, ARGB pixels.
    long size=frame.length();if(size>=17&&size<=4L*1024*1024+17){
     ByteBuffer data=ByteBuffer.wrap(Files.readAllBytes(frame.toPath())).order(ByteOrder.LITTLE_ENDIAN);
     int w=data.getInt(),h=data.getInt();data.position(17);
     if(w>0&&h>0&&(long)w*h<=1024*1024&&size==17+4L*w*h){
      int[] pixels=new int[w*h];data.asIntBuffer().get(pixels);Bitmap bitmap=Bitmap.createBitmap(pixels,w,h,Bitmap.Config.ARGB_8888);
      try{zip.putNextEntry(new ZipEntry("screenshot.png"));if(!bitmap.compress(Bitmap.CompressFormat.PNG,100,zip))throw new IOException("Screenshot encoding failed");zip.closeEntry();}finally{bitmap.recycle();}
     }
    }
   }
  }
 }
 private static void append(ZipOutputStream zip,File directory,String prefix)throws IOException {
  File[] files=directory.listFiles();if(files==null)throw new IOException("Cannot read rescue report");
  for(File f:files){if(Files.isSymbolicLink(f.toPath()))throw new IOException("Unexpected link in rescue report");String name=prefix+f.getName();
   if(f.isDirectory()){zip.putNextEntry(new ZipEntry(name+"/"));zip.closeEntry();append(zip,f,name+"/");}
   else {zip.putNextEntry(new ZipEntry(name));try(InputStream in=new FileInputStream(f)){byte[] b=new byte[65536];int n;while((n=in.read(b))!=-1)zip.write(b,0,n);}zip.closeEntry();}
  }
 }
}
