package local.wie.nativeapp;
final class NativeBridge {
 static {System.loadLibrary("wie_android");}
 static native void start(String archive,String saveRoot);
 static native void stop();
 static native String checkpoint(String action);
 static native void pause(boolean paused);
 static native void key(String key,boolean down);
 static native long frame(int[] pixels);
 static native String status();
 static native long paints();
 static native byte[] audio();
}
