package local.wie.nativeapp;
import java.util.*;
public final class ControllerStateTest {
 public static void main(String[] args){
  List<String> events=new ArrayList<>();ControllerState state=new ControllerState((key,down)->events.add(key+":"+down));
  state.set("dpad","UP",true);state.set("stick","UP",true);state.set("dpad","UP",true);state.set("dpad","UP",false);
  if(!events.equals(Arrays.asList("UP:true")))throw new AssertionError(events);
  state.set("stick","UP",false);if(!events.equals(Arrays.asList("UP:true","UP:false")))throw new AssertionError(events);
  events.clear();state.set("button","5",true);state.set("button","1",true);state.release();state.release();
  if(!events.equals(Arrays.asList("5:true","5:false","1:true","1:false")))throw new AssertionError(events);
  if(ControllerState.axis(0,.4f)!=0||ControllerState.axis(0,.6f)!=1||ControllerState.axis(1,.4f)!=1||ControllerState.axis(1,.2f)!=0||ControllerState.axis(1,-.6f)!=-1)throw new AssertionError("stick hysteresis");
  System.out.println("Controller overlap, repeats, remap/release and stick hysteresis: PASS");
 }
}
