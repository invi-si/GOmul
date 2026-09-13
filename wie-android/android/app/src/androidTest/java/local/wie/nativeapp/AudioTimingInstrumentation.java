package local.wie.nativeapp;

import android.app.Instrumentation;
import android.os.Bundle;

/** Verify conversion between audio's guest timeline and Handler deadlines. */
final class AudioTimingInstrumentation {
 static void run(Instrumentation runner){Bundle result=new Bundle();try{
  for(int rate:new int[]{250,500,1000,1250,1500,1750,2000}){
   if(AudioOutput.guestElapsed(1000,rate)!=rate)throw new AssertionError("elapsed "+rate);
   if(AudioOutput.hostDelay(rate,rate)!=1000)throw new AssertionError("deadline "+rate);
   if(AudioOutput.hostDelay(0,rate)!=0)throw new AssertionError("zero deadline");
   for(long delay:new long[]{1,17,999,2000}){
    long host=AudioOutput.hostDelay(delay,rate);
    if(AudioOutput.guestElapsed(host,rate)<delay)throw new AssertionError("early audio deadline");
   }
  }
  result.putString("stream","Audio timing: PASS\n");runner.finish(0,result);
 }catch(Throwable e){result.putString("stream",android.util.Log.getStackTraceString(e));runner.finish(1,result);}}
}
