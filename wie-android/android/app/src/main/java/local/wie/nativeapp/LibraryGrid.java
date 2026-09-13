package local.wie.nativeapp;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.Color;
import android.graphics.drawable.GradientDrawable;
import android.text.Editable;
import android.text.TextWatcher;
import android.util.LruCache;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.*;
import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.*;
import java.util.function.Consumer;
import org.json.JSONObject;

/** Local library only: artwork and display metadata live beside each imported archive. */
final class LibraryGrid extends LinearLayout {
 private final List<Entry> all=new ArrayList<>(),visible=new ArrayList<>();
 private final LruCache<String,Bitmap> covers=new LruCache<String,Bitmap>(8*1024*1024){
  @Override protected int sizeOf(String key,Bitmap value){return value.getAllocationByteCount();}
 };
 private final Consumer<File> launch,manage;
 private final TextView count;
 private final BaseAdapter adapter;
 private final EditText search;
 private final GridView grid;
 static final class Position {
  final String query;final android.os.Parcelable scroll;
  Position(String query,android.os.Parcelable scroll){this.query=query;this.scroll=scroll;}
 }
 Position rememberPosition(){return new Position(search.getText().toString(),grid.onSaveInstanceState());}
 void restorePosition(Position position){
  if(position==null)return;
  search.setText(position.query);
  grid.onRestoreInstanceState(position.scroll);
 }
 private int dp(int value){return Math.round(value*getResources().getDisplayMetrics().density);}
 private static final class Entry {
  final File game,cover;final String title,carrier;final List<Entry> versions=new ArrayList<>();
  Entry(File file){game=file;cover=new File(file.getParentFile(),"thumbnail.jpg");
   String name=file.getName().replaceFirst("(?i)\\.(jar|zip)$", ""),network="";
   try{JSONObject metadata=new JSONObject(new String(Files.readAllBytes(new File(file.getParentFile(),"library.json").toPath()),StandardCharsets.UTF_8));name=metadata.optString("title",name);network=metadata.optString("carrier","");}catch(Exception ignored){}
   title=name;carrier=network;versions.add(this);
  }
 }
 LibraryGrid(Context context,File directory,Consumer<File> open,Consumer<File> options){
  super(context);launch=open;manage=options;setOrientation(VERTICAL);
  File[] folders=directory.listFiles();if(folders!=null)for(File folder:folders){File[] games=folder.listFiles((d,n)->n.toLowerCase(Locale.ROOT).endsWith(".jar")||n.toLowerCase(Locale.ROOT).endsWith(".zip"));if(games!=null)for(File game:games)all.add(new Entry(game));}
  Map<String,Entry> grouped=new LinkedHashMap<>();for(Entry entry:all){String key=java.text.Normalizer.normalize(entry.title,java.text.Normalizer.Form.NFKC).toLowerCase(Locale.KOREAN).replaceAll("\\s+","");Entry previous=grouped.get(key);if(previous==null)grouped.put(key,entry);else previous.versions.add(entry);}all.clear();all.addAll(grouped.values());
  all.sort(Comparator.comparing((Entry e)->e.title,java.text.Collator.getInstance(Locale.KOREAN)).thenComparing(e->e.carrier));
  search=new EditText(context);search.setSingleLine(true);search.setTextColor(0xff25292e);search.setHintTextColor(0xff687582);search.setHint("게임 검색");search.setTextSize(15);addView(search,new LayoutParams(-1,dp(46)));
  count=new TextView(context);count.setTextColor(0xff566370);count.setTextSize(12);count.setPadding(dp(4),dp(6),0,dp(8));addView(count);
  grid=new GridView(context);grid.setNumColumns(GridView.AUTO_FIT);grid.setColumnWidth(dp(112));grid.setStretchMode(GridView.STRETCH_COLUMN_WIDTH);grid.setHorizontalSpacing(dp(10));grid.setVerticalSpacing(dp(12));grid.setClipToPadding(false);grid.setPadding(dp(2),dp(2),dp(2),dp(8));
  adapter=new BaseAdapter(){
   public int getCount(){return visible.size();}public Object getItem(int position){return visible.get(position);}public long getItemId(int position){return position;}
   public View getView(int position,View reusable,ViewGroup parent){
    LinearLayout tile;
    if(reusable instanceof LinearLayout)tile=(LinearLayout)reusable;else{
     tile=new LinearLayout(context);tile.setOrientation(VERTICAL);tile.setPadding(dp(4),dp(4),dp(4),dp(4));
     GradientDrawable bg=new GradientDrawable();bg.setColor(Color.WHITE);bg.setCornerRadius(dp(6));bg.setStroke(dp(1),0xffd2dce5);tile.setBackground(bg);
     ImageView image=new ImageView(context);image.setScaleType(ImageView.ScaleType.CENTER_CROP);tile.addView(image,new LayoutParams(-1,dp(140)));
     TextView title=new TextView(context);title.setTextSize(13);title.setTextColor(0xff172c3d);title.setGravity(Gravity.CENTER_VERTICAL);title.setMaxLines(2);title.setEllipsize(android.text.TextUtils.TruncateAt.END);tile.addView(title,new LayoutParams(-1,dp(40)));
     TextView carrier=new TextView(context);carrier.setTextSize(10);carrier.setTextColor(0xff087caa);tile.addView(carrier,new LayoutParams(-1,dp(16)));
    }
    Entry entry=visible.get(position);ImageView image=(ImageView)tile.getChildAt(0);Bitmap cover=cover(entry.cover);
    image.setImageBitmap(cover);image.setBackgroundColor(0xffdfeaf2);if(cover==null){image.setImageResource(android.R.drawable.ic_menu_gallery);image.setScaleType(ImageView.ScaleType.CENTER);}else image.setScaleType(ImageView.ScaleType.CENTER_CROP);
    ((TextView)tile.getChildAt(1)).setText(entry.title);((TextView)tile.getChildAt(2)).setText(carriers(entry));
    tile.setContentDescription(entry.title+(entry.carrier.isEmpty()?"":" · "+entry.carrier));return tile;
   }
  };
  grid.setAdapter(adapter);grid.setOnItemClickListener((p,v,position,id)->choose(visible.get(position),launch));grid.setOnItemLongClickListener((p,v,position,id)->{choose(visible.get(position),manage);return true;});
  addView(grid,new LayoutParams(-1,0,1));
  search.addTextChangedListener(new TextWatcher(){public void beforeTextChanged(CharSequence s,int start,int count,int after){}public void onTextChanged(CharSequence s,int start,int before,int count){filter(s.toString());}public void afterTextChanged(Editable e){}});filter("");
 }
 private String carriers(Entry entry){Set<String> names=new LinkedHashSet<>();for(Entry version:entry.versions)if(!version.carrier.isEmpty())names.add(version.carrier);return String.join(" · ",names);}
 private void choose(Entry entry,Consumer<File> action){if(entry.versions.size()==1){action.accept(entry.game);return;}String[] names=new String[entry.versions.size()];for(int i=0;i<names.length;i++){Entry version=entry.versions.get(i);names[i]=version.carrier.isEmpty()?version.game.getName():version.carrier+" 버전";}new android.app.AlertDialog.Builder(getContext()).setTitle(entry.title+" · 버전 선택").setItems(names,(dialog,which)->action.accept(entry.versions.get(which).game)).setNegativeButton("취소",null).show();}
 private void filter(String query){String q=query.trim().toLowerCase(Locale.ROOT);visible.clear();for(Entry entry:all)if((entry.title+" "+carriers(entry)).toLowerCase(Locale.ROOT).contains(q))visible.add(entry);count.setText(all.isEmpty()?"게임 파일을 추가해 시작하세요.":"전체 게임 · "+visible.size()+"개");adapter.notifyDataSetChanged();}
 private Bitmap cover(File file){String key=file.getAbsolutePath();Bitmap result=covers.get(key);if(result!=null||!file.isFile())return result;BitmapFactory.Options bounds=new BitmapFactory.Options();bounds.inJustDecodeBounds=true;BitmapFactory.decodeFile(key,bounds);BitmapFactory.Options options=new BitmapFactory.Options();options.inSampleSize=1;while(Math.max(bounds.outWidth,bounds.outHeight)/options.inSampleSize>512)options.inSampleSize*=2;result=BitmapFactory.decodeFile(key,options);if(result!=null)covers.put(key,result);return result;}
}
