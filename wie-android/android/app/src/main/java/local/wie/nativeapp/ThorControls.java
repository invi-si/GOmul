package local.wie.nativeapp;

import android.content.SharedPreferences;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;
import java.util.HashMap;
import java.util.Map;

final class ThorControls {
 interface Host {void key(String key,boolean down);void settings();}
 static final int[] BUTTONS={KeyEvent.KEYCODE_BUTTON_A,KeyEvent.KEYCODE_BUTTON_B,KeyEvent.KEYCODE_BUTTON_X,KeyEvent.KEYCODE_BUTTON_Y,
  KeyEvent.KEYCODE_BUTTON_L1,KeyEvent.KEYCODE_BUTTON_R1,KeyEvent.KEYCODE_BUTTON_L2,KeyEvent.KEYCODE_BUTTON_R2,
  KeyEvent.KEYCODE_BUTTON_THUMBL,KeyEvent.KEYCODE_BUTTON_THUMBR,KeyEvent.KEYCODE_BUTTON_START};
 static final String[] LABELS={"A","B","X","Y","L1","R1","L2","R2","왼쪽 스틱 누르기","오른쪽 스틱 누르기","Start"};
 static final String[] DEFAULTS={"OK","CLR","5","0","L","R","1","3","*","#","CALL"};
 static final String[] CHOICES={"OK","CLR","L","R","CALL","1","2","3","4","5","6","7","8","9","0","*","#","NONE"};
 private final Host host;
 private final ControllerState state;
 private final Map<String,Integer> axes=new HashMap<>();
 ThorControls(Host host){this.host=host;state=new ControllerState(host::key);}
 static String mapping(SharedPreferences prefs,int button){for(int i=0;i<BUTTONS.length;i++)if(BUTTONS[i]==button)return prefs.getString("controller-"+button,DEFAULTS[i]);return null;}
 void release(){state.release();axes.clear();}
 boolean key(KeyEvent event,SharedPreferences prefs){
  int code=event.getKeyCode();boolean down=event.getAction()==KeyEvent.ACTION_DOWN;
  if(code==KeyEvent.KEYCODE_BUTTON_SELECT||code==KeyEvent.KEYCODE_MENU){if(down&&event.getRepeatCount()==0){release();host.settings();}return true;}
  String guest;
  switch(code){case KeyEvent.KEYCODE_DPAD_UP:guest="UP";break;case KeyEvent.KEYCODE_DPAD_DOWN:guest="DOWN";break;
   case KeyEvent.KEYCODE_DPAD_LEFT:guest="LEFT";break;case KeyEvent.KEYCODE_DPAD_RIGHT:guest="RIGHT";break;
   case KeyEvent.KEYCODE_DPAD_CENTER:guest="OK";break;default:guest=mapping(prefs,code);}
  if(guest==null)return false;
  if(event.getAction()==KeyEvent.ACTION_DOWN||event.getAction()==KeyEvent.ACTION_UP)state.set(event.getDeviceId()+":key:"+code,guest,down);
  return true;
 }
 private void axis(MotionEvent event,int axis,String negative,String positive){
  String source=event.getDeviceId()+":axis:"+axis;int value=ControllerState.axis(axes.getOrDefault(source,0),event.getAxisValue(axis));axes.put(source,value);
  state.set(source+":negative",negative,value<0);state.set(source+":positive",positive,value>0);
 }
 boolean motion(MotionEvent event,SharedPreferences prefs){
  if((event.getSource()&InputDevice.SOURCE_JOYSTICK)!=InputDevice.SOURCE_JOYSTICK)return false;
  axis(event,MotionEvent.AXIS_X,"LEFT","RIGHT");axis(event,MotionEvent.AXIS_Y,"UP","DOWN");
  axis(event,MotionEvent.AXIS_HAT_X,"LEFT","RIGHT");axis(event,MotionEvent.AXIS_HAT_Y,"UP","DOWN");
  trigger(event,prefs,MotionEvent.AXIS_LTRIGGER,KeyEvent.KEYCODE_BUTTON_L2);
  trigger(event,prefs,MotionEvent.AXIS_RTRIGGER,KeyEvent.KEYCODE_BUTTON_R2);return true;
 }
 private void trigger(MotionEvent event,SharedPreferences prefs,int axis,int button){
  String source=event.getDeviceId()+":axis:"+axis;int previous=axes.getOrDefault(source,0),value=ControllerState.axis(previous,event.getAxisValue(axis));axes.put(source,value);
  String guest=mapping(prefs,button);
  state.set(source,guest,value>0);
 }
}
