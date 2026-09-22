package local.wie.nativeapp;

import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import java.util.zip.*;

public final class GameSaveBackupTest {
 private interface Task {void run()throws Exception;}
 private static void check(boolean value){if(!value)throw new AssertionError();}
 private static void fails(Task task)throws Exception {try{task.run();throw new AssertionError("Expected backup rejection");}catch(IOException expected){}}
 private static void put(Path root,String name,String data)throws Exception {
  Path file=root.resolve(name);Files.createDirectories(file.getParent());Files.writeString(file,data);
 }
 private static byte[] rewrite(byte[] original,String entryName,byte[] replacement)throws Exception {
  ByteArrayOutputStream result=new ByteArrayOutputStream();
  try(ZipInputStream input=new ZipInputStream(new ByteArrayInputStream(original));ZipOutputStream output=new ZipOutputStream(result)){
   ZipEntry entry;
   while((entry=input.getNextEntry())!=null){
    output.putNextEntry(new ZipEntry(entry.getName()));
    output.write(entry.getName().equals(entryName)?replacement:input.readAllBytes());
    output.closeEntry();input.closeEntry();
   }
  }
  return result.toByteArray();
 }
 private static void remove(Path root)throws Exception {
  try(var paths=Files.walk(root)){for(Path path:paths.sorted(Comparator.reverseOrder()).toList())Files.delete(path);}
 }
 public static void main(String[] args)throws Exception {
  Path root=Files.createTempDirectory("gomul-save-backup-");
  String a="a".repeat(64),b="b".repeat(64);
  try{
   put(root,"saves/"+a+"/PDTEST/db/기록/1","saved progress");
   put(root,"saves/"+a+"/phone-number.txt","01012345678\n");
   put(root,"saves/"+a+"/companion-imported","1");
   Files.createDirectories(root.resolve("saves/"+a+"/empty"));
   put(root,"games/"+a+"/game.zip","commercial archive must not be exported");
   put(root,"checkpoints/"+a+"/quick/trace","checkpoint must not be exported");
   put(root,"saves/"+b+"/progress","other game");
   ByteArrayOutputStream output=new ByteArrayOutputStream();GameSaveBackup.exportSave(root.toFile(),a,output);
   byte[] backup=output.toByteArray();
   try(ZipInputStream zip=new ZipInputStream(new ByteArrayInputStream(backup))){
    List<String> names=new ArrayList<>();ZipEntry entry;
    while((entry=zip.getNextEntry())!=null){names.add(entry.getName());zip.closeEntry();}
    check(names.get(0).equals("manifest.txt"));
    check(names.stream().skip(1).allMatch(name->name.matches("data/[0-9]+")));
    check(names.stream().noneMatch(name->name.contains("game.zip")||name.contains("checkpoint")));
   }
   put(root,"saves/"+a+"/PDTEST/db/기록/1","current progress");
   fails(()->GameSaveBackup.importSave(root.toFile(),b,new ByteArrayInputStream(backup)));
   check(Files.readString(root.resolve("saves/"+b+"/progress")).equals("other game"));
   byte[] corrupt=rewrite(backup,"data/0","tampered".getBytes(StandardCharsets.UTF_8));
   fails(()->GameSaveBackup.importSave(root.toFile(),a,new ByteArrayInputStream(corrupt)));
   check(Files.readString(root.resolve("saves/"+a+"/PDTEST/db/기록/1")).equals("current progress"));
   byte[] invalidPath;
   try(ZipInputStream zip=new ZipInputStream(new ByteArrayInputStream(backup))){
    zip.getNextEntry();String manifest=new String(zip.readAllBytes(),StandardCharsets.US_ASCII);
    String encoded=Base64.getUrlEncoder().withoutPadding().encodeToString("../escaped".getBytes(StandardCharsets.UTF_8));
    String[] lines=manifest.split("\n",-1);
    for(int i=3;i<lines.length-1;i++)if(lines[i].startsWith("f\t")){String[] fields=lines[i].split("\t");fields[1]=encoded;lines[i]=String.join("\t",fields);break;}
    invalidPath=rewrite(backup,"manifest.txt",String.join("\n",lines).getBytes(StandardCharsets.US_ASCII));
   }
   fails(()->GameSaveBackup.importSave(root.toFile(),a,new ByteArrayInputStream(invalidPath)));
   check(!Files.exists(root.resolve("escaped")));
   byte[] missing;
   try(ByteArrayOutputStream partial=new ByteArrayOutputStream();ZipOutputStream zip=new ZipOutputStream(partial)){
    try(ZipInputStream source=new ZipInputStream(new ByteArrayInputStream(backup))){source.getNextEntry();zip.putNextEntry(new ZipEntry("manifest.txt"));zip.write(source.readAllBytes());zip.closeEntry();}
    zip.finish();missing=partial.toByteArray();
   }
   fails(()->GameSaveBackup.importSave(root.toFile(),a,new ByteArrayInputStream(missing)));
   check(Files.readString(root.resolve("saves/"+a+"/PDTEST/db/기록/1")).equals("current progress"));
   GameSaveBackup.importSave(root.toFile(),a,new ByteArrayInputStream(backup));
   check(Files.readString(root.resolve("saves/"+a+"/PDTEST/db/기록/1")).equals("saved progress"));
   check(Files.isDirectory(root.resolve("saves/"+a+"/empty")));
   check(Files.readString(root.resolve("saves/"+b+"/progress")).equals("other game"));
   check(Files.readString(root.resolve("games/"+a+"/game.zip")).equals("commercial archive must not be exported"));
   check(Files.readString(root.resolve("checkpoints/"+a+"/quick/trace")).equals("checkpoint must not be exported"));
   try(var children=Files.list(root.resolve("saves"))){
    Path previous=children.filter(path->path.getFileName().toString().startsWith(a+".before-save-import-")).findFirst().orElseThrow();
    check(Files.readString(previous.resolve("PDTEST/db/기록/1")).equals("current progress"));
   }
   System.out.println("PASS: save export scope, identity and digest validation, malformed-input preservation, isolated restore and rollback copy");
  }finally{remove(root);}
 }
}
