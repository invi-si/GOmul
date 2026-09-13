// GOmul provenance: gomul-component:5b3221ec-dfc4-4817-9693-367ec65b6180 (android-launcher); see PROVENANCE.json.
// GOmul contributions: Copyright (c) 2026 invi-si. SPDX-License-Identifier: MIT
package local.wie.nativeapp;

import android.app.*;
import android.os.*;
import android.content.*;
import android.content.res.Configuration;
import android.database.Cursor;
import android.provider.OpenableColumns;
import android.net.Uri;
import android.graphics.*;
import android.graphics.drawable.*;
import android.content.pm.ActivityInfo;
import android.content.res.ColorStateList;
import android.view.*;
import android.widget.*;
import android.util.Log;
import java.io.*;
import java.security.MessageDigest;
import java.util.*;
import java.util.concurrent.*;

public final class MainActivity extends Activity implements Choreographer.FrameCallback {
 private final ExecutorService io=Executors.newSingleThreadExecutor();
 private final Set<String> held=new HashSet<>();
 private LinearLayout root;private TextView status;private GameView gameView;private AudioOutput audio;
 private File dataImportGame;
 private File rescueExport;private boolean rescueShown=false,rescueBusy=false;private long rescueSessionStart=0;
 private volatile int playbackRate=1000;private volatile long lastFramePickup;
 private File activeGame;private volatile boolean foreground=false;private boolean starting=false;private final int[] pixels=new int[1024*1024];
 private long lastReport=0,lastPaints=0;
 private boolean tracing=false;
 private boolean koreanKeypad=false;private volatile boolean numericDirections=false;private Button languageKey;
 private final Map<String,Button> keypadButtons=new HashMap<>();
 private final ThorControls controller=new ThorControls(new ThorControls.Host(){
  public void key(String code,boolean down){MainActivity.this.key(code,down);}
  public void settings(){showGameSettings();}
 });
 private final android.hardware.input.InputManager.InputDeviceListener controllerListener=new android.hardware.input.InputManager.InputDeviceListener(){
  public void onInputDeviceAdded(int id){}public void onInputDeviceChanged(int id){releaseKeys();}public void onInputDeviceRemoved(int id){releaseKeys();}
 };
 @Override public boolean dispatchKeyEvent(KeyEvent event){
  if(BuildConfig.THOR&&activeGame!=null&&!starting&&!checkpointBusy&&controller.key(event,gamePreferences(activeGame)))return true;
  return super.dispatchKeyEvent(event);
 }
 @Override public boolean onGenericMotionEvent(MotionEvent event){
  if(BuildConfig.THOR&&activeGame!=null&&!starting&&!checkpointBusy&&controller.motion(event,gamePreferences(activeGame)))return true;
  return super.onGenericMotionEvent(event);
 }
 private long nextInput=0,nextDraw=0;
 private final long[] frameMetadata=new long[2];
 private final Map<String,Long> pressIds=new HashMap<>();
 @Override protected void onNewIntent(Intent intent){super.onNewIntent(intent);setIntent(intent);if(intent.hasExtra("timerCensus"))NativeBridge.tracePoint(200,0,intent.getBooleanExtra("timerCensus",false)?1:0);if(intent.hasExtra("tracePhase"))NativeBridge.tracePoint(94,intent.getIntExtra("tracePhase",0),0);if(intent.hasExtra("trace"))setTracing(intent.getBooleanExtra("trace",false));}
 private void setTracing(boolean enabled){
  if(enabled==tracing)return;
  if(enabled){NativeBridge.traceControl(true,"");tracing=true;}
  else{tracing=false;File output=new File(getExternalFilesDir(null),"input-trace.csv");io.execute(()->{NativeBridge.traceControl(false,output.getAbsolutePath());Log.i("GOmul-Trace","Saved "+output);});}
 }

