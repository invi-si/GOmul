import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const appDirectory = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const javaDirectory = path.join(appDirectory, "gen/android/app/src/main/java");
const activities = readdirSync(javaDirectory, { recursive: true }).filter(name => path.basename(name) === "MainActivity.kt");
if (activities.length !== 1) {
  throw new Error("Run tauri android init first; expected one generated MainActivity.kt.");
}

const activityPath = path.join(javaDirectory, activities[0]);
let activity = readFileSync(activityPath, "utf8");
const marker = "// WIE Launcher: fit the WebView around system UI and display cutouts.";
if (!activity.includes(marker)) {
  const imports = "import android.os.Bundle";
  const onCreate = "    super.onCreate(savedInstanceState)";
  if (!activity.includes(imports) || !activity.includes(onCreate)) {
    throw new Error("The generated Tauri activity template changed; review Android inset handling before building.");
  }
  activity = activity.replace(imports, `${imports}
import android.view.View
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat`);
  activity = activity.replace(onCreate, `${onCreate}
    ${marker}
    val content = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, windowInsets ->
      val occupied = windowInsets.getInsets(
        WindowInsetsCompat.Type.systemBars() or
          WindowInsetsCompat.Type.displayCutout() or WindowInsetsCompat.Type.ime()
      )
      view.setPadding(occupied.left, occupied.top, occupied.right, occupied.bottom)
      WindowInsetsCompat.CONSUMED
    }
    ViewCompat.requestApplyInsets(content)`);
  writeFileSync(activityPath, activity);
}
console.log("Android WebView inset handling prepared.");
