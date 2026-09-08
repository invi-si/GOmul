package local.wie.nativeapp;

import java.io.*;
import java.nio.charset.Charset;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import java.util.zip.*;

/** Local companion-data import. Never downloads data or replaces existing progress. */
final class GameDataImport {
 static final int MAX_DATA=16*1024*1024;
 static final class Plan {
  final String pid; final Map<String,byte[]> files; final String phone;
  Plan(String pid,Map<String,byte[]> files,String phone){this.pid=pid;this.files=files;this.phone=phone;}
 }
 static boolean dataName(String name){return name.matches("savedata|locdata|ranker|it[0-9]+|mk[0-9]+");}
 static void safeName(String name)throws IOException {
  if(name.startsWith("/")||name.contains("\\")||Arrays.asList(name.split("/")).contains(".."))throw new IOException("Unsafe archive path.");
 }
 static byte[] read(InputStream input,int limit)throws IOException {
  ByteArrayOutputStream output=new ByteArrayOutputStream();byte[] bytes=new byte[8192];int count;
  while((count=input.read(bytes))!=-1){if(output.size()+count>limit)throw new IOException("Imported data is too large.");output.write(bytes,0,count);}return output.toByteArray();
 }
 static ZipFile zip(File file)throws IOException{return new ZipFile(file,Charset.forName("CP437"));}
 static String pid(File game)throws IOException {
  try(ZipFile archive=zip(game)){
   ZipEntry info=archive.getEntry("app_info");if(info==null)return null;
   String text;try(InputStream in=archive.getInputStream(info)){text=new String(read(in,65536),Charset.forName("EUC-KR"));}
   for(String line:text.split("\n"))if(line.startsWith("PID:")){
    String value=line.substring(4).trim();if(!value.matches("[A-Za-z0-9_-]+"))throw new IOException("Invalid application identity.");return value;
   }
  }return null;
 }
 static void normalize(File game)throws IOException {
  try(ZipFile archive=zip(game)){
   String infoName=null;Enumeration<? extends ZipEntry> entries=archive.entries();
   while(entries.hasMoreElements()){ZipEntry entry=entries.nextElement();safeName(entry.getName());if(entry.getName().equals("app_info")||entry.getName().endsWith("/app_info")){if(infoName!=null)throw new IOException("Choose a package containing one game.");infoName=entry.getName();}}
   if(infoName==null||infoName.equals("app_info"))return;
   String prefix=infoName.substring(0,infoName.length()-"app_info".length());File temp=File.createTempFile("normalize-",".tmp",game.getParentFile());
   try{
    try(ZipOutputStream output=new ZipOutputStream(new FileOutputStream(temp))){
     entries=archive.entries();long total=0;Set<String> names=new HashSet<>();
     while(entries.hasMoreElements()){ZipEntry entry=entries.nextElement();String name=entry.getName();if(entry.isDirectory()||!name.startsWith(prefix))continue;
      name=name.substring(prefix.length());if(!names.add(name))throw new IOException("Duplicate archive entry.");
      byte[] bytes;try(InputStream in=archive.getInputStream(entry)){bytes=read(in,128*1024*1024);}total+=bytes.length;if(total>128L*1024*1024)throw new IOException("Game package is too large.");
      output.putNextEntry(new ZipEntry(name));output.write(bytes);output.closeEntry();
     }
    }
    Files.move(temp.toPath(),game.toPath(),StandardCopyOption.REPLACE_EXISTING,StandardCopyOption.ATOMIC_MOVE);
   }finally{temp.delete();}
  }
 }
 static Plan inspect(File game,File dataZip)throws IOException {
  String application=pid(game);if(application==null)return null;
  Map<String,byte[]> files=new LinkedHashMap<>();String phone=null;String dataParent=null;int total=0;
  try(ZipFile archive=zip(dataZip)){
   Enumeration<? extends ZipEntry> entries=archive.entries();
   while(entries.hasMoreElements()){
    ZipEntry entry=entries.nextElement();String name=entry.getName();safeName(name);if(entry.isDirectory())continue;
    int slash=name.lastIndexOf('/');String leaf=name.substring(slash+1),parent=name.substring(0,slash+1);
    if(leaf.equals("gomul.properties")){
     if(phone!=null)throw new IOException("More than one identity setup file.");Properties properties=new Properties();try(InputStream in=archive.getInputStream(entry)){properties.load(new ByteArrayInputStream(read(in,4096)));}phone=properties.getProperty("phoneNumber");continue;
    }
    if(!dataName(leaf))continue;
    if(dataParent!=null&&!dataParent.equals(parent))throw new IOException("Choose the data folder for one game only.");dataParent=parent;
    if(files.containsKey(leaf))throw new IOException("Duplicate data file.");byte[] bytes;try(InputStream in=archive.getInputStream(entry)){bytes=read(in,MAX_DATA);}total+=bytes.length;if(total>MAX_DATA)throw new IOException("Data bundle is too large.");files.put(leaf,bytes);
   }
  }
  return files.isEmpty()?null:new Plan(application,files,phone);
 }
 static void install(File save,Plan plan,String phone)throws IOException { install(save,plan,phone,false); }
 static void install(File save,Plan plan,String phone,boolean replace)throws IOException {
  String number=phone.trim();if(!number.matches("[0-9]{11}"))throw new IOException("Enter the 11-digit number supplied with these data files.");
  if(save.exists()&&!replace)throw new IOException("This game already has save data. Existing progress was kept.");
  File parent=save.getParentFile();if(!parent.isDirectory()&&!parent.mkdirs())throw new IOException("Cannot create save storage.");
  File temp=Files.createTempDirectory(parent.toPath(),"import-data-").toFile();
  try{
   for(Map.Entry<String,byte[]> entry:plan.files.entrySet()){
    if(!dataName(entry.getKey()))throw new IOException("Unsupported data filename.");
    File record=new File(temp,plan.pid+"/db/"+entry.getKey()+"/1");if(!record.getParentFile().mkdirs())throw new IOException("Cannot create data record.");Files.write(record.toPath(),entry.getValue());
   }
   Files.write(new File(temp,"phone-number.txt").toPath(),(number+"\n").getBytes(StandardCharsets.UTF_8));
   File backup=null;
   if(save.exists()){backup=new File(parent,save.getName()+".before-data-"+UUID.randomUUID());Files.move(save.toPath(),backup.toPath(),StandardCopyOption.ATOMIC_MOVE);}
   try{Files.move(temp.toPath(),save.toPath(),StandardCopyOption.ATOMIC_MOVE);}
   catch(IOException error){if(backup!=null)Files.move(backup.toPath(),save.toPath(),StandardCopyOption.ATOMIC_MOVE);throw error;}
  }finally{removeTree(temp);}
 }
 static void removeTree(File file){File[] children=file.listFiles();if(children!=null)for(File child:children)removeTree(child);file.delete();}
}