 @Override public void onCreate(Bundle state){super.onCreate(state);if(BuildConfig.THOR)((android.hardware.input.InputManager)getSystemService(INPUT_SERVICE)).registerInputDeviceListener(controllerListener,new Handler(getMainLooper()));getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);audio=new AudioOutput(getCacheDir());if(Build.VERSION.SDK_INT>=33)getOnBackInvokedDispatcher().registerOnBackInvokedCallback(android.window.OnBackInvokedDispatcher.PRIORITY_DEFAULT,this::handleBack);showLibrary();
  if(Build.VERSION.SDK_INT>=36)getWindow().addOnFrameMetricsAvailableListener((window,metrics,dropped)->{
   if(tracing){long id=frameTimelineId(metrics);NativeBridge.tracePoint(27,id,metrics.getMetric(FrameMetrics.INTENDED_VSYNC_TIMESTAMP));NativeBridge.tracePoint(28,id,metrics.getMetric(FrameMetrics.VSYNC_TIMESTAMP));NativeBridge.tracePoint(29,id,metrics.getMetric(FrameMetrics.TOTAL_DURATION));NativeBridge.tracePoint(30,id,dropped);}
  },new Handler(getMainLooper()));
 }
 // Android 36 exposes this documented metric, but its @Metric IntDef omits it.
 // https://developer.android.com/reference/android/view/FrameMetrics#FRAME_TIMELINE_VSYNC_ID
 @android.annotation.SuppressLint("WrongConstant")
 @android.annotation.TargetApi(36)
 private static long frameTimelineId(FrameMetrics metrics){return metrics.getMetric(FrameMetrics.FRAME_TIMELINE_VSYNC_ID);}
 private static void copy(InputStream in,OutputStream out)throws IOException{byte[] b=new byte[65536];int n;while((n=in.read(b))!=-1)out.write(b,0,n);}
 private static byte[] readBytes(InputStream in)throws IOException{ByteArrayOutputStream out=new ByteArrayOutputStream();copy(in,out);return out.toByteArray();}
 private int dp(float value){return (int)(value*getResources().getDisplayMetrics().density+0.5f);}
 private LinearLayout column(){LinearLayout v=new LinearLayout(this);v.setOrientation(LinearLayout.VERTICAL);return v;}
 private static final int BACKGROUND=0xffb9bcc0, PANEL=0xffd6d8da, BORDER=0xff666a70, ACCENT=0xff30343a;
 private GradientDrawable rounded(int color,int radius){GradientDrawable d=new GradientDrawable();d.setColor(color);d.setCornerRadius(dp(radius));return d;}
 private Drawable metalFace(boolean center,boolean pressed){
  GradientDrawable rim=new GradientDrawable(GradientDrawable.Orientation.TOP_BOTTOM,new int[]{0xfff4f5f6,0xff85898e,0xff454a50});
  rim.setCornerRadius(dp(center?15:10));rim.setStroke(dp(1),0xff53585e);
  GradientDrawable face=new GradientDrawable(GradientDrawable.Orientation.TOP_BOTTOM,pressed?new int[]{0xff8c9299,0xffb6bdc3,0xffd2d7dc}:new int[]{0xfff7f8f9,0xffdfe2e5,0xffb6bbc1,0xffe3e6e9});
  face.setCornerRadius(dp(center?12:7));face.setStroke(dp(1),pressed?0xff717980:0xfff5f6f7);
  LayerDrawable layers=new LayerDrawable(new Drawable[]{rim,face});
  layers.setLayerInset(1,dp(center?4:2),dp(2),dp(center?4:2),dp(4));
  return layers;
 }
 private Drawable metalKey(boolean center){
  StateListDrawable states=new StateListDrawable();
  states.addState(new int[]{android.R.attr.state_pressed},metalFace(center,true));
  states.addState(new int[]{android.R.attr.state_focused},metalFace(center,true));
  states.addState(new int[]{},metalFace(center,false));return states;
 }
 private class MetalShell extends Drawable {
  final Paint metal=new Paint(Paint.ANTI_ALIAS_FLAG),grain=new Paint();
  @Override protected void onBoundsChange(Rect b){
   metal.setShader(new LinearGradient(b.left,0,b.right,0,new int[]{0xff686d73,0xffd9dcdf,0xffb6babf,0xffe2e4e6,0xff747a80},new float[]{0,.045f,.48f,.95f,1},Shader.TileMode.CLAMP));
   grain.setColor(0x0affffff);grain.setStrokeWidth(1);
  }
  @Override public void draw(Canvas canvas){Rect b=getBounds();canvas.drawRect(b,metal);for(int y=b.top;y<b.bottom;y+=4)canvas.drawLine(b.left,y,b.right,y,grain);}
  @Override public void setAlpha(int alpha){metal.setAlpha(alpha);}
  @Override public void setColorFilter(ColorFilter filter){metal.setColorFilter(filter);}
  @Override public int getOpacity(){return PixelFormat.OPAQUE;}
 }
 private LibraryGrid libraryGrid;
 private LibraryGrid.Position libraryPosition;
 private void setupRoot(){
  if(libraryGrid!=null){libraryPosition=libraryGrid.rememberPosition();libraryGrid=null;}
  root=column();root.setBackground(new MetalShell());root.setMotionEventSplittingEnabled(true);
  boolean playing=activeGame!=null;
  root.setOnApplyWindowInsetsListener((v,insets)->{
   int left,right,top,bottom;
   if(Build.VERSION.SDK_INT>=30){
    Insets safe=insets.getInsets(WindowInsets.Type.systemBars()|WindowInsets.Type.displayCutout());
    left=safe.left;right=safe.right;top=safe.top;bottom=safe.bottom;
   }else{left=insets.getSystemWindowInsetLeft();right=insets.getSystemWindowInsetRight();top=insets.getSystemWindowInsetTop();bottom=insets.getSystemWindowInsetBottom();}
   int margin=dp(playing?6:20);v.setPadding(left+margin,top+dp(playing?2:16),right+margin,bottom+dp(playing?4:12));return insets;
  });
  setContentView(root);updateSystemBars();root.requestApplyInsets();
 }
 private void updateSystemBars(){
  boolean playing=activeGame!=null;
  if(Build.VERSION.SDK_INT>=30){
   getWindow().setDecorFitsSystemWindows(false);
   WindowInsetsController controller=getWindow().getInsetsController();
   if(controller!=null){controller.setSystemBarsBehavior(WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);if(playing)controller.hide(WindowInsets.Type.systemBars());else controller.show(WindowInsets.Type.systemBars());}
  }else getWindow().getDecorView().setSystemUiVisibility(playing?View.SYSTEM_UI_FLAG_FULLSCREEN|View.SYSTEM_UI_FLAG_HIDE_NAVIGATION|View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY|View.SYSTEM_UI_FLAG_LAYOUT_STABLE|View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN|View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION:0);
 }
 @Override public void onWindowFocusChanged(boolean focused){super.onWindowFocusChanged(focused);if(!focused&&BuildConfig.THOR)releaseKeys();if(focused)updateSystemBars();}
 private Button button(String title,Runnable action){
  Button b=new Button(this);b.setText(title);b.setAllCaps(false);b.setTextColor(ACCENT);b.setTextSize(15);b.setGravity(Gravity.CENTER);b.setMinHeight(0);b.setMinimumHeight(0);b.setMinWidth(0);b.setMinimumWidth(0);b.setPadding(dp(14),dp(8),dp(14),dp(8));
  b.setBackground(metalKey(false));b.setShadowLayer(dp(0.6f),0,dp(0.7f),0xfffafbfc);b.setStateListAnimator(null);b.setOnClickListener(v->action.run());return b;
 }
 private TextView label(String text,int size){TextView t=new TextView(this);t.setText(text);t.setTextColor(0xff25292e);t.setTextSize(size);t.setPadding(dp(5),dp(5),dp(5),dp(5));return t;}
 private void addSpaced(LinearLayout parent,View child,int height){LinearLayout.LayoutParams params=new LinearLayout.LayoutParams(-1,dp(height));params.setMargins(0,dp(6),0,dp(6));parent.addView(child,params);}
 private File games(){File f=new File(getFilesDir(),"games");f.mkdirs();return f;}
 private void showLibrary(){
  new File(getFilesDir(),"active-game.txt").delete();
  activeGame=null;gameView=null;setRequestedOrientation(BuildConfig.THOR?ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE:ActivityInfo.SCREEN_ORIENTATION_UNSPECIFIED);setupRoot();
  TextView eyebrow=label(BuildConfig.THOR?"G O m u l   T H O R":"G O m u l   /   고 물",12);eyebrow.setTextColor(ACCENT);root.addView(eyebrow);
  addSpaced(root,button("＋  게임 추가",()->{if(starting)return;Intent i=new Intent(Intent.ACTION_OPEN_DOCUMENT);i.setType("*/*");i.addCategory(Intent.CATEGORY_OPENABLE);startActivityForResult(i,1);}),54);
  TextView hint=label(".jar / .zip 파일 추가 · 게임을 길게 눌러 관리",12);hint.setTextColor(0xff4d535b);root.addView(hint);
  libraryGrid=new LibraryGrid(this,games(),this::launch,this::gameOptions);
  root.addView(libraryGrid,new LinearLayout.LayoutParams(-1,0,1));
  libraryGrid.restorePosition(libraryPosition);
  File lastRescue=RescueReports.latest(getFilesDir(),null);if(lastRescue!=null)addSpaced(root,button("최근 오류 보고서 내보내기",()->exportRescue(lastRescue)),44);
  addSpaced(root,button("오픈소스 라이선스",()->{try(InputStream in=getAssets().open("LICENSES.txt")){new AlertDialog.Builder(this).setTitle("라이선스").setMessage(new String(readBytes(in),java.nio.charset.StandardCharsets.UTF_8)).setPositiveButton("닫기",null).show();}catch(IOException e){error(e);}}),48);
 }
 private void gameOptions(File game){
  if(starting)return;
  new AlertDialog.Builder(this).setTitle(game.getName()).setItems(new String[]{"저장 데이터 가져오기","저장 데이터 초기화","게임 삭제"},(dialog,which)->{
   if(which==0){chooseData(game);return;}
   if(which==1){resetGameSave(game);return;}
   new AlertDialog.Builder(this).setTitle("게임을 삭제할까요?").setMessage("목록에서 삭제할 게임: "+game.getName()+"\n저장 데이터, 빠른 저장 및 오류 보고서는 유지됩니다. 원본 파일은 삭제하지 않습니다.")
    .setNegativeButton("취소",null).setPositiveButton("삭제",(confirmation,button)->{
     starting=true;io.execute(()->{
      try{
       java.nio.file.Files.delete(game.toPath());
       File[] remaining=game.getParentFile().listFiles();if(remaining!=null&&remaining.length==0)game.getParentFile().delete();
       runOnUiThread(()->{starting=false;showLibrary();Toast.makeText(this,"게임을 삭제했습니다",Toast.LENGTH_SHORT).show();});
      }catch(Exception failure){runOnUiThread(()->{starting=false;error(failure);});}
     });
    }).show();
  }).show();
 }
 private void resetGameSave(File game){
  new AlertDialog.Builder(this).setTitle("저장 데이터를 초기화할까요?")
   .setMessage("초기화할 게임: "+game.getName()+"\n진행 상황, 빠른 저장 및 불러오기 복구 데이터가 삭제됩니다. 게임 파일, 화면·속도 설정 및 가상 전화번호는 유지됩니다. 함께 제공된 초기 데이터는 다음 실행 시 다시 가져옵니다. 이 작업은 되돌릴 수 없습니다.")
   .setNegativeButton("취소",null).setPositiveButton("저장 데이터 초기화",(dialog,which)->{
    if(starting)return;starting=true;io.execute(()->{
     try{GameSaveReset.reset(getFilesDir(),game.getParentFile().getName());runOnUiThread(()->{starting=false;Toast.makeText(this,"초기화했습니다. 게임을 열면 처음부터 시작합니다.",Toast.LENGTH_LONG).show();});}
     catch(Exception failure){runOnUiThread(()->{starting=false;error(failure);});}
    });
   }).show();
 }
 private File saveFolder(File game){return new File(getFilesDir(),"saves/"+game.getParentFile().getName());}
 private void chooseData(File game){
  if(starting)return;dataImportGame=game;
  new AlertDialog.Builder(this).setTitle("저장 데이터 가져오기").setItems(new String[]{"데이터 ZIP 선택","데이터 폴더 선택"},(dialog,which)->{
   Intent intent=new Intent(which==0?Intent.ACTION_OPEN_DOCUMENT:Intent.ACTION_OPEN_DOCUMENT_TREE);
   if(which==0){intent.setType("*/*");intent.addCategory(Intent.CATEGORY_OPENABLE);}startActivityForResult(intent,which==0?2:3);
  }).show();
 }
 private GameDataImport.Plan readDataFolder(File game,Uri tree)throws Exception {
  String pid=GameDataImport.pid(game);if(pid==null)throw new IOException("저장 데이터를 가져오려면 app_info가 포함된 LGT 게임 ZIP이 필요합니다.");
  String document=android.provider.DocumentsContract.getTreeDocumentId(tree);
  Uri children=android.provider.DocumentsContract.buildChildDocumentsUriUsingTree(tree,document);
  Map<String,byte[]> files=new LinkedHashMap<>();String phone=null;int total=0;
  String[] columns={android.provider.DocumentsContract.Document.COLUMN_DOCUMENT_ID,android.provider.DocumentsContract.Document.COLUMN_DISPLAY_NAME};
  try(Cursor cursor=getContentResolver().query(children,columns,null,null,null)){
   if(cursor==null)throw new IOException("선택한 폴더를 읽을 수 없습니다.");
   while(cursor.moveToNext()){
    String name=cursor.getString(1);if(!GameDataImport.dataName(name)&&!name.equals("gomul.properties"))continue;
    Uri uri=android.provider.DocumentsContract.buildDocumentUriUsingTree(tree,cursor.getString(0));byte[] bytes;
    try(InputStream in=getContentResolver().openInputStream(uri)){if(in==null)throw new IOException("파일을 읽을 수 없습니다: "+name);bytes=GameDataImport.read(in,GameDataImport.MAX_DATA);}
    total+=bytes.length;if(total>GameDataImport.MAX_DATA)throw new IOException("데이터 용량이 너무 큽니다.");
    if(name.equals("gomul.properties")){Properties props=new Properties();props.load(new ByteArrayInputStream(bytes));phone=props.getProperty("phoneNumber");}
    else {if(files.put(name,bytes)!=null)throw new IOException("같은 이름의 데이터 파일이 있습니다.");}
   }
  }
  if(files.isEmpty())throw new IOException("savedata와 함께 제공된 파일이 들어 있는 폴더를 선택하세요.");
  return new GameDataImport.Plan(pid,files,phone);
 }
 private void finishDataImport(File game,GameDataImport.Plan plan,boolean replace){
  if(plan.phone!=null){installData(game,plan,plan.phone,replace);return;}
  EditText number=new EditText(this);number.setInputType(android.text.InputType.TYPE_CLASS_PHONE);number.setHint("가상 전화번호 11자리");
  new AlertDialog.Builder(this).setTitle("저장 데이터 발견").setMessage("데이터 제공자가 안내한 전화번호를 입력하세요. 게임 안에서 사용할 가상 전화번호입니다.").setView(number)
   .setPositiveButton("가져와서 실행",(dialog,which)->installData(game,plan,number.getText().toString(),replace)).setNegativeButton("취소",null).show();
 }
 private void installData(File game,GameDataImport.Plan plan,String phone,boolean replace){
  starting=true;io.execute(()->{try{GameDataImport.install(saveFolder(game),plan,phone,replace);runOnUiThread(()->{starting=false;Toast.makeText(this,"저장 데이터를 가져왔습니다",Toast.LENGTH_SHORT).show();launch(game);});}
   catch(Exception error){runOnUiThread(()->{starting=false;error(error);});}});
 }
 private void offerDataImport(File game,GameDataImport.Plan plan,boolean separate){
  if(plan==null){if(separate)error(new IOException("함께 제공된 저장 데이터가 없습니다."));else launch(game);return;}
  if(saveFolder(game).exists()){
   if(!separate){launch(game);return;}
   new AlertDialog.Builder(this).setTitle("저장 데이터를 교체할까요?").setMessage("현재 저장 데이터를 백업한 뒤 가져옵니다. 다른 게임에는 영향을 주지 않습니다.")
    .setPositiveButton("백업 후 가져오기",(dialog,which)->finishDataImport(game,plan,true)).setNegativeButton("취소",null).show();return;
  }
  finishDataImport(game,plan,false);
 }
 @Override protected void onActivityResult(int request,int result,Intent data){
  super.onActivityResult(request,result,data);
  if(request==4){File prepared=rescueExport;rescueExport=null;rescueBusy=false;
   if(prepared!=null){if(result==RESULT_OK&&data!=null&&data.getData()!=null){Uri destination=data.getData();io.execute(()->{try(InputStream in=new FileInputStream(prepared);OutputStream out=getContentResolver().openOutputStream(destination)){if(out==null)throw new IOException("오류 보고서를 내보낼 수 없습니다");byte[] buffer=new byte[65536];int n;while((n=in.read(buffer))!=-1)out.write(buffer,0,n);runOnUiThread(()->Toast.makeText(this,"오류 보고서를 내보냈습니다",Toast.LENGTH_LONG).show());}catch(Exception e){runOnUiThread(()->rescueError(e));}finally{prepared.delete();}});}else prepared.delete();}return;}

  if(result!=RESULT_OK||data==null||request<1||request>3)return;Uri uri=data.getData();if(uri==null)return;
  File selected=dataImportGame;starting=true;Toast.makeText(this,"파일을 읽는 중…",Toast.LENGTH_SHORT).show();
  io.execute(()->{try{
   File game;GameDataImport.Plan plan;
   if(request==1){game=importGame(uri);plan=GameDataImport.inspect(game,game);}
   else {if(selected==null)throw new IOException("데이터를 가져올 게임을 다시 선택하세요.");game=selected;
    if(request==3)plan=readDataFolder(game,uri);
    else {File temporary=File.createTempFile("data-import-",".zip",getCacheDir());try{
     try(InputStream in=getContentResolver().openInputStream(uri)){if(in==null)throw new IOException("데이터 ZIP을 열 수 없습니다.");java.nio.file.Files.write(temporary.toPath(),GameDataImport.read(in,GameDataImport.MAX_DATA));}
     plan=GameDataImport.inspect(game,temporary);
    }finally{temporary.delete();}}
   }
   File ready=game;GameDataImport.Plan pending=plan;runOnUiThread(()->{starting=false;showLibrary();offerDataImport(ready,pending,request!=1);});
  }catch(Exception error){runOnUiThread(()->{starting=false;error(error);});}});
 }
 private File importGame(Uri uri)throws Exception {
  String name="game.jar";try(Cursor c=getContentResolver().query(uri,new String[]{OpenableColumns.DISPLAY_NAME},null,null,null)){if(c!=null&&c.moveToFirst()){String displayName=c.getString(0);if(displayName!=null&&!displayName.isEmpty())name=displayName;}}
  name=new File(name).getName().replaceAll("[^\\p{L}\\p{N}._ -]","_");String lower=name.toLowerCase(Locale.ROOT);if(!lower.endsWith(".jar")&&!lower.endsWith(".zip"))throw new IOException(".jar 또는 .zip 게임 파일을 선택하세요.");
  File temp=File.createTempFile("import-",".tmp",getCacheDir());MessageDigest hash=MessageDigest.getInstance("SHA-256");
  try {try(InputStream in=getContentResolver().openInputStream(uri);OutputStream out=new FileOutputStream(temp)){if(in==null)throw new IOException("게임 파일을 열 수 없습니다");byte[] b=new byte[65536];long total=0;int n;while((n=in.read(b))!=-1){total+=n;if(total>128L*1024*1024)throw new IOException("128MB 이하의 게임 파일을 선택하세요.");hash.update(b,0,n);out.write(b,0,n);}}
   StringBuilder id=new StringBuilder();for(byte b:hash.digest())id.append(String.format(Locale.ROOT,"%02x",b));File folder=new File(games(),id.toString());if(!folder.exists()&&!folder.mkdirs())throw new IOException("게임 폴더를 만들 수 없습니다");File destination=new File(folder,name);if(destination.exists())return destination;
   if(lower.endsWith(".zip"))GameDataImport.normalize(temp);java.nio.file.Files.move(temp.toPath(),destination.toPath(),java.nio.file.StandardCopyOption.ATOMIC_MOVE);return destination;
  }finally{temp.delete();}
 }
 private void launch(File file){
  if(starting)return;
  File save=saveFolder(file);
  if(new File(save,"companion-imported").isFile()){launchPrepared(file);return;}
  starting=true;
  io.execute(()->{try{
   GameDataImport.prepareBundledIdentity(file,save);
   GameDataImport.Plan plan=GameDataImport.inspect(file,file);
   if(plan!=null&&plan.phone==null){
    File identity=new File(save,"phone-number.txt");
    if(identity.isFile())plan=new GameDataImport.Plan(plan.pid,plan.files,new String(java.nio.file.Files.readAllBytes(identity.toPath()),java.nio.charset.StandardCharsets.UTF_8).trim());
   }
   GameDataImport.Plan pending=plan;boolean keepProgress=plan!=null&&GameDataImport.hasProgress(save);
   runOnUiThread(()->{starting=false;if(pending==null||keepProgress)launchPrepared(file);else finishDataImport(file,pending,save.exists());});
  }catch(Exception error){runOnUiThread(()->{starting=false;error(error);});}});
 }
 private void launchPrepared(File file){if(starting)return;starting=true;activeGame=file;koreanKeypad=false;rescueShown=false;rescueSessionStart=System.currentTimeMillis();showGame();status.setText("불러오는 중…");audio.stopAll();io.execute(()->{try{
   numericDirections=GameDataImport.isInotiaCompanionBundle(file);
   java.nio.file.Files.write(new File(getFilesDir(),"active-game.txt").toPath(),file.getParentFile().getName().getBytes(java.nio.charset.StandardCharsets.UTF_8));
   if(new File(getFilesDir(),"mac-checkpoints.json").isFile()){
    String requestId=UUID.randomUUID().toString();
    org.json.JSONObject result=checkpointRequest("POST","",new org.json.JSONObject().put("id",requestId).put("action","boot").put("game",file.getParentFile().getName()));
    long deadline=SystemClock.elapsedRealtime()+180000;
    while(result.optBoolean("pending")){
     if(SystemClock.elapsedRealtime()>deadline)throw new IOException("시작 상태를 불러오는 시간이 초과되었습니다.");
     Thread.sleep(750);result=checkpointRequest("GET",requestId,null);
    }
    if(result.has("error"))throw new IOException(result.getString("error"));
    if(!result.optBoolean("startNormally")){runOnUiThread(()->starting=false);return;}
   }
   writeDisplayOptions(file);
   NativeBridge.start(file.getAbsolutePath(),new File(getFilesDir(),"saves/"+file.getParentFile().getName()).getAbsolutePath());applySpeed(playbackPreferences().getInt("speed",1000));NativeBridge.pause(!foreground);runOnUiThread(()->starting=false);}catch(Exception e){runOnUiThread(()->{starting=false;error(e);});}});}
 private android.content.SharedPreferences gamePreferences(File game){return getSharedPreferences("game-settings-"+game.getParentFile().getName(),MODE_PRIVATE);}
 private void writeDisplayOptions(File game)throws IOException{
  android.content.SharedPreferences p=gamePreferences(game);File dir=new File(getFilesDir(),"saves/"+game.getParentFile().getName());dir.mkdirs();File options=new File(dir,"display-options");
  if(p.getBoolean("customDisplay",false))java.nio.file.Files.write(options.toPath(),(p.getInt("width",240)+" "+p.getInt("height",320)+" "+(p.getBoolean("full",false)?1:0)).getBytes(java.nio.charset.StandardCharsets.UTF_8));
  else java.nio.file.Files.deleteIfExists(options.toPath());
 }
 private TextView dialogLabel(String text,int size){TextView view=label(text,size);view.setTextColor(0xffeeeeee);return view;}
 private void showGameSettings(){
  if(activeGame==null||starting||checkpointBusy)return;releaseKeys();
  new AlertDialog.Builder(this).setTitle("게임 설정").setItems(BuildConfig.THOR?new String[]{"빠른 저장","빠른 불러오기","화면 크기","실행 속도","오류 보고서 만들기 (ZIP)","컨트롤러 버튼 설정"}:new String[]{"빠른 저장","빠른 불러오기","화면 크기","실행 속도","오류 보고서 만들기 (ZIP)"},(d,which)->{
   if(which==0)checkpoint("save");else if(which==1)checkpoint("load");else if(which==2)showDisplaySettings();else if(which==3)showSpeedSettings();else if(which==4)createManualRescue();else showControllerSettings();
  }).setNegativeButton("닫기",null).show();
 }
 private static String keyLabel(String key){switch(key){case "UP":return "위";case "DOWN":return "아래";case "LEFT":return "왼쪽";case "RIGHT":return "오른쪽";case "OK":return "확인";case "CLR":return "지움";case "CALL":return "통화";case "L":return "좌측 키";case "R":return "우측 키";case "NONE":return "사용 안 함";default:return key;}}
 private void showControllerSettings(){
  android.content.SharedPreferences prefs=gamePreferences(activeGame);String[] labels=new String[ThorControls.BUTTONS.length];
  for(int i=0;i<labels.length;i++)labels[i]=ThorControls.LABELS[i]+" → "+keyLabel(ThorControls.mapping(prefs,ThorControls.BUTTONS[i]));
  new AlertDialog.Builder(this).setTitle("이 게임의 컨트롤러 설정").setItems(labels,(dialog,index)->{
   String[] choices=new String[ThorControls.CHOICES.length];for(int i=0;i<choices.length;i++)choices[i]=keyLabel(ThorControls.CHOICES[i]);
   String current=ThorControls.mapping(prefs,ThorControls.BUTTONS[index]);int selected=java.util.Arrays.asList(ThorControls.CHOICES).indexOf(current);
   new AlertDialog.Builder(this).setTitle(ThorControls.LABELS[index]+"에 연결할 휴대폰 키").setSingleChoiceItems(choices,selected,(choice,item)->{
    releaseKeys();prefs.edit().putString("controller-"+ThorControls.BUTTONS[index],ThorControls.CHOICES[item]).apply();choice.dismiss();showControllerSettings();
   }).setNegativeButton("취소",null).show();
  }).setNeutralButton("기본값으로 초기화",(dialog,index)->{releaseKeys();android.content.SharedPreferences.Editor editor=prefs.edit();for(int button:ThorControls.BUTTONS)editor.remove("controller-"+button);editor.apply();})
   .setNegativeButton("닫기",null).show();
 }
 private android.content.SharedPreferences playbackPreferences(){return getSharedPreferences("playback-settings",MODE_PRIVATE);}
 private void applySpeed(int rate){playbackRate=rate;lastFramePickup=0;audio.speed(rate);NativeBridge.speed(rate);}
 private long pickupFrame(long nanos){if(playbackRate>1000&&lastFramePickup!=0&&nanos-lastFramePickup<33_333_333L)return 0;lastFramePickup=nanos;return NativeBridge.frame(pixels,frameMetadata);}
 private void showSpeedSettings(){
  android.content.SharedPreferences p=playbackPreferences();LinearLayout box=new LinearLayout(this);box.setOrientation(LinearLayout.VERTICAL);box.setPadding(dp(20),dp(12),dp(20),dp(12));
  TextView value=dialogLabel("",16);box.addView(value);SeekBar slider=new SeekBar(this);slider.setMax(11);slider.setProgress(Math.max(0,Math.min(11,p.getInt("speed",1000)/250-1)));box.addView(slider);
  value.setText(String.format(java.util.Locale.US,"%.2f×",(slider.getProgress()+1)*0.25));
  slider.setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener(){public void onStartTrackingTouch(SeekBar s){}public void onStopTrackingTouch(SeekBar s){}public void onProgressChanged(SeekBar s,int progress,boolean user){value.setText(String.format(java.util.Locale.US,"%.2f×",(progress+1)*0.25));}});
  box.addView(dialogLabel("모든 게임에 같은 속도가 적용되며 앱을 다시 열어도 유지됩니다. 2배 초과는 실험용입니다. 1배보다 빠르게 실행하면 화면 갱신을 줄이고 소리도 빨라집니다. 일부 소리는 재생되지 않을 수 있으며, 실제 속도는 기기 성능에 따라 달라집니다.",12));
  new AlertDialog.Builder(this).setTitle("실행 속도").setView(box).setPositiveButton("적용",(d,w)->{int rate=(slider.getProgress()+1)*250;p.edit().putInt("speed",rate).apply();applySpeed(rate);}).setNeutralButton("1배로 초기화",(d,w)->{p.edit().putInt("speed",1000).apply();applySpeed(1000);}).setNegativeButton("취소",null).show();
 }
 private void showDisplaySettings(){
  File game=activeGame;android.content.SharedPreferences p=gamePreferences(game);LinearLayout box=new LinearLayout(this);box.setOrientation(LinearLayout.VERTICAL);box.setPadding(dp(20),dp(8),dp(20),dp(8));
  box.addView(dialogLabel("이 게임에만 적용됩니다. 변경하면 게임이 재시작되므로 먼저 저장하세요. 전체 화면 영역 옵션은 LGT 게임에서 지원합니다.",12));
  EditText width=new EditText(this),height=new EditText(this);width.setInputType(2);height.setInputType(2);width.setHint("가로 (64~1024)");height.setHint("세로 (64~1024)");width.setText(""+p.getInt("width",240));height.setText(""+p.getInt("height",320));box.addView(dialogLabel("가로",12));box.addView(width);box.addView(dialogLabel("세로",12));box.addView(height);
  CheckBox full=new CheckBox(this);full.setText("전체 화면 영역 표시 (상태 표시줄 영역 포함)");full.setChecked(p.getBoolean("full",false));box.addView(full);
  AlertDialog dialog=new AlertDialog.Builder(this).setTitle("화면 크기").setView(box).setPositiveButton("적용 후 재시작",null).setNeutralButton("자동 설정 후 재시작",(d,w)->{p.edit().putBoolean("customDisplay",false).apply();restartWithSettings(game);}).setNegativeButton("취소",null).create();
  dialog.setOnShowListener(v->dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener(b->{try{int x=Integer.parseInt(width.getText().toString()),y=Integer.parseInt(height.getText().toString());if(x<64||x>1024||y<64||y>1024)throw new NumberFormatException();p.edit().putBoolean("customDisplay",true).putInt("width",x).putInt("height",y).putBoolean("full",full.isChecked()).apply();dialog.dismiss();restartWithSettings(game);}catch(NumberFormatException e){Toast.makeText(this,"가로·세로에 64~1024 사이의 값을 입력하세요",Toast.LENGTH_LONG).show();}}));dialog.show();
 }
 private void restartWithSettings(File game){releaseKeys();audio.stopAll();starting=true;io.execute(()->{NativeBridge.stop();runOnUiThread(()->{starting=false;launchPrepared(game);});});}
 private void showGame(){
  setRequestedOrientation(BuildConfig.THOR?ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE:ActivityInfo.SCREEN_ORIENTATION_FULL_SENSOR);setupRoot();
  keypadButtons.clear();languageKey=null;
  boolean landscape=getResources().getConfiguration().orientation==Configuration.ORIENTATION_LANDSCAPE;
  LinearLayout header=new LinearLayout(this);header.setGravity(Gravity.CENTER_VERTICAL);
  Button back=button("‹  게임 목록",()->leaveGame());back.setTextSize(12);header.addView(back,new LinearLayout.LayoutParams(dp(92),dp(32)));
  status=label(activeGame.getName(),12);status.setSingleLine(true);status.setEllipsize(android.text.TextUtils.TruncateAt.END);status.setTextColor(0xff4d535b);
  header.addView(status,new LinearLayout.LayoutParams(0,-2,1));
  if(!BuildConfig.THOR){
   Button save=button("저장",()->checkpoint("save"));save.setContentDescription("빠른 저장");save.setTextSize(11);save.setPadding(0,0,0,0);header.addView(save,new LinearLayout.LayoutParams(dp(48),dp(32)));
   Button load=button("불러오기",()->checkpoint("load"));load.setContentDescription("빠른 불러오기 · 길게 누르면 불러오기 전으로 복구");load.setOnLongClickListener(v->{checkpoint("recover");return true;});load.setTextSize(11);load.setPadding(0,0,0,0);header.addView(load,new LinearLayout.LayoutParams(dp(48),dp(32)));
  }
  Button settings=button("설정",this::showGameSettings);settings.setTextSize(11);header.addView(settings,new LinearLayout.LayoutParams(dp(78),dp(32)));
  root.addView(header,new LinearLayout.LayoutParams(-1,dp(36)));
  gameView=new GameView();
  if(BuildConfig.THOR){root.addView(gameView,new LinearLayout.LayoutParams(-1,0,1));lastReport=0;return;}
  String[][] directions={{"","▲",""},{"◀","OK","▶"},{"","▼",""},{"","CALL","한글"}};
  String[][] directionCodes={{"","UP",""},{"LEFT","OK","RIGHT"},{"","DOWN",""},{"","CALL","LANG"}};
  String[][] keys={{"L","CLR","R"},{"1","2","3"},{"4","5","6"},{"7","8","9"},{"*","0","#"}};
  LinearLayout dpad=keyGrid(directions,directionCodes),keypad=keyGrid(keys,keys);
  if(landscape){
   LinearLayout body=new LinearLayout(this);body.setGravity(Gravity.CENTER_VERTICAL);body.setMotionEventSplittingEnabled(true);
   root.addView(body,new LinearLayout.LayoutParams(-1,0,1));
   int screenWidth=getResources().getConfiguration().screenWidthDp;
   int side=dp(Math.max(132,Math.min(210,(int)(screenWidth*0.22f))));
   body.addView(dpad,new LinearLayout.LayoutParams(side,-1));
   LinearLayout.LayoutParams display=new LinearLayout.LayoutParams(0,-1,1);display.setMargins(dp(8),dp(4),dp(8),dp(4));body.addView(gameView,display);
   body.addView(keypad,new LinearLayout.LayoutParams(side,-1));
  }else{
   LinearLayout.LayoutParams display=new LinearLayout.LayoutParams(-1,0,1);display.setMargins(dp(4),dp(8),dp(4),dp(8));root.addView(gameView,display);
   LinearLayout controls=new LinearLayout(this);controls.setGravity(Gravity.CENTER);controls.setMotionEventSplittingEnabled(true);
   int controlsHeight=Math.min(290,Math.max(220,(int)(getResources().getConfiguration().screenHeightDp*0.34f)));
   root.addView(controls,new LinearLayout.LayoutParams(-1,dp(controlsHeight)));
   controls.addView(dpad,new LinearLayout.LayoutParams(0,-1,0.46f));
   View gap=new View(this);controls.addView(gap,new LinearLayout.LayoutParams(dp(12),1));
   controls.addView(keypad,new LinearLayout.LayoutParams(0,-1,0.54f));
  }
  updateKeypadLabels();lastReport=0;
 }
 private void updateKeypadLabels(){
  String[] letters={"ㅇ ㅁ","ㅣ","ㆍ","ㅡ","ㄱ ㅋ","ㄴ ㄹ","ㄷ ㅌ","ㅂ ㅍ","ㅅ ㅎ","ㅈ ㅊ"};
  for(Map.Entry<String,Button> entry:keypadButtons.entrySet()){
   String key=entry.getKey(),label=key;
   if(koreanKeypad){if(key.charAt(0)>='0'&&key.charAt(0)<='9')label=key+"\n"+letters[key.charAt(0)-'0'];else if(key.equals("*"))label="* 공백";else if(key.equals("#"))label="# 확정";}
   entry.getValue().setText(label);entry.getValue().setTextSize(koreanKeypad?15:20);entry.getValue().setContentDescription(label);
  }
  if(languageKey!=null){languageKey.setText(koreanKeypad?"ABC":"한글");languageKey.setContentDescription(koreanKeypad?"영문 키패드로 전환":"한글 키패드로 전환");}
 }
 private LinearLayout keyGrid(String[][] labels,String[][] codes){
  LinearLayout col=column();col.setMotionEventSplittingEnabled(true);
  for(int y=0;y<labels.length;y++){
   LinearLayout row=new LinearLayout(this);row.setMotionEventSplittingEnabled(true);col.addView(row,new LinearLayout.LayoutParams(-1,0,1));
   for(int x=0;x<labels[y].length;x++){
    String text=labels[y][x],code=codes[y][x];LinearLayout.LayoutParams cell=new LinearLayout.LayoutParams(0,-1,1);cell.setMargins(dp(3),dp(3),dp(3),dp(3));
    if(code.isEmpty()){row.addView(new View(this),cell);continue;}
    if(code.equals("LANG")){
     languageKey=button(text,()->{if(starting||checkpointBusy)return;releaseKeys();koreanKeypad=!koreanKeypad;NativeBridge.textInputMode(koreanKeypad);updateKeypadLabels();
      Toast.makeText(this,koreanKeypad?"천지인 입력 · # 글자 완성 · * 띄어쓰기 · 지움 삭제":"영문 / 게임 키패드",Toast.LENGTH_SHORT).show();});
     languageKey.setTextSize(13);languageKey.setPadding(0,0,0,0);row.addView(languageKey,cell);continue;
    }
    if(code.equals("SAVE")||code.equals("LOAD")){
     Button checkpoint=button(text,()->checkpoint(code.equals("SAVE")?"save":"load"));
     checkpoint.setTextSize(11);checkpoint.setPadding(0,0,0,0);checkpoint.setContentDescription(code.equals("SAVE")?"빠른 저장":"빠른 불러오기 · 길게 누르면 불러오기 전으로 복구");
     if(code.equals("LOAD"))checkpoint.setOnLongClickListener(v->{checkpoint("recover");return true;});
     row.addView(checkpoint,cell);continue;
    }
    Button b=button(keyLabel(text),()->{});boolean primary=code.equals("OK");boolean utility=code.equals("L")||code.equals("R")||code.equals("CLR")||code.equals("CALL");
    b.setTextSize(utility?12:20);b.setTypeface(Typeface.DEFAULT,Typeface.BOLD);b.setTextColor(primary?0xff20252a:utility?0xff454a50:0xff282d33);b.setPadding(0,0,0,0);b.setContentDescription(keyLabel(code));
    b.setBackground(metalKey(primary));row.addView(b,cell);if(code.length()==1&&!code.equals("L")&&!code.equals("R"))keypadButtons.put(code,b);
    final boolean[] touchClick={false};
    b.setOnTouchListener((v,event)->{long entry=System.nanoTime();long eventNs=Build.VERSION.SDK_INT>=34?event.getEventTimeNanos():event.getEventTime()*1000000L;switch(event.getActionMasked()){case MotionEvent.ACTION_DOWN:key(code,true,eventNs,entry);b.setPressed(true);return true;case MotionEvent.ACTION_UP:key(code,false,eventNs,entry);b.setPressed(false);touchClick[0]=true;b.performClick();touchClick[0]=false;return true;case MotionEvent.ACTION_CANCEL:key(code,false,eventNs,entry);b.setPressed(false);return true;}return true;});
    b.setOnClickListener(v->{if(touchClick[0])return;key(code,true);b.postDelayed(()->key(code,false),150);});
   }
  }
  return col;
 }
 private boolean checkpointBusy=false;
 private org.json.JSONObject checkpointRequest(String method,String path,org.json.JSONObject body)throws Exception {
  org.json.JSONObject config=new org.json.JSONObject(new String(java.nio.file.Files.readAllBytes(new File(getFilesDir(),"mac-checkpoints.json").toPath()),java.nio.charset.StandardCharsets.UTF_8));
  java.net.HttpURLConnection connection=(java.net.HttpURLConnection)new java.net.URL("http://127.0.0.1:"+config.getInt("port")+"/"+path).openConnection();
  connection.setConnectTimeout(3000);connection.setReadTimeout(5000);connection.setRequestMethod(method);
  connection.setRequestProperty("Authorization","Bearer "+config.getString("token"));
  try {
   if(body!=null){connection.setDoOutput(true);connection.setRequestProperty("Content-Type","application/json");try(OutputStream out=connection.getOutputStream()){out.write(body.toString().getBytes(java.nio.charset.StandardCharsets.UTF_8));}}
   try(InputStream in=connection.getInputStream()){return new org.json.JSONObject(new String(readBytes(in),java.nio.charset.StandardCharsets.UTF_8));}
  }finally{connection.disconnect();}
 }
 private static String userNotice(String text){
  switch(text){
   case "Quick Save created on this device (experimental).":return "현재 상태를 빠른 저장했습니다. (실험 기능)";
   case "Quick Load complete. No undo was created for the stopped/expired session.":return "불러왔습니다. 중단되거나 만료된 세션이므로 불러오기 전으로 복구할 수 없습니다.";
   case "Quick Load complete. Active audio restarts from its beginning.":return "불러왔습니다. 재생 중이던 소리는 처음부터 다시 재생됩니다.";
   case "Cannot save a stopped game; Quick Load remains available":return "중단된 게임은 저장할 수 없습니다. 빠른 불러오기는 사용할 수 있습니다.";
   case "Game stopped; Quick Load available":return "게임 중단 · 빠른 불러오기 가능";
   case "Stopped":return "중단됨";
   default:return text;
  }
 }
 private void checkpoint(String action){
  if(checkpointBusy||starting||activeGame==null)return;
  if((action.equals("save")||action.equals("load"))&&!new File(getFilesDir(),"mac-checkpoints.json").isFile()){
   releaseKeys();
   File directory=new File(getFilesDir(),"checkpoints/"+activeGame.getParentFile().getName());
   String[] labels=new String[3];
   for(int i=0;i<3;i++){
    String name=i==0?"quick":"quick-"+(i+1);File saved=new File(directory,name);
    if(!saved.isDirectory())saved=new File(directory,name+"-old");
    labels[i]="슬롯 "+(i+1)+(saved.isDirectory()?" · 저장됨":" · 비어 있음");
   }
   new AlertDialog.Builder(this).setTitle(action.equals("save")?"빠른 저장 · 슬롯 선택":"빠른 불러오기 · 슬롯 선택")
    .setItems(labels,(dialog,index)->checkpointSlot(action,index+1)).setNegativeButton("취소",null).show();
   return;
  }
  checkpointSlot(action,1);
 }
 private void checkpointSlot(String action,int slot){
  if(checkpointBusy||starting||activeGame==null)return;
  releaseKeys();checkpointBusy=true;
  status.setText(action.equals("save")?"현재 상태를 저장하는 중…":"저장한 상태를 불러오는 중…");
  Toast.makeText(this,action.equals("save")?"현재 상태를 저장하는 중…":"현재 상태를 백업한 뒤 불러오는 중…",Toast.LENGTH_SHORT).show();
  if(!new File(getFilesDir(),"mac-checkpoints.json").isFile()){
   if(!action.equals("save"))audio.stopAll();
   io.execute(()->{
    String notice;
    try{notice=NativeBridge.checkpoint((action.equals("save")||action.equals("load"))?action+":"+slot:action);}catch(Exception error){notice="빠른 저장·불러오기를 사용할 수 없습니다: "+error.getMessage();}
    String result=userNotice(notice);runOnUiThread(()->{checkpointBusy=false;koreanKeypad=NativeBridge.isKoreanInput();updateKeypadLabels();Toast.makeText(this,result,Toast.LENGTH_LONG).show();});
   });
   return;
  }
  String id=UUID.randomUUID().toString();
  String checkpointGame=activeGame.getParentFile().getName();
  io.execute(()->{
   String message;
   try{
    org.json.JSONObject request=new org.json.JSONObject().put("id",id).put("action",action).put("game",checkpointGame);
    org.json.JSONObject result=checkpointRequest("POST","",request);
    long deadline=SystemClock.elapsedRealtime()+180000;
    while(result.optBoolean("pending")){
     if(SystemClock.elapsedRealtime()>deadline)throw new IOException("처리 시간이 초과되었습니다. Mac 도우미 앱을 확인한 뒤 다시 시도하세요.");
     Thread.sleep(750);result=checkpointRequest("GET",id,null);
    }
    message=result.optString("error",result.optString("message","완료했습니다."));
   }catch(Exception error){message="빠른 저장·불러오기를 사용할 수 없습니다: "+error.getMessage();}
   String notice=userNotice(message);runOnUiThread(()->{checkpointBusy=false;Toast.makeText(this,notice,Toast.LENGTH_LONG).show();});
  });
 }
 private void key(String code,boolean down){key(code,down,0,System.nanoTime());}
 private String guestKey(String code){
  if(numericDirections)switch(code){case "UP":return "2";case "LEFT":return "4";case "RIGHT":return "6";case "DOWN":return "8";}
  return code;
 }
 private void key(String code,boolean down,long eventNs,long listenerNs){
  if(activeGame==null||starting||checkpointBusy)return;
  if(down?held.add(code):held.remove(code)){
   String mapped=guestKey(code);
   for(String other:held)if(!other.equals(code)&&guestKey(other).equals(mapped))return;
   long id=++nextInput;Long press=down?id:pressIds.remove(mapped);if(down)pressIds.put(mapped,id);
   if(tracing)NativeBridge.tracePoint(26,id,press==null?0:press);
   NativeBridge.key(mapped,down,id,eventNs,listenerNs);
  }
 }
 private void releaseKeys(){controller.release();for(String k:new ArrayList<>(held))key(k,false);}
 private void leaveGame(){if(starting||checkpointBusy)return;releaseKeys();audio.stopAll();starting=true;io.execute(()->{NativeBridge.stop();runOnUiThread(()->{starting=false;showLibrary();});});}
 // API 33+ uses the registered platform callback; keep this for Android 8–12.
 @android.annotation.SuppressLint("GestureBackNavigation")
 @Override public void onBackPressed(){handleBack();}
 private void handleBack(){if(activeGame!=null)leaveGame();else finish();}
 @Override protected void onResume(){super.onResume();foreground=true;NativeBridge.pause(false);audio.pause(false);Choreographer.getInstance().postFrameCallback(this);}
 @Override protected void onPause(){foreground=false;releaseKeys();NativeBridge.pause(true);audio.pause(true);Choreographer.getInstance().removeFrameCallback(this);super.onPause();}
 @Override protected void onDestroy(){if(BuildConfig.THOR)((android.hardware.input.InputManager)getSystemService(INPUT_SERVICE)).unregisterInputDeviceListener(controllerListener);io.execute(NativeBridge::stop);io.shutdown();audio.close();super.onDestroy();}
 @Override public void onConfigurationChanged(Configuration c){super.onConfigurationChanged(c);releaseKeys();if(activeGame!=null){Bitmap old=gameView==null?null:gameView.bitmap;showGame();gameView.bitmap=old;}else showLibrary();}
 @Override public void doFrame(long nanos){if(!foreground)return;if(tracing)NativeBridge.tracePoint(20,nanos,System.nanoTime());if(activeGame!=null&&!starting&&!checkpointBusy){long shape=pickupFrame(nanos);if(shape!=0){int w=(int)(shape>>>32),h=(int)shape;if(gameView.bitmap==null||gameView.bitmap.getWidth()!=w||gameView.bitmap.getHeight()!=h)gameView.bitmap=Bitmap.createBitmap(w,h,Bitmap.Config.ARGB_8888);if(tracing)NativeBridge.tracePoint(31,frameMetadata[0],0);gameView.bitmap.setPixels(pixels,0,w,0,0,w,h);gameView.paintId=frameMetadata[0];if(tracing)NativeBridge.tracePoint(21,gameView.paintId,0);gameView.invalidate();}
   for(int i=0;i<8;i++){byte[] packet=NativeBridge.audio();if(packet==null)break;audio.submit(packet);}
   long now=SystemClock.elapsedRealtime();if(now-lastReport>=1000){String current=NativeBridge.status();if(current.startsWith("Error:")&&!rescueShown){rescueShown=true;showRescue(current);}status.setText(current.equals("Running")?activeGame.getName():userNotice(current));long paints=NativeBridge.paints();if(lastReport!=0)Log.i("WIE-Native","frames="+(paints-lastPaints)+" elapsedMs="+(now-lastReport)+" state="+current);lastReport=now;lastPaints=paints;}}
  Choreographer.getInstance().postFrameCallback(this);
 }
 private void createManualRescue(){
  if(activeGame==null||starting||checkpointBusy||rescueBusy)return;
  prepareRescue(null);
 }
 private void exportRescue(File report){if(!rescueBusy)prepareRescue(report);}
 private void prepareRescue(File existing){
  rescueBusy=true;Toast.makeText(this,"오류 보고서를 준비하는 중…",Toast.LENGTH_SHORT).show();
  io.execute(()->{File prepared=null;try{
   File report=existing==null?new File(NativeBridge.manualRescue()):existing;
   prepared=File.createTempFile("gomul-rescue-",".zip",getCacheDir());
   try(OutputStream out=new FileOutputStream(prepared)){RescueReports.export(report,out);}
   File ready=prepared;runOnUiThread(()->{
    if(isFinishing()||isDestroyed()){ready.delete();rescueBusy=false;return;}
    rescueExport=ready;
    Intent intent=new Intent(Intent.ACTION_CREATE_DOCUMENT);intent.setType("application/zip");intent.addCategory(Intent.CATEGORY_OPENABLE);intent.putExtra(Intent.EXTRA_TITLE,"GOmul-rescue-"+System.currentTimeMillis()+".zip");
    try{startActivityForResult(intent,4);}catch(Exception e){ready.delete();rescueExport=null;rescueBusy=false;rescueError(e);}
   });
  }catch(Exception e){if(prepared!=null)prepared.delete();runOnUiThread(()->{rescueBusy=false;rescueError(e);});}});
 }
 private void rescueError(Exception e){Log.e("WIE-Native","Rescue export failed",e);new AlertDialog.Builder(this).setTitle("오류 보고서를 만들 수 없습니다").setMessage(e.getMessage()).setPositiveButton("확인",null).show();}
 private void showRescue(String failure){
  File report=activeGame==null?null:RescueReports.latest(getFilesDir(),activeGame.getParentFile().getName());
  if(report!=null&&report.lastModified()<rescueSessionStart)report=null;
  AlertDialog.Builder dialog=new AlertDialog.Builder(this).setTitle("게임 중단 · 오류 보고서").setMessage(failure+"\n\n"+(report==null?"이번 오류의 보고서가 저장되지 않았습니다.":"오류 보고서가 기기에 저장되었습니다. 화면, 재현 기록 및 개인 저장 데이터가 포함되므로 개발자에게 비공개로 전달하세요."));
  if(report!=null){File ready=report;dialog.setPositiveButton("오류 보고서 내보내기",(d,w)->exportRescue(ready));}
  dialog.setNegativeButton("게임 목록으로",(d,w)->leaveGame()).setNeutralButton("화면 유지",null).show();
 }
 private void error(Exception e){Log.e("WIE-Native","Operation failed",e);new AlertDialog.Builder(this).setTitle("작업을 완료할 수 없습니다").setMessage(e.getMessage()).setPositiveButton("확인",null).show();}
 private class GameView extends View {Bitmap bitmap;long paintId;final Paint paint=new Paint();final RectF destination=new RectF();GameView(){super(MainActivity.this);setBackgroundColor(Color.BLACK);paint.setFilterBitmap(false);}protected void onDraw(Canvas canvas){super.onDraw(canvas);if(bitmap==null)return;float scale=Math.min(getWidth()/(float)bitmap.getWidth(),getHeight()/(float)bitmap.getHeight());float w=bitmap.getWidth()*scale,h=bitmap.getHeight()*scale;destination.set((getWidth()-w)/2,(getHeight()-h)/2,(getWidth()+w)/2,(getHeight()+h)/2);long drawId=++nextDraw,drawPaint=paintId;boolean capture=tracing;
 if(capture){
  Trace.beginSection("GOmulDraw:"+drawId+":"+drawPaint);
  try{NativeBridge.tracePoint(22,drawId,drawPaint);canvas.drawBitmap(bitmap,null,destination,paint);NativeBridge.tracePoint(32,drawId,drawPaint);}finally{Trace.endSection();}
  if(Build.VERSION.SDK_INT>=29&&isHardwareAccelerated())getViewTreeObserver().registerFrameCommitCallback(()->{if(tracing)NativeBridge.tracePoint(23,drawId,drawPaint);});
 }else canvas.drawBitmap(bitmap,null,destination,paint);}}
}
