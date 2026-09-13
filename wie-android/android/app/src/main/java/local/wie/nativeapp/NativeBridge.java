package local.wie.nativeapp;
final class NativeBridge {
 public static native void textInputMode(boolean korean);
    public static native boolean isKoreanInput();
 static {System.loadLibrary("wie_android");}
 static native void start(String archive,String saveRoot);
 static native void stop();
 static native String checkpoint(String action);
 static native String manualRescue();
 static native void pause(boolean paused);
 static native void speed(int thousandths);
 static native void key(String key,boolean down,long id,long eventNs,long listenerNs);
 static native long frame(int[] pixels,long[] metadata);
 static native void tracePoint(int kind,long id,long value);
 static native void traceControl(boolean active,String path);
 static native String status();
 static native long paints();
 static native byte[] audio();
}
