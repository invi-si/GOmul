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
 private File activeGame;private boolean foreground=false,starting=false;private final int[] pixels=new int[1024*1024];
 private long lastReport=0,lastPaints=0;
 @Override public void onCreate(Bundle state){super.onCreate(state);getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);audio=new AudioOutput(getCacheDir());if(Build.VERSION.SDK_INT>=33)getOnBackInvokedDispatcher().registerOnBackInvokedCallback(android.window.OnBackInvokedDispatcher.PRIORITY_DEFAULT,this::handleBack);showLibrary();}
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
 private void setupRoot(){
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
 @Override public void onWindowFocusChanged(boolean focused){super.onWindowFocusChanged(focused);if(focused)updateSystemBars();}
 private Button button(String title,Runnable action){
  Button b=new Button(this);b.setText(title);b.setAllCaps(false);b.setTextColor(ACCENT);b.setTextSize(15);b.setGravity(Gravity.CENTER);b.setMinHeight(0);b.setMinimumHeight(0);b.setMinWidth(0);b.setMinimumWidth(0);b.setPadding(dp(14),dp(8),dp(14),dp(8));
  b.setBackground(metalKey(false));b.setShadowLayer(dp(0.6f),0,dp(0.7f),0xfffafbfc);b.setStateListAnimator(null);b.setOnClickListener(v->action.run());return b;
 }
 private TextView label(String text,int size){TextView t=new TextView(this);t.setText(text);t.setTextColor(0xff25292e);t.setTextSize(size);t.setPadding(dp(5),dp(5),dp(5),dp(5));return t;}
 private void addSpaced(LinearLayout parent,View child,int height){LinearLayout.LayoutParams params=new LinearLayout.LayoutParams(-1,dp(height));params.setMargins(0,dp(6),0,dp(6));parent.addView(child,params);}
 private File games(){File f=new File(getFilesDir(),"games");f.mkdirs();return f;}
 private void showLibrary(){
  new File(getFilesDir(),"active-game.txt").delete();
  activeGame=null;gameView=null;setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_UNSPECIFIED);setupRoot();
  TextView eyebrow=label("G O m u l   /   고 물",12);eyebrow.setTextColor(ACCENT);root.addView(eyebrow);
  TextView title=label("Your games",32);title.setTypeface(Typeface.DEFAULT,Typeface.BOLD);root.addView(title);
  TextView subtitle=label("A little screen. A whole world.",15);subtitle.setTextColor(0xff4d535b);root.addView(subtitle);
  addSpaced(root,button("＋  Import a game",()->{if(starting)return;Intent i=new Intent(Intent.ACTION_OPEN_DOCUMENT);i.setType("*/*");i.addCategory(Intent.CATEGORY_OPENABLE);startActivityForResult(i,1);}),54);
  TextView hint=label("Import .jar / .zip · Hold a game to import saved data",12);hint.setTextColor(0xff4d535b);root.addView(hint);
  ScrollView scroll=new ScrollView(this);scroll.setClipToPadding(false);LinearLayout list=column();scroll.addView(list);root.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));
  File[] folders=games().listFiles();if(folders!=null){Arrays.sort(folders);for(File folder:folders){File[] files=folder.listFiles((d,name)->name.toLowerCase(Locale.ROOT).endsWith(".jar")||name.toLowerCase(Locale.ROOT).endsWith(".zip"));if(files!=null)for(File game:files){Button item=button("▶   "+game.getName(),()->launch(game));item.setOnLongClickListener(v->{chooseData(game);return true;});item.setGravity(Gravity.CENTER_VERTICAL|Gravity.START);item.setTextColor(0xff25292e);addSpaced(list,item,72);}}}
  if(list.getChildCount()==0){TextView empty=label("Your library is ready.\nImport your first game to start playing.",17);empty.setPadding(dp(8),dp(36),dp(8),dp(24));empty.setTextColor(0xff4d535b);list.addView(empty);}
  addSpaced(root,button("Open-source licences",()->{try(InputStream in=getAssets().open("LICENSES.txt")){new AlertDialog.Builder(this).setTitle("Licences").setMessage(new String(readBytes(in),java.nio.charset.StandardCharsets.UTF_8)).setPositiveButton("Close",null).show();}catch(IOException e){error(e);}}),48);
 }
 private File saveFolder(File game){return new File(getFilesDir(),"saves/"+game.getParentFile().getName());}
 private void chooseData(File game){
  if(starting)return;dataImportGame=game;
  new AlertDialog.Builder(this).setTitle("Import saved data").setItems(new String[]{"Choose data ZIP","Choose data folder"},(dialog,which)->{
   Intent intent=new Intent(which==0?Intent.ACTION_OPEN_DOCUMENT:Intent.ACTION_OPEN_DOCUMENT_TREE);
   if(which==0){intent.setType("*/*");intent.addCategory(Intent.CATEGORY_OPENABLE);}startActivityForResult(intent,which==0?2:3);
  }).show();
 }
 private GameDataImport.Plan readDataFolder(File game,Uri tree)throws Exception {
  String pid=GameDataImport.pid(game);if(pid==null)throw new IOException("Saved-data import currently requires a complete LGT ZIP with app_info.");
  String document=android.provider.DocumentsContract.getTreeDocumentId(tree);
  Uri children=android.provider.DocumentsContract.buildChildDocumentsUriUsingTree(tree,document);
  Map<String,byte[]> files=new LinkedHashMap<>();String phone=null;int total=0;
  String[] columns={android.provider.DocumentsContract.Document.COLUMN_DOCUMENT_ID,android.provider.DocumentsContract.Document.COLUMN_DISPLAY_NAME};
  try(Cursor cursor=getContentResolver().query(children,columns,null,null,null)){
   if(cursor==null)throw new IOException("Cannot read the selected folder.");
   while(cursor.moveToNext()){
    String name=cursor.getString(1);if(!GameDataImport.dataName(name)&&!name.equals("gomul.properties"))continue;
    Uri uri=android.provider.DocumentsContract.buildDocumentUriUsingTree(tree,cursor.getString(0));byte[] bytes;
    try(InputStream in=getContentResolver().openInputStream(uri)){if(in==null)throw new IOException("Cannot read "+name);bytes=GameDataImport.read(in,GameDataImport.MAX_DATA);}
    total+=bytes.length;if(total>GameDataImport.MAX_DATA)throw new IOException("Data bundle is too large.");
    if(name.equals("gomul.properties")){Properties props=new Properties();props.load(new ByteArrayInputStream(bytes));phone=props.getProperty("phoneNumber");}
    else {if(files.put(name,bytes)!=null)throw new IOException("Duplicate data filename.");}
   }
  }
  if(files.isEmpty())throw new IOException("Choose the folder containing savedata and its companion files.");
  return new GameDataImport.Plan(pid,files,phone);
 }
 private void finishDataImport(File game,GameDataImport.Plan plan,boolean replace){
  if(plan.phone!=null){installData(game,plan,plan.phone,replace);return;}
  EditText number=new EditText(this);number.setInputType(android.text.InputType.TYPE_CLASS_PHONE);number.setHint("11-digit emulated phone number");
  new AlertDialog.Builder(this).setTitle("Saved data detected").setMessage("Enter the phone number specified by the supplier of these files. This only sets the game's emulated identity.").setView(number)
   .setPositiveButton("Import and play",(dialog,which)->installData(game,plan,number.getText().toString(),replace)).setNegativeButton("Cancel",null).show();
 }
 private void installData(File game,GameDataImport.Plan plan,String phone,boolean replace){
  starting=true;io.execute(()->{try{GameDataImport.install(saveFolder(game),plan,phone,replace);runOnUiThread(()->{starting=false;Toast.makeText(this,"Saved data imported",Toast.LENGTH_SHORT).show();launch(game);});}
   catch(Exception error){runOnUiThread(()->{starting=false;error(error);});}});
 }
 private void offerDataImport(File game,GameDataImport.Plan plan,boolean separate){
  if(plan==null){if(separate)error(new IOException("No companion saved-data files were found."));else launch(game);return;}
  if(saveFolder(game).exists()){
   if(!separate){launch(game);return;}
   new AlertDialog.Builder(this).setTitle("Replace this game's saved data?").setMessage("The current save will be backed up before importing these files. Other games are unaffected.")
    .setPositiveButton("Back up and import",(dialog,which)->finishDataImport(game,plan,true)).setNegativeButton("Cancel",null).show();return;
  }
  finishDataImport(game,plan,false);
 }
 @Override protected void onActivityResult(int request,int result,Intent data){
  super.onActivityResult(request,result,data);if(result!=RESULT_OK||data==null||request<1||request>3)return;Uri uri=data.getData();if(uri==null)return;
  File selected=dataImportGame;starting=true;Toast.makeText(this,"Reading imported files…",Toast.LENGTH_SHORT).show();
  io.execute(()->{try{
   File game;GameDataImport.Plan plan;
   if(request==1){game=importGame(uri);plan=GameDataImport.inspect(game,game);}
   else {if(selected==null)throw new IOException("Select a game again before importing data.");game=selected;
    if(request==3)plan=readDataFolder(game,uri);
    else {File temporary=File.createTempFile("data-import-",".zip",getCacheDir());try{
     try(InputStream in=getContentResolver().openInputStream(uri)){if(in==null)throw new IOException("Cannot open data ZIP.");java.nio.file.Files.write(temporary.toPath(),GameDataImport.read(in,GameDataImport.MAX_DATA));}
     plan=GameDataImport.inspect(game,temporary);
    }finally{temporary.delete();}}
   }
   File ready=game;GameDataImport.Plan pending=plan;runOnUiThread(()->{starting=false;showLibrary();offerDataImport(ready,pending,request!=1);});
  }catch(Exception error){runOnUiThread(()->{starting=false;error(error);});}});
 }
 private File importGame(Uri uri)throws Exception {
  String name="game.jar";try(Cursor c=getContentResolver().query(uri,new String[]{OpenableColumns.DISPLAY_NAME},null,null,null)){if(c!=null&&c.moveToFirst())name=c.getString(0);}
  name=new File(name).getName().replaceAll("[^\\p{L}\\p{N}._ -]","_");String lower=name.toLowerCase(Locale.ROOT);if(!lower.endsWith(".jar")&&!lower.endsWith(".zip"))throw new IOException("Choose a .jar or .zip game archive.");
  File temp=File.createTempFile("import-",".tmp",getCacheDir());MessageDigest hash=MessageDigest.getInstance("SHA-256");
  try {try(InputStream in=getContentResolver().openInputStream(uri);OutputStream out=new FileOutputStream(temp)){if(in==null)throw new IOException("Cannot open game file");byte[] b=new byte[65536];long total=0;int n;while((n=in.read(b))!=-1){total+=n;if(total>128L*1024*1024)throw new IOException("This prototype accepts archives up to 128 MB.");hash.update(b,0,n);out.write(b,0,n);}}
   StringBuilder id=new StringBuilder();for(byte b:hash.digest())id.append(String.format(Locale.ROOT,"%02x",b));File folder=new File(games(),id.toString());if(!folder.exists()&&!folder.mkdirs())throw new IOException("Cannot create game directory");File destination=new File(folder,name);if(destination.exists())return destination;
   if(lower.endsWith(".zip"))GameDataImport.normalize(temp);java.nio.file.Files.move(temp.toPath(),destination.toPath(),java.nio.file.StandardCopyOption.ATOMIC_MOVE);return destination;
  }finally{temp.delete();}
 }
 private void launch(File file){if(starting)return;starting=true;activeGame=file;showGame();status.setText("Loading…");audio.stopAll();io.execute(()->{try{
   java.nio.file.Files.write(new File(getFilesDir(),"active-game.txt").toPath(),file.getParentFile().getName().getBytes(java.nio.charset.StandardCharsets.UTF_8));
   if(new File(getFilesDir(),"mac-checkpoints.json").isFile()){
    String requestId=UUID.randomUUID().toString();
    org.json.JSONObject result=checkpointRequest("POST","",new org.json.JSONObject().put("id",requestId).put("action","boot").put("game",file.getParentFile().getName()));
    long deadline=SystemClock.elapsedRealtime()+180000;
    while(result.optBoolean("pending")){
     if(SystemClock.elapsedRealtime()>deadline)throw new IOException("Startup checkpoint timed out.");
     Thread.sleep(750);result=checkpointRequest("GET",requestId,null);
    }
    if(result.has("error"))throw new IOException(result.getString("error"));
    if(!result.optBoolean("startNormally")){runOnUiThread(()->starting=false);return;}
   }
   NativeBridge.start(file.getAbsolutePath(),new File(getFilesDir(),"saves/"+file.getParentFile().getName()).getAbsolutePath());NativeBridge.pause(!foreground);runOnUiThread(()->starting=false);}catch(Exception e){runOnUiThread(()->{starting=false;error(e);});}});}
 private void showGame(){
  setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_FULL_SENSOR);setupRoot();
  boolean landscape=getResources().getConfiguration().orientation==Configuration.ORIENTATION_LANDSCAPE;
  LinearLayout header=new LinearLayout(this);header.setGravity(Gravity.CENTER_VERTICAL);
  Button back=button("‹  Library",()->leaveGame());back.setTextSize(12);header.addView(back,new LinearLayout.LayoutParams(dp(92),dp(32)));
  status=label(activeGame.getName(),12);status.setSingleLine(true);status.setEllipsize(android.text.TextUtils.TruncateAt.END);status.setTextColor(0xff4d535b);
  header.addView(status,new LinearLayout.LayoutParams(0,-2,1));
  TextView tip=label(landscape?"":"Rotate for a wider view",11);tip.setTextColor(0xff4d535b);header.addView(tip);
  root.addView(header,new LinearLayout.LayoutParams(-1,dp(36)));
  gameView=new GameView();
  boolean checkpoints=new File(getFilesDir(),"mac-checkpoints.json").isFile();
  String[][] directions={{"","▲",""},{"◀","OK","▶"},{"","▼",""},{checkpoints?"Quick\nSave":"","CALL",checkpoints?"Quick\nLoad":""}};
  String[][] directionCodes={{"","UP",""},{"LEFT","OK","RIGHT"},{"","DOWN",""},{checkpoints?"SAVE":"","CALL",checkpoints?"LOAD":""}};
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
  lastReport=0;
 }
 private LinearLayout keyGrid(String[][] labels,String[][] codes){
  LinearLayout col=column();col.setMotionEventSplittingEnabled(true);
  for(int y=0;y<labels.length;y++){
   LinearLayout row=new LinearLayout(this);row.setMotionEventSplittingEnabled(true);col.addView(row,new LinearLayout.LayoutParams(-1,0,1));
   for(int x=0;x<labels[y].length;x++){
    String text=labels[y][x],code=codes[y][x];LinearLayout.LayoutParams cell=new LinearLayout.LayoutParams(0,-1,1);cell.setMargins(dp(3),dp(3),dp(3),dp(3));
    if(code.isEmpty()){row.addView(new View(this),cell);continue;}
    if(code.equals("SAVE")||code.equals("LOAD")){
     Button checkpoint=button(text,()->checkpoint(code.equals("SAVE")?"save":"load"));
     checkpoint.setTextSize(11);checkpoint.setPadding(0,0,0,0);checkpoint.setContentDescription(code.equals("SAVE")?"Quick Save":"Quick Load; hold to undo load");
     if(code.equals("LOAD"))checkpoint.setOnLongClickListener(v->{checkpoint("recover");return true;});
     row.addView(checkpoint,cell);continue;
    }
    Button b=button(text,()->{});boolean primary=code.equals("OK");boolean utility=code.equals("L")||code.equals("R")||code.equals("CLR")||code.equals("CALL");
    b.setTextSize(utility?12:20);b.setTypeface(Typeface.DEFAULT,Typeface.BOLD);b.setTextColor(primary?0xff20252a:utility?0xff454a50:0xff282d33);b.setPadding(0,0,0,0);b.setContentDescription(code);
    b.setBackground(metalKey(primary));row.addView(b,cell);
    final boolean[] touchClick={false};
    b.setOnTouchListener((v,event)->{switch(event.getActionMasked()){case MotionEvent.ACTION_DOWN:key(code,true);b.setPressed(true);return true;case MotionEvent.ACTION_UP:key(code,false);b.setPressed(false);touchClick[0]=true;b.performClick();touchClick[0]=false;return true;case MotionEvent.ACTION_CANCEL:key(code,false);b.setPressed(false);return true;}return true;});
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
 private void checkpoint(String action){
  if(checkpointBusy||starting)return;
  releaseKeys();checkpointBusy=true;
  Toast.makeText(this,action.equals("save")?"Saving checkpoint…":"Saving recovery, then loading…",Toast.LENGTH_SHORT).show();
  String id=UUID.randomUUID().toString();
  String checkpointGame=activeGame.getParentFile().getName();
  io.execute(()->{
   String message;
   try{
    org.json.JSONObject request=new org.json.JSONObject().put("id",id).put("action",action).put("game",checkpointGame);
    org.json.JSONObject result=checkpointRequest("POST","",request);
    long deadline=SystemClock.elapsedRealtime()+180000;
    while(result.optBoolean("pending")){
     if(SystemClock.elapsedRealtime()>deadline)throw new IOException("Checkpoint operation timed out. Check the Mac helper before retrying.");
     Thread.sleep(750);result=checkpointRequest("GET",id,null);
    }
    message=result.optString("error",result.optString("message","Checkpoint complete."));
   }catch(Exception error){message="Checkpoint unavailable: "+error.getMessage();}
   String notice=message;runOnUiThread(()->{checkpointBusy=false;Toast.makeText(this,notice,Toast.LENGTH_LONG).show();});
  });
 }
 private void key(String code,boolean down){if(activeGame==null||starting)return;if(down?held.add(code):held.remove(code))NativeBridge.key(code,down);}
 private void releaseKeys(){for(String k:new ArrayList<>(held))key(k,false);}
 private void leaveGame(){if(starting)return;releaseKeys();audio.stopAll();starting=true;io.execute(()->{NativeBridge.stop();runOnUiThread(()->{starting=false;showLibrary();});});}
 // API 33+ uses the registered platform callback; keep this for Android 8–12.
 @android.annotation.SuppressLint("GestureBackNavigation")
 @Override public void onBackPressed(){handleBack();}
 private void handleBack(){if(activeGame!=null)leaveGame();else finish();}
 @Override protected void onResume(){super.onResume();foreground=true;NativeBridge.pause(false);audio.pause(false);Choreographer.getInstance().postFrameCallback(this);}
 @Override protected void onPause(){foreground=false;releaseKeys();NativeBridge.pause(true);audio.pause(true);Choreographer.getInstance().removeFrameCallback(this);super.onPause();}
 @Override protected void onDestroy(){io.execute(NativeBridge::stop);io.shutdown();audio.close();super.onDestroy();}
 @Override public void onConfigurationChanged(Configuration c){super.onConfigurationChanged(c);releaseKeys();if(activeGame!=null){Bitmap old=gameView==null?null:gameView.bitmap;showGame();gameView.bitmap=old;}else showLibrary();}
 @Override public void doFrame(long nanos){if(!foreground)return;if(activeGame!=null&&!starting){long shape=NativeBridge.frame(pixels);if(shape!=0){int w=(int)(shape>>>32),h=(int)shape;if(gameView.bitmap==null||gameView.bitmap.getWidth()!=w||gameView.bitmap.getHeight()!=h)gameView.bitmap=Bitmap.createBitmap(w,h,Bitmap.Config.ARGB_8888);gameView.bitmap.setPixels(pixels,0,w,0,0,w,h);gameView.invalidate();}
   for(int i=0;i<8;i++){byte[] packet=NativeBridge.audio();if(packet==null)break;audio.submit(packet);}
   long now=SystemClock.elapsedRealtime();if(now-lastReport>=1000){String current=NativeBridge.status();status.setText(current.equals("Running")?activeGame.getName():current);long paints=NativeBridge.paints();if(lastReport!=0)Log.i("WIE-Native","frames="+(paints-lastPaints)+" elapsedMs="+(now-lastReport)+" state="+current);lastReport=now;lastPaints=paints;}}
  Choreographer.getInstance().postFrameCallback(this);
 }
 private void error(Exception e){Log.e("WIE-Native","Operation failed",e);new AlertDialog.Builder(this).setTitle("Unable to open game").setMessage(e.getMessage()).setPositiveButton("OK",null).show();}
 private class GameView extends View {Bitmap bitmap;final Paint paint=new Paint();final RectF destination=new RectF();GameView(){super(MainActivity.this);setBackgroundColor(Color.BLACK);paint.setFilterBitmap(false);}protected void onDraw(Canvas canvas){super.onDraw(canvas);if(bitmap==null)return;float scale=Math.min(getWidth()/(float)bitmap.getWidth(),getHeight()/(float)bitmap.getHeight());float w=bitmap.getWidth()*scale,h=bitmap.getHeight()*scale;destination.set((getWidth()-w)/2,(getHeight()-h)/2,(getWidth()+w)/2,(getHeight()+h)/2);canvas.drawBitmap(bitmap,null,destination,paint);}}
}
