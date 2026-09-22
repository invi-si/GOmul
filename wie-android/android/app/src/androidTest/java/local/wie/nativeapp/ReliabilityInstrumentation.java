package local.wie.nativeapp;

import android.app.AlertDialog;
import android.app.Instrumentation;
import android.content.Intent;
import android.content.pm.ActivityInfo;
import android.os.Bundle;
import android.os.SystemClock;
import android.view.MotionEvent;
import android.widget.Button;
import java.io.File;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.function.BooleanSupplier;

/** Real JNI and Activity regression checks, with isolated synthetic save identities. */
final class ReliabilityInstrumentation {
 private static final String ID="e".repeat(62)+"09";
 private static void require(boolean ok,String reason){if(!ok)throw new AssertionError(reason);}
 private static void waitFor(BooleanSupplier condition,String reason)throws Exception{
  long deadline=SystemClock.elapsedRealtime()+15000;
  do{if(condition.getAsBoolean())return;Thread.sleep(40);}while(SystemClock.elapsedRealtime()<deadline);
  throw new AssertionError(reason+"; native="+NativeBridge.status());
 }
 private static Object field(Object object,String name){try{Field f=object.getClass().getDeclaredField(name);f.setAccessible(true);return f.get(object);}catch(Exception e){throw new RuntimeException(e);}}
 private static void call(Object object,String method,Class<?>[] types,Object... args){try{Method m=object.getClass().getDeclaredMethod(method,types);m.setAccessible(true);m.invoke(object,args);}catch(Exception e){throw new RuntimeException(e);}}
 private static boolean ui(Instrumentation test,BooleanSupplier condition){AtomicBoolean result=new AtomicBoolean();test.runOnMainSync(()->result.set(condition.getAsBoolean()));return result.get();}
 private static Map<String,String> tree(File root)throws Exception{
  Map<String,String> hashes=new TreeMap<>();if(!root.exists())return hashes;
  try(var paths=Files.walk(root.toPath())){for(var p:(Iterable<java.nio.file.Path>)paths::iterator){if(Files.isRegularFile(p))hashes.put(root.toPath().relativize(p).toString(),Arrays.toString(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(p))));}}
  return hashes;
 }
 private static void prepare(File game,File save)throws Exception{
  GameDataImport.prepareBundledIdentity(game,save);
  GameDataImport.Plan plan=GameDataImport.inspect(game,game);
  if(plan!=null){if(plan.phone==null)throw new AssertionError("Use a fixture without an interactive identity requirement");GameDataImport.install(save,plan,plan.phone,save.exists());}
 }
 private static void checkpoint(String action)throws Exception{NativeBridge.checkpoint(action);}
 private static void failedLoad(String action,String expected)throws Exception{
  try{checkpoint(action);throw new AssertionError("Expected rejection: "+action);}catch(Exception error){require(error.getMessage().contains(expected),"Unexpected load rejection: "+error.getMessage());}
 }
 private static long lastClock(File trace)throws Exception{
  java.nio.ByteBuffer b=java.nio.ByteBuffer.wrap(Files.readAllBytes(trace.toPath())).order(java.nio.ByteOrder.LITTLE_ENDIAN);long last=-1;
  while(b.hasRemaining())switch(b.get()){
   case 1:last=b.getLong();b.getInt();break;
   case 2:long count=b.getLong();b.position(Math.toIntExact(b.position()+count*8));break;
   case 3:case 5:break;
   case 4:b.position(b.position()+2);break;
   default:throw new AssertionError("Invalid recorded event");
  }
  require(last>=0,"No guest clocks recorded");return last;
 }
 static void run(Instrumentation test,Bundle arguments){
  if(arguments.getString("mode","").startsWith("checkpoint-process-")){processPersistence(test,arguments);return;}
  Bundle result=new Bundle();File root=new File(test.getTargetContext().getCacheDir(),"reliability-test");MainActivity activity=null;
  File app=test.getTargetContext().getFilesDir(),save=new File(app,"saves/"+ID),slots=new File(app,"checkpoints/"+ID),rescues=new File(app,"rescues/"+ID);
  boolean owned=false;java.util.concurrent.CountDownLatch releaseIo=new java.util.concurrent.CountDownLatch(1);
  try{
   require(!save.exists()&&!slots.exists()&&!rescues.exists(),"Synthetic test identity already exists");owned=true;
   GameDataImport.removeTree(root);File game=new File(root,ID+"/fixture.zip");game.getParentFile().mkdirs();Files.copy(new File(arguments.getString("game")).toPath(),game.toPath());
   prepare(game,save);
   if("android-lifecycle".equals(arguments.getString("mode"))){
    require(!new File(app,"mac-checkpoints.json").exists(),"Standalone test requires no Mac snapshot helper");
    activity=(MainActivity)test.startActivitySync(new Intent(test.getTargetContext(),MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));MainActivity current=activity;
    test.runOnMainSync(()->call(current,"launch",new Class<?>[]{File.class},game));
    waitFor(()->ui(test,()->!(Boolean)field(current,"starting"))&&"Running".equals(NativeBridge.status()),"Launcher did not start");
    Thread.sleep(1000);
    Button key=(Button)((Map<?,?>)field(current,"keypadButtons")).get("5");require(key!=null,"No keypad button");
    test.runOnMainSync(()->{long now=SystemClock.uptimeMillis();MotionEvent down=MotionEvent.obtain(now,now,MotionEvent.ACTION_DOWN,5,5,0);key.dispatchTouchEvent(down);down.recycle();});
    require(ui(test,()->!((Set<?>)field(current,"held")).isEmpty()),"Touch did not hold a key");
    AlertDialog[] dialog=new AlertDialog[1];test.runOnMainSync(()->dialog[0]=new AlertDialog.Builder(current).setTitle("Regression test").setPositiveButton("Close",null).show());
    waitFor(()->ui(test,()->((Set<?>)field(current,"held")).isEmpty()),"Dialog focus loss left a held key");
    test.runOnMainSync(()->dialog[0].dismiss());
    test.runOnMainSync(()->call(current,"key",new Class<?>[]{String.class,boolean.class},"LEFT",true));
    test.getUiAutomation().executeShellCommand("input keyevent KEYCODE_HOME").close();
    waitFor(()->ui(test,()->!(Boolean)field(current,"foreground")&&((Set<?>)field(current,"held")).isEmpty()),"Home did not pause/release keys");
    checkpoint("save:1");long paints=NativeBridge.paints();long before=lastClock(new File(slots,"quick/trace"));
    Thread.sleep(3000);require(NativeBridge.paints()==paints,"Rendering advanced in background");
    test.runOnMainSync(()->current.startActivity(new Intent(current,MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_REORDER_TO_FRONT)));
    waitFor(()->ui(test,()->(Boolean)field(current,"foreground")),"Activity did not resume");Thread.sleep(200);
    NativeBridge.pause(true);checkpoint("save:2");long after=lastClock(new File(slots,"quick-2/trace"));
    require(after>=before&&after-before<2000,"Guest clock included the 3-second background interval: "+(after-before));
    NativeBridge.pause(false);
    Object previous=field(current,"gameView");
    test.runOnMainSync(()->{call(current,"key",new Class<?>[]{String.class,boolean.class},"RIGHT",true);current.setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE);});
    waitFor(()->ui(test,()->field(current,"gameView")!=previous&&((Set<?>)field(current,"held")).isEmpty()),"Rotation did not rebuild layout/release keys");
    require("Running".equals(NativeBridge.status()),"Game stopped after rotation");
    java.util.concurrent.CountDownLatch enteredIo=new java.util.concurrent.CountDownLatch(1);
    ((java.util.concurrent.ExecutorService)field(current,"io")).execute(()->{enteredIo.countDown();try{releaseIo.await(15,java.util.concurrent.TimeUnit.SECONDS);}catch(InterruptedException e){Thread.currentThread().interrupt();}});
    require(enteredIo.await(5,java.util.concurrent.TimeUnit.SECONDS),"Background I/O did not start");
    test.runOnMainSync(current::finish);waitFor(()->ui(test,current::isDestroyed),"Old Activity was not destroyed");
    AtomicBoolean staleCompletion=new AtomicBoolean();call(current,"postIfAlive",new Class<?>[]{Runnable.class},(Runnable)()->staleCompletion.set(true));
    activity=(MainActivity)test.startActivitySync(new Intent(test.getTargetContext(),MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));MainActivity replacement=activity;
    test.runOnMainSync(()->call(replacement,"launch",new Class<?>[]{File.class},game));
    releaseIo.countDown();
    waitFor(()->ui(test,()->!(Boolean)field(replacement,"starting"))&&"Running".equals(NativeBridge.status()),"Old cleanup stopped replacement Activity's game");
    require(!staleCompletion.get(),"Destroyed Activity received asynchronous UI work");
    test.getUiAutomation().executeShellCommand("input keyevent KEYCODE_SLEEP").close();
    waitFor(()->ui(test,()->!(Boolean)field(replacement,"foreground")),"Screen lock did not pause Activity");
    checkpoint("save:3");long lockedPaints=NativeBridge.paints();Thread.sleep(300);require(NativeBridge.paints()==lockedPaints,"Game advanced with screen locked");
    test.getUiAutomation().executeShellCommand("input keyevent KEYCODE_WAKEUP").close();test.getUiAutomation().executeShellCommand("wm dismiss-keyguard").close();
    test.runOnMainSync(()->replacement.startActivity(new Intent(replacement,MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_REORDER_TO_FRONT)));
    waitFor(()->ui(test,()->(Boolean)field(replacement,"foreground")),"Screen unlock did not resume Activity");
    result.putString("stream","Activity touch/focus release, Home pause, resume clock continuity, rotation, pending I/O recreation, screen lock/unlock and continued game session: PASS\n");
   }else{
    NativeBridge.start(game.getAbsolutePath(),save.getAbsolutePath());waitFor(()->"Running".equals(NativeBridge.status()),"Native startup failed");Thread.sleep(1500);
    NativeBridge.pause(true);checkpoint("save:1");
    long beforePause=lastClock(new File(slots,"quick/trace"));Thread.sleep(3000);
    for(int slot=2;slot<=3;slot++){
     NativeBridge.pause(false);Thread.sleep(250);NativeBridge.pause(true);checkpoint("save:"+slot);
     if(slot==2){long afterPause=lastClock(new File(slots,"quick-2/trace"));require(afterPause>=beforePause&&afterPause-beforePause<2000,"Guest clock advanced during pause: "+(afterPause-beforePause));}
    }
    Map<String,String> completeSlots=tree(slots);NativeBridge.stop();
    NativeBridge.start(game.getAbsolutePath(),save.getAbsolutePath());NativeBridge.pause(true);
    for(int slot=1;slot<=3;slot++){checkpoint("load:"+slot);require("Running".equals(NativeBridge.status()),"Stopped loading slot "+slot);}
    for(var entry:completeSlots.entrySet())require(entry.getValue().equals(tree(slots).get(entry.getKey())),"Loading altered an existing quick slot");
    File build=new File(slots,"quick-2/build-id");byte[] identity=Files.readAllBytes(build.toPath());Map<String,String> progress=tree(save);
    Files.write(build.toPath(),"unrelated-build".getBytes(StandardCharsets.UTF_8));failedLoad("load:2","APK build");require(tree(save).equals(progress),"Wrong-build load changed progress");Files.write(build.toPath(),identity);
    File frame=new File(slots,"quick-2/frame");byte[] image=Files.readAllBytes(frame.toPath()),bad=image.clone();bad[bad.length-1]^=1;Files.write(frame.toPath(),bad);
    failedLoad("load:2","display mismatch");require(tree(save).equals(progress),"Failed reconstruction changed progress");require(!new File(slots,"rollback").exists(),"Failed load did not complete rollback");Files.write(frame.toPath(),image);
    checkpoint("load:2");checkpoint("recover");
    File other=new File(root,"saves/other/progress");other.getParentFile().mkdirs();Files.write(other.toPath(),"other game".getBytes(StandardCharsets.UTF_8));checkpoint("load:3");require(new String(Files.readAllBytes(other.toPath()),StandardCharsets.UTF_8).equals("other game"),"Another game's data changed");
    NativeBridge.pause(false);Thread.sleep(300);NativeBridge.pause(true);checkpoint("save:1");
    require("Running".equals(NativeBridge.status()),"Session did not continue after recovery");
    result.putString("stream","Three real replay slots, stop/restart, build mismatch, failed reconstruction rollback, undo, game isolation and continued execution: PASS\n");
   }
  }catch(Throwable error){result.putString("stream",android.util.Log.getStackTraceString(error));result.putBoolean("failed",true);}
  finally{
   releaseIo.countDown();
   if(activity!=null){MainActivity current=activity;test.runOnMainSync(current::finish);test.waitForIdleSync();}
   NativeBridge.stop();GameDataImport.removeTree(root);if(owned){GameDataImport.removeTree(save);GameDataImport.removeTree(slots);GameDataImport.removeTree(rescues);}
  }
  test.finish(result.getBoolean("failed")?1:0,result);
 }
 private static void processPersistence(Instrumentation test,Bundle arguments){
  Bundle result=new Bundle();File root=new File(test.getTargetContext().getCacheDir(),"checkpoint-process-test"),save=new File(root,"saves/fixture");
  boolean seed=arguments.getString("mode").endsWith("seed");
  try{
   File game=new File(arguments.getString("game"));
   if(seed){GameDataImport.removeTree(root);prepare(game,save);}
   else require(new File(root,"ready").isFile(),"Missing process-kill fixture");
   NativeBridge.start(game.getAbsolutePath(),save.getAbsolutePath());
   if(seed){
    waitFor(()->"Running".equals(NativeBridge.status()),"Persistence fixture did not start");Thread.sleep(1000);
    for(int slot=1;slot<=3;slot++){NativeBridge.pause(true);checkpoint("save:"+slot);NativeBridge.pause(false);Thread.sleep(100);}
    Files.write(new File(root,"ready").toPath(),new byte[]{1});
    // The host force-stops this process while the native guest is still running.
    Thread.sleep(60000);throw new AssertionError("Host did not force-stop the seeded process");
   }
   NativeBridge.pause(true);for(int slot=1;slot<=3;slot++)checkpoint("load:"+slot);
   result.putString("stream","All three checkpoints reconstructed in a new Android process after force-stop: PASS\n");
  }catch(Throwable error){result.putString("stream",android.util.Log.getStackTraceString(error));result.putBoolean("failed",true);}
  finally{NativeBridge.stop();if(!seed)GameDataImport.removeTree(root);}
  test.finish(result.getBoolean("failed")?1:0,result);
 }
}
