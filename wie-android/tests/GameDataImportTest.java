package local.wie.nativeapp;
import java.io.*;
import java.nio.file.*;
import java.util.*;
import java.util.zip.*;
public final class GameDataImportTest {
 static void check(boolean value){if(!value)throw new AssertionError();}
 static void archive(File file,Map<String,String> entries)throws Exception {
  try(ZipOutputStream z=new ZipOutputStream(new FileOutputStream(file))){for(Map.Entry<String,String> entry:entries.entrySet()){z.putNextEntry(new ZipEntry(entry.getKey()));z.write(entry.getValue().getBytes("UTF-8"));z.closeEntry();}}
 }
 static void fails(RunnableIO task)throws Exception {boolean failed=false;try{task.run();}catch(IOException expected){failed=true;}check(failed);}
 interface RunnableIO {void run()throws Exception;}
 public static void main(String[] args)throws Exception {
  File root=Files.createTempDirectory("gomul-import-test-").toFile();
  try{
   File game=new File(root,"input.tmp");
   archive(game,Map.of("package/app_info","PID:PDTEST\n","package/test.jar","synthetic","package/data/savedata","progress","package/data/locdata","locations","package/gomul.properties","phoneNumber=01000000000\n"));
   GameDataImport.normalize(game);check(GameDataImport.pid(game).equals("PDTEST"));
   GameDataImport.Plan plan=GameDataImport.inspect(game,game);check(plan.files.size()==2);check(plan.phone.equals("01000000000"));
   File save=new File(root,"saves/game-a");GameDataImport.install(save,plan,plan.phone);
   check(Files.readString(new File(save,"PDTEST/db/savedata/1").toPath()).equals("progress"));
   check(Files.readString(new File(save,"phone-number.txt").toPath()).equals("01000000000\n"));
   Files.writeString(new File(save,"PDTEST/db/savedata/1").toPath(),"newer progress");fails(()->GameDataImport.install(save,plan,plan.phone));
   check(Files.readString(new File(save,"PDTEST/db/savedata/1").toPath()).equals("newer progress"));
   GameDataImport.install(save,plan,plan.phone,true);
   File backup=Arrays.stream(save.getParentFile().listFiles()).filter(f->f.getName().startsWith("game-a.before-data-")).findFirst().get();
   check(Files.readString(new File(backup,"PDTEST/db/savedata/1").toPath()).equals("newer progress"));
   File bad=new File(root,"invalid.zip");archive(bad,Map.of("app_info","PID:PDTEST\n","../savedata","bad"));fails(()->GameDataImport.inspect(bad,bad));
   archive(bad,Map.of("app_info","PID:PDTEST\n","a/savedata","first","b/savedata","second"));fails(()->GameDataImport.inspect(bad,bad));
   fails(()->GameDataImport.install(new File(root,"saves/game-b"),plan,"invalid"));check(!new File(root,"saves/game-b").exists());
   File plain=new File(root,"plain.jar");archive(plain,Map.of("META-INF/MANIFEST.MF","Manifest-Version: 1.0"));check(GameDataImport.inspect(plain,plain)==null);
   File external=new File(root,"external.zip");archive(external,Map.of("savedata","separate progress"));check(GameDataImport.inspect(game,external).phone==null);
   System.out.println("PASS: normalization, identity, data mapping, existing-save protection, replacement backup, traversal/mixed-bundle rejection, invalid identity, plain JAR and separate data.");
  }finally{GameDataImport.removeTree(root);}
 }
}
