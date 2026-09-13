package local.wie.nativeapp;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;

/** Reset only the selected archive's progress; keep its configured identity. */
final class GameSaveReset {
 static void reset(File root,String gameId)throws IOException {
  if(!gameId.matches("[0-9a-f]{64}"))throw new IOException("Invalid game identifier");
  File save=new File(root,"saves/"+gameId);
  File[] children=save.listFiles();
  if(save.exists()&&children==null)throw new IOException("Cannot read game saves");
  remove(new File(root,"checkpoints/"+gameId));
  if(children!=null)for(File child:children){
   if(!child.getName().equals("phone-number.txt"))remove(child);
  }
 }
 private static void remove(File file)throws IOException {
  if(!file.exists())return;
  if(file.isDirectory()){
   File[] children=file.listFiles();if(children==null)throw new IOException("Cannot read saved data");
   for(File child:children)remove(child);
  }
  Files.delete(file.toPath());
 }
}
