// GOmul contributions: Copyright (c) 2026 invi-si. SPDX-License-Identifier: MIT
package local.wie.nativeapp;

import java.io.*;
import java.nio.ByteBuffer;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.*;
import java.util.zip.*;

/** Transfers one game's ordinary save tree, without its archive or quick-save slots. */
final class GameSaveBackup {
 private static final String FORMAT="GOmul Android Save v1";
 private static final long MAX_BYTES=128L*1024*1024;
 private static final int MAX_ENTRIES=4096,MAX_MANIFEST=1024*1024,MAX_PATH_BYTES=4096;

 private static final class Entry {
  final String path,hash;
  final boolean directory;
  final long length;
  Entry(String path,boolean directory,long length,String hash){this.path=path;this.directory=directory;this.length=length;this.hash=hash;}
 }

 private static void gameId(String id)throws IOException {
  if(id==null||!id.matches("[0-9a-f]{64}"))throw new IOException("Invalid game identifier.");
 }
 private static void safePath(String path)throws IOException {
  if(path.isEmpty()||path.getBytes(StandardCharsets.UTF_8).length>MAX_PATH_BYTES||path.indexOf('\\')>=0)throw new IOException("Invalid save path.");
  for(String part:path.split("/",-1)){
   if(part.isEmpty()||part.equals(".")||part.equals(".."))throw new IOException("Invalid save path.");
   for(int i=0;i<part.length();i++)if(part.charAt(i)<32||part.charAt(i)==127)throw new IOException("Invalid save path.");
  }
 }
 private static MessageDigest digest()throws IOException {
  try{return MessageDigest.getInstance("SHA-256");}catch(NoSuchAlgorithmException error){throw new IOException("SHA-256 is unavailable.",error);}
 }
 private static String hex(byte[] bytes){
  char[] digits="0123456789abcdef".toCharArray(),result=new char[bytes.length*2];
  for(int i=0;i<bytes.length;i++){int value=bytes[i]&255;result[i*2]=digits[value>>>4];result[i*2+1]=digits[value&15];}
  return new String(result);
 }
 private static Entry inspectFile(File file,String path,long[] total)throws IOException {
  MessageDigest hash=digest();long length=0;
  try(InputStream input=new FileInputStream(file)){
   byte[] buffer=new byte[65536];int read;
   while((read=input.read(buffer))!=-1){
    length+=read;
    if(length>MAX_BYTES-total[0])throw new IOException("Save data is too large.");
    hash.update(buffer,0,read);
   }
  }
  total[0]+=length;
  return new Entry(path,false,length,hex(hash.digest()));
 }
 private static void scan(File folder,String prefix,List<Entry> entries,long[] total)throws IOException {
  File[] children=folder.listFiles();if(children==null)throw new IOException("Cannot read game saves.");
  Arrays.sort(children,Comparator.comparing(File::getName));
  for(File child:children){
   if(Files.isSymbolicLink(child.toPath()))throw new IOException("Unexpected link in game saves.");
   String path=prefix+child.getName();safePath(path);
   if(entries.size()>=MAX_ENTRIES)throw new IOException("Too many save entries.");
   if(child.isDirectory()){
    entries.add(new Entry(path,true,0,""));
    scan(child,path+"/",entries,total);
   }else if(child.isFile())entries.add(inspectFile(child,path,total));
   else throw new IOException("Unsupported save entry.");
  }
 }
 private static String encoded(String path){return Base64.getUrlEncoder().withoutPadding().encodeToString(path.getBytes(StandardCharsets.UTF_8));}
 private static byte[] manifest(String game,List<Entry> entries)throws IOException {
  StringBuilder text=new StringBuilder(FORMAT).append('\n').append(game).append('\n').append(entries.size()).append('\n');
  for(Entry entry:entries){
   text.append(entry.directory?'d':'f').append('\t').append(encoded(entry.path));
   if(!entry.directory)text.append('\t').append(entry.length).append('\t').append(entry.hash);
   text.append('\n');
  }
  byte[] bytes=text.toString().getBytes(StandardCharsets.US_ASCII);
  if(bytes.length>MAX_MANIFEST)throw new IOException("Save manifest is too large.");
  return bytes;
 }
 static void exportSave(File root,String game,OutputStream destination)throws IOException {
  gameId(game);
  File save=new File(root,"saves/"+game);
  if(Files.isSymbolicLink(save.toPath())||!save.isDirectory())throw new IOException("This game has no ordinary save data.");
  List<Entry> entries=new ArrayList<>();scan(save,"",entries,new long[]{0});
  if(entries.isEmpty())throw new IOException("This game has no ordinary save data.");
  byte[] manifest=manifest(game,entries);
  try(ZipOutputStream zip=new ZipOutputStream(destination,StandardCharsets.UTF_8)){
   zip.putNextEntry(new ZipEntry("manifest.txt"));zip.write(manifest);zip.closeEntry();
   int fileIndex=0;
   for(Entry entry:entries){
    if(entry.directory)continue;
    File file=new File(save,entry.path);
    if(Files.isSymbolicLink(file.toPath())||!file.isFile())throw new IOException("Save data changed during export.");
    zip.putNextEntry(new ZipEntry("data/"+fileIndex++));
    MessageDigest hash=digest();long length=0;
    try(InputStream input=new FileInputStream(file)){
     byte[] buffer=new byte[65536];int read;
     while((read=input.read(buffer))!=-1){
      length+=read;if(length>entry.length)throw new IOException("Save data changed during export.");
      hash.update(buffer,0,read);zip.write(buffer,0,read);
     }
    }
    if(length!=entry.length||!hex(hash.digest()).equals(entry.hash))throw new IOException("Save data changed during export.");
    zip.closeEntry();
   }
   zip.finish();
  }
 }
 private static byte[] readManifest(ZipInputStream zip)throws IOException {
  ByteArrayOutputStream output=new ByteArrayOutputStream();byte[] buffer=new byte[8192];int read;
  while((read=zip.read(buffer))!=-1){if(output.size()+read>MAX_MANIFEST)throw new IOException("Save manifest is too large.");output.write(buffer,0,read);}
  return output.toByteArray();
 }
 private static String decoded(String value)throws IOException {
  try{
   byte[] bytes=Base64.getUrlDecoder().decode(value);
   if(!Base64.getUrlEncoder().withoutPadding().encodeToString(bytes).equals(value))throw new IOException("Invalid save path encoding.");
   String path=StandardCharsets.UTF_8.newDecoder().decode(ByteBuffer.wrap(bytes)).toString();safePath(path);return path;
  }catch(IllegalArgumentException|CharacterCodingException error){throw new IOException("Invalid save path encoding.",error);}
 }
 private static List<Entry> parseManifest(byte[] bytes,String game)throws IOException {
  String text=new String(bytes,StandardCharsets.US_ASCII);
  if(!Arrays.equals(bytes,text.getBytes(StandardCharsets.US_ASCII)))throw new IOException("Invalid save manifest.");
  String[] lines=text.split("\n",-1);
  if(lines.length<4||!lines[0].equals(FORMAT)||!lines[1].equals(game)||!lines[lines.length-1].isEmpty())throw new IOException("This backup belongs to another game or format.");
  int count;
  try{count=Integer.parseInt(lines[2]);}catch(NumberFormatException error){throw new IOException("Invalid save manifest.",error);}
  if(count<1||count>MAX_ENTRIES||lines.length!=count+4||!Integer.toString(count).equals(lines[2]))throw new IOException("Invalid save manifest.");
  List<Entry> entries=new ArrayList<>();Set<String> paths=new HashSet<>();long total=0;
  for(int i=0;i<count;i++){
   String[] fields=lines[i+3].split("\t",-1);
   if(fields.length<2)throw new IOException("Invalid save manifest.");
   String path=decoded(fields[1]);if(!paths.add(path))throw new IOException("Duplicate save path.");
   if(fields[0].equals("d")&&fields.length==2)entries.add(new Entry(path,true,0,""));
   else if(fields[0].equals("f")&&fields.length==4){
    long length;
    try{length=Long.parseLong(fields[2]);}catch(NumberFormatException error){throw new IOException("Invalid save length.",error);}
    if(length<0||length>MAX_BYTES-total||!Long.toString(length).equals(fields[2])||!fields[3].matches("[0-9a-f]{64}"))throw new IOException("Invalid save length or digest.");
    total+=length;entries.add(new Entry(path,false,length,fields[3]));
   }else throw new IOException("Invalid save manifest.");
  }
  return entries;
 }
 private static void removeTree(File file)throws IOException {
  if(!Files.exists(file.toPath(),LinkOption.NOFOLLOW_LINKS))return;
  if(Files.isDirectory(file.toPath(),LinkOption.NOFOLLOW_LINKS)){
   File[] children=file.listFiles();if(children==null)throw new IOException("Cannot clean temporary save data.");
   for(File child:children)removeTree(child);
  }
  Files.delete(file.toPath());
 }
 static void importSave(File root,String game,InputStream source)throws IOException {
  gameId(game);
  File parent=new File(root,"saves"),save=new File(parent,game);
  if(!parent.isDirectory()&&!parent.mkdirs())throw new IOException("Cannot create save storage.");
  if(Files.isSymbolicLink(parent.toPath())||Files.isSymbolicLink(save.toPath()))throw new IOException("Unexpected link in save storage.");
  File stage=Files.createTempDirectory(parent.toPath(),"restore-").toFile();boolean installed=false;
  try(ZipInputStream zip=new ZipInputStream(source,StandardCharsets.UTF_8)){
   ZipEntry first=zip.getNextEntry();
   if(first==null||first.isDirectory()||!first.getName().equals("manifest.txt"))throw new IOException("Invalid GOmul save backup.");
   List<Entry> entries=parseManifest(readManifest(zip),game);zip.closeEntry();
   int fileIndex=0;
   for(Entry entry:entries){
    File target=new File(stage,entry.path);
    if(entry.directory){Files.createDirectories(target.toPath());continue;}
    ZipEntry data=zip.getNextEntry();
    if(data==null||data.isDirectory()||!data.getName().equals("data/"+fileIndex++))throw new IOException("Invalid GOmul save backup.");
    Files.createDirectories(target.getParentFile().toPath());MessageDigest hash=digest();long length=0;
    try(OutputStream output=Files.newOutputStream(target.toPath(),StandardOpenOption.CREATE_NEW)){
     byte[] buffer=new byte[65536];int read;
     while((read=zip.read(buffer))!=-1){
      length+=read;if(length>entry.length)throw new IOException("Save entry is larger than declared.");
      hash.update(buffer,0,read);output.write(buffer,0,read);
     }
    }
    if(length!=entry.length||!hex(hash.digest()).equals(entry.hash))throw new IOException("Save entry failed integrity validation.");
    zip.closeEntry();
   }
   if(zip.getNextEntry()!=null)throw new IOException("Unexpected entry in save backup.");
   File backup=new File(parent,game+".before-save-import-"+UUID.randomUUID());boolean moved=false;
   if(Files.exists(save.toPath(),LinkOption.NOFOLLOW_LINKS)){
    if(!Files.isDirectory(save.toPath(),LinkOption.NOFOLLOW_LINKS))throw new IOException("Cannot replace game saves.");
    Files.move(save.toPath(),backup.toPath(),StandardCopyOption.ATOMIC_MOVE);moved=true;
   }
   try{Files.move(stage.toPath(),save.toPath(),StandardCopyOption.ATOMIC_MOVE);installed=true;}
   catch(IOException error){if(moved)try{Files.move(backup.toPath(),save.toPath(),StandardCopyOption.ATOMIC_MOVE);}catch(IOException restore){error.addSuppressed(restore);}throw error;}
  }finally{if(!installed)removeTree(stage);}
 }
}
