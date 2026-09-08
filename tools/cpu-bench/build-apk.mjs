import { copyFile, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const [frontendArgument, outputArgument, ...extra] = process.argv.slice(2);
if (!frontendArgument || !outputArgument || extra.length) {
  throw new Error("Usage: node tools/cpu-bench/build-apk.mjs STAGED_FRONTEND OUTPUT_APK");
}
const root = fileURLToPath(new URL("../../", import.meta.url));
const app = join(root, "wie-app");
const frontend = resolve(frontendArgument);
const output = resolve(outputArgument);
const sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
if (!sdk) throw new Error("Set ANDROID_HOME to an installed Android SDK");
await readFile(join(frontend, "cpu-bench", "index.html"));
// Older retained diagnostic stages predate the selectable variant manifest.
const variantManifest = await readFile(join(frontend, "cpu-bench", "variants.txt"), "utf8").catch(error => {
  if (error.code !== "ENOENT") throw error;
  return "off profiled";
});
const variants = variantManifest.trim().split(/\s+/);
if (!variants.includes("off") || variants.some(variant => !["off", "profiled", "throughput", "table", "pc_local", "blocks"].includes(variant))) {
  throw new Error("Invalid benchmark variant manifest");
}
for (const variant of variants) await readFile(join(frontend, "cpu-bench", `${variant}.wasm`));
const versions = (await readdir(join(sdk, "build-tools"))).sort((a, b) => b.localeCompare(a, undefined, { numeric: true }));
if (!versions.length) throw new Error("Install Android SDK build-tools");
const aapt = join(sdk, "build-tools", versions[0], process.platform === "win32" ? "aapt.exe" : "aapt");

await import("./prepare-diagnostic.mjs");
const work = join(root, "target", "cpu-bench-apk");
await mkdir(work, { recursive: true });
const config = join(work, "tauri.diagnostic.json");
await writeFile(config, JSON.stringify({
  productName: "WIE CPU Diagnostics",
  build: { frontendDist: relative(app, frontend).replaceAll("\\", "/"), beforeBuildCommand: "" },
  app: { windows: [{ label: "main", title: "WIE CPU Diagnostics", url: "cpu-bench/index.html", width: 1100, height: 760, dragDropEnabled: false, resizable: true, fullscreen: false }] },
}, null, 2) + "\n");
const built = join(app, "gen", "android", "app", "build", "outputs", "apk", "universal", "debug", "app-universal-debug.apk");
// Incremental APK packaging can leave unused ZIP gaps after large asset changes.
// Remove only this generated output so Gradle packages a fresh diagnostic APK.
await rm(built, { force: true });
const build = spawnSync(process.env.TAURI_BIN ?? "tauri", ["android", "build", "--debug", "--target", "aarch64", "--apk", "--ci", "--config", config], {
  cwd: app,
  stdio: "inherit",
  env: { ...process.env, ORG_GRADLE_PROJECT_wieDiagnostic: "true", CARGO_PROFILE_DEV_DEBUG: "0" },
});
if (build.error) throw build.error;
if (build.status !== 0) throw new Error(`Diagnostic build failed (${build.status})`);

const audit = spawnSync(aapt, ["dump", "badging", built], { encoding: "utf8" });
if (audit.error) throw audit.error;
if (audit.status !== 0 || !/^package: name='local\.wie\.launcher\.profile'/m.test(audit.stdout) || !/^application-debuggable$/m.test(audit.stdout)) {
  throw new Error("Refusing to copy an APK without the isolated diagnostic package ID and debug flag");
}
await mkdir(dirname(output), { recursive: true });
await copyFile(built, output);
console.log(`Verified isolated package local.wie.launcher.profile: ${output}`);
