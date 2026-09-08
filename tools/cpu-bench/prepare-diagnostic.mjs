import { copyFile, readFile, writeFile } from "node:fs/promises";

const gradle = new URL("../../wie-app/gen/android/app/build.gradle.kts", import.meta.url);
let source = await readFile(gradle, "utf8");
// Tauri regenerates/removes applicationIdSuffix lines in this file. Keep the
// optional diagnostic configuration in a separately applied Gradle script.
source = source.replace(/            \/\/ The diagnostic application has its own data directory; JNI namespace stays unchanged\.\n            if \(providers\.gradleProperty\("wieDiagnostic"\)\.orNull == "true"\) \{\n(?:                applicationIdSuffix = "\.profile"\n)?            \}\n/, "");
const marker = 'apply(from = "wie-diagnostic.gradle.kts")';
if (!source.includes(marker)) source += `\n\n${marker}\n`;
await writeFile(gradle, source);
await copyFile(new URL("diagnostic.gradle.kts", import.meta.url), new URL("../../wie-app/gen/android/app/wie-diagnostic.gradle.kts", import.meta.url));
