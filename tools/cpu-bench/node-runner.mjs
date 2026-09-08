import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { cpus, platform, release, arch } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [artifactArgument, planArgument, outputArgument, ...extra] = process.argv.slice(2);
if (!artifactArgument || !planArgument || !outputArgument || extra.length) {
  throw new Error("Usage: node [V8 flags] tools/cpu-bench/node-runner.mjs ARTIFACT_CPU_BENCH_DIR PLAN_JSON OUTPUT_JSON");
}
const artifacts = resolve(artifactArgument);
const plan = JSON.parse(readFileSync(planArgument, "utf8"));
const conditions = plan.conditions ?? [
  { variant: "off", mode: "off" },
  { variant: "profiled", mode: "off" },
  { variant: "profiled", mode: "counts" },
  { variant: "profiled", mode: "sampled" },
];
const conditionIndices = conditions.map((_, index) => index).join(",");
if (!conditions.length || !plan.balancedOrder?.length || plan.balancedOrder.some(order => [...order].sort((a, b) => a - b).join(",") !== conditionIndices)) {
  throw new Error("Every trial order must include all requested conditions exactly once");
}
// Reuse the WebView's exact setup/run/validation implementation in Node.
// Only asset fetching and the browser window global need local adapters.
globalThis.window = globalThis;
globalThis.fetch = async url => new Response(readFileSync(url), { status: 200 });
await import(pathToFileURL(join(artifacts, "runner.js")));
// Node can otherwise exit while V8's eager optimizing compilation is pending.
// This timer is cleared before warmup or measured work starts.
const compilationKeepAlive = setInterval(() => {}, 1000);
const variants = [...new Set(conditions.map(condition => condition.variant))];
const includeClockCalibration = plan.clockCalibration ?? variants.includes("profiled");
try {
  await Promise.all([...new Set([...variants, ...(includeClockCalibration ? ["profiled"] : [])])].map(variant => wieBench.prepare(variant)));
} finally {
  clearInterval(compilationKeepAlive);
}
const hash = name => createHash("sha256").update(readFileSync(join(artifacts, name))).digest("hex");
const result = {
  runtime: "mac_node_wasm",
  runtimeScope: "Local Node.js Wasm; separate from Android WebView measurements",
  node: process.version,
  v8: process.versions.v8,
  compilerFlags: process.execArgv,
  host: { platform: platform(), release: release(), arch: arch(), cpu: cpus()[0]?.model },
  artifactSha256: { ...Object.fromEntries(variants.map(variant => [variant, hash(`${variant}.wasm`)])), runner: hash("runner.js") },
  protocol: { ...plan, conditions, instructionsPerBatch: 10000, setupAndWarmupExcluded: true },
  clockCalibration: includeClockCalibration ? await wieBench.calibrateClock({ pairs: 10000 }) : null,
  trials: [],
};
for (const [workload, batches] of Object.entries(plan.batches)) {
  for (const [repeat, order] of plan.balancedOrder.entries()) {
    for (const [position, condition] of order.entries()) {
      const trial = await wieBench.runTrial({ ...conditions[condition], workload, batches, warmupBatches: plan.warmupBatches, interval: plan.interval, seed: plan.seed });
      if (!trial.validated || trial.validation.instructions !== trial.guestInstructions) throw new Error("Trial failed validation");
      result.trials.push({ repeat, position, recordedUtc: new Date().toISOString(), ...trial, millionInstructionsPerSecond: trial.guestInstructions / trial.elapsedMs / 1000 });
      await new Promise(resolve => setTimeout(resolve, plan.pauseMs ?? 150));
    }
  }
}
const median = values => {
  const sorted = [...values].sort((a, b) => a - b);
  return (sorted[Math.floor((sorted.length - 1) / 2)] + sorted[Math.floor(sorted.length / 2)]) / 2;
};
result.summary = Object.fromEntries(Object.keys(plan.batches).map(workload => [workload, conditions.map(condition => {
  const rows = result.trials.filter(row => row.workload === workload && row.variant === condition.variant && row.mode === condition.mode);
  return { ...condition, observations: rows.length, medianElapsedMs: median(rows.map(row => row.elapsedMs)), medianMillionInstructionsPerSecond: median(rows.map(row => row.millionInstructionsPerSecond)) };
})]));
writeFileSync(outputArgument, JSON.stringify(result, null, 2) + "\n");
console.log(JSON.stringify(result.summary, null, 2));
