package local.wie.nativeapp;
import java.nio.file.*;
public final class GameSaveResetTest {
 static void check(boolean value){if(!value)throw new AssertionError();}
 static void put(Path root,String path)throws Exception {Path p=root.resolve(path);Files.createDirectories(p.getParent());Files.writeString(p,"keep");}
 public static void main(String[] args)throws Exception {
  Path root=Files.createTempDirectory("gomul-reset-test");String a="a".repeat(64),b="b".repeat(64);
  try{
   for(String path:new String[]{"saves/"+a+"/db/record","saves/"+a+"/phone-number.txt","saves/"+a+"/companion-imported","checkpoints/"+a+"/quick/trace","checkpoints/"+a+"/recovery/trace","saves/"+b+"/db/record","checkpoints/"+b+"/quick/trace","games/"+a+"/game.jar","rescues/"+a+"/latest/error.txt","shared_prefs/game-settings-"+a+".xml"})put(root,path);
   GameSaveReset.reset(root.toFile(),a);
   check(!Files.exists(root.resolve("saves/"+a+"/db")));check(!Files.exists(root.resolve("saves/"+a+"/companion-imported")));check(!Files.exists(root.resolve("checkpoints/"+a)));
   for(String path:new String[]{"saves/"+a+"/phone-number.txt","saves/"+b+"/db/record","checkpoints/"+b+"/quick/trace","games/"+a+"/game.jar","rescues/"+a+"/latest/error.txt","shared_prefs/game-settings-"+a+".xml"})check(Files.readString(root.resolve(path)).equals("keep"));
   GameSaveReset.reset(root.toFile(),a);GameSaveReset.reset(root.toFile(),"c".repeat(64));
   boolean rejected=false;try{GameSaveReset.reset(root.toFile(),"../");}catch(java.io.IOException expected){rejected=true;}check(rejected);
   System.out.println("Game save reset isolation, retained configuration, checkpoint removal and repeat reset: PASS");
  }finally{try(var paths=Files.walk(root)){for(Path p:paths.sorted(java.util.Comparator.reverseOrder()).toList())Files.delete(p);}}
 }
}
