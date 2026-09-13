package local.wie.nativeapp;

import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;

/** One guest transition even when a stick, hat and button hold the same key. */
final class ControllerState {
 interface Sink {void key(String key,boolean down);}
 private final Map<String,String> sources=new HashMap<>();
 private final Sink sink;
 ControllerState(Sink sink){this.sink=sink;}
 void set(String source,String key,boolean down){
  Set<String> before=new HashSet<>(sources.values());
  if(down&&!key.equals("NONE"))sources.put(source,key);else sources.remove(source);
  Set<String> after=new HashSet<>(sources.values());
  for(String old:before)if(!after.contains(old))sink.key(old,false);
  for(String next:after)if(!before.contains(next))sink.key(next,true);
 }
 void release(){Set<String> keys=new HashSet<>(sources.values());sources.clear();for(String key:keys)sink.key(key,false);}
 static int axis(int previous,float value){
  if(value>=0.5f)return 1;if(value<=-0.5f)return -1;
  if(previous==1&&value>0.35f)return 1;if(previous==-1&&value<-0.35f)return -1;return 0;
 }
}
