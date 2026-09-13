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
   File ktf=new File(root,"ktf.zip");archive(ktf,Map.of("wrapper/game/__adf__","synthetic metadata","wrapper/game/test.jar","jar bytes","wrapper/game/P/prefs","initial data","wrapper/game2/readme.txt","outside root"));
   GameDataImport.normalize(ktf);
   try(ZipFile z=GameDataImport.zip(ktf)){
    check(z.size()==3);check(z.getEntry("__adf__")!=null);check(z.getEntry("test.jar")!=null);
    check(new String(z.getInputStream(z.getEntry("P/prefs")).readAllBytes(),"UTF-8").equals("initial data"));
   }
   byte[] normalized=Files.readAllBytes(ktf.toPath());GameDataImport.normalize(ktf);check(Arrays.equals(normalized,Files.readAllBytes(ktf.toPath())));
   for(Map<String,String> mixed:List.of(Map.of("a/__adf__","a","b/__adf__","b"),Map.of("app_info","a","b/__adf__","b"))){
    archive(bad,mixed);byte[] original=Files.readAllBytes(bad.toPath());fails(()->GameDataImport.normalize(bad));check(Arrays.equals(original,Files.readAllBytes(bad.toPath())));
   }
   File legacy=new File(root,"saves/legacy");check(!GameDataImport.hasProgress(legacy));legacy.mkdirs();
   Files.writeString(new File(legacy,"phone-number.txt").toPath(),"01000000000");
   Files.writeString(new File(legacy,"display-options").toPath(),"240 320 0");
   new File(legacy,"PDTEST/db/savedata").mkdirs();check(!GameDataImport.hasProgress(legacy));
   File legacyRecord=new File(legacy,"PDTEST/db/savedata/1");Files.writeString(legacyRecord.toPath(),"earned progress");
   check(GameDataImport.hasProgress(legacy));
   // The ordinary-launch decision used by MainActivity must never replace an unmarked save.
   if(!GameDataImport.hasProgress(legacy))GameDataImport.install(legacy,plan,plan.phone,true);
   check(Files.readString(legacyRecord.toPath()).equals("earned progress"));
   File plain=new File(root,"plain.jar");archive(plain,Map.of("META-INF/MANIFEST.MF","Manifest-Version: 1.0"));check(GameDataImport.inspect(plain,plain)==null);
   File external=new File(root,"external.zip");archive(external,Map.of("savedata","separate progress"));check(GameDataImport.inspect(game,external).phone==null);
   File hero=new File(root,"hero.zip");archive(hero,Map.of("app_info","PID:PD121120\nAID:00028E76\n","data/savedata","hero progress"));
   GameDataImport.Plan heroPlan=GameDataImport.inspect(hero,hero);check(heroPlan.phone.equals("01055145031"));
   check(GameDataImport.inspect(hero,external).phone.equals("01055145031"));
   check(new GameDataImport.Plan("PD121120",heroPlan.files,null).phone.equals("01055145031"));
   check(new GameDataImport.Plan("PD121120",heroPlan.files,"01000000000").phone.equals("01000000000"));
   File heroSave=new File(root,"saves/hero");GameDataImport.install(heroSave,heroPlan,heroPlan.phone);
   check(Files.readString(new File(heroSave,"phone-number.txt").toPath()).equals("01055145031\n"));
   check(Files.readString(new File(heroSave,"PD121120/db/savedata/1").toPath()).equals("hero progress"));
   System.out.println("PASS: normalization, identity, data mapping, existing-save protection, replacement backup, traversal/mixed-bundle rejection, invalid identity, plain JAR and separate data.");
  }finally{GameDataImport.removeTree(root);}
 }
}
