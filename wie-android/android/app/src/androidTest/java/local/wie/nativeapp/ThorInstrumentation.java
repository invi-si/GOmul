package local.wie.nativeapp;
import android.app.Instrumentation;
import android.os.Bundle;
import android.content.SharedPreferences;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.InputDevice;
import java.util.*;

final class ThorInstrumentation {
 static void run(Instrumentation instrumentation){Bundle result=new Bundle();
  SharedPreferences prefs=instrumentation.getTargetContext().getSharedPreferences("thor-controller-test",0);
  try{
   prefs.edit().clear().commit();List<String> events=new ArrayList<>();
   ThorControls controls=new ThorControls(new ThorControls.Host(){public void key(String key,boolean down){events.add(key+":"+down);}public void settings(){events.add("settings");}});
   controls.key(new KeyEvent(KeyEvent.ACTION_DOWN,KeyEvent.KEYCODE_BUTTON_A),prefs);
   controls.key(new KeyEvent(KeyEvent.ACTION_DOWN,KeyEvent.KEYCODE_BUTTON_A),prefs);
   controls.key(new KeyEvent(KeyEvent.ACTION_UP,KeyEvent.KEYCODE_BUTTON_A),prefs);
   if(!events.equals(Arrays.asList("OK:true","OK:false")))throw new AssertionError(events);
   events.clear();prefs.edit().putString("controller-"+KeyEvent.KEYCODE_BUTTON_A,"7").commit();
   controls.key(new KeyEvent(KeyEvent.ACTION_DOWN,KeyEvent.KEYCODE_BUTTON_A),prefs);controls.release();
   if(!events.equals(Arrays.asList("7:true","7:false")))throw new AssertionError(events);
   events.clear();controls.motion(motion(.8f,0),prefs);controls.motion(motion(.4f,0),prefs);controls.motion(motion(0,0),prefs);
   if(!events.equals(Arrays.asList("RIGHT:true","RIGHT:false")))throw new AssertionError(events);
   events.clear();controls.key(new KeyEvent(KeyEvent.ACTION_DOWN,KeyEvent.KEYCODE_DPAD_UP),prefs);
   controls.key(new KeyEvent(KeyEvent.ACTION_DOWN,KeyEvent.KEYCODE_BUTTON_SELECT),prefs);
   if(!events.equals(Arrays.asList("UP:true","UP:false","settings")))throw new AssertionError(events);
   result.putString("stream","Thor physical button, repeat suppression, remapping, joystick and menu-release tests: PASS\n");instrumentation.finish(0,result);
  }catch(Throwable error){result.putString("stream",android.util.Log.getStackTraceString(error));instrumentation.finish(1,result);}
  finally{prefs.edit().clear().commit();}
 }
 private static MotionEvent motion(float x,float y){
  MotionEvent.PointerProperties property=new MotionEvent.PointerProperties();property.id=0;
  MotionEvent.PointerCoords coords=new MotionEvent.PointerCoords();coords.setAxisValue(MotionEvent.AXIS_X,x);coords.setAxisValue(MotionEvent.AXIS_Y,y);
  return MotionEvent.obtain(0,0,MotionEvent.ACTION_MOVE,1,new MotionEvent.PointerProperties[]{property},new MotionEvent.PointerCoords[]{coords},0,0,1,1,0,0,InputDevice.SOURCE_JOYSTICK,0);
 }
}
