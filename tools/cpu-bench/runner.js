const workloads = new Map([["thumb_add", 0], ["thumb_memory", 1], ["arm_add", 2]]);
const modes = new Map([["off", 0], ["counts", 1], ["sampled", 2]]);
const instances = new Map();
const decoder = new TextDecoder();
let running = false;

function report(api) {
  const bytes = new Uint8Array(api.memory.buffer, api.bench_report_ptr(), api.bench_report_len());
  return JSON.parse(decoder.decode(bytes));
}

function check(api, status) {
  if (status !== 0) throw new Error(report(api).error ?? `Benchmark failed (${status})`);
}

function unsigned(value, name, positive = false) {
  if (!Number.isInteger(value) || value < (positive ? 1 : 0) || value > 0xffffffff) {
    throw new Error(`${name} must be ${positive ? "a positive" : "an unsigned"} 32-bit integer`);
  }
  return value;
}

export async function prepare(variant) {
  if (!["off", "profiled", "throughput", "table", "pc_local", "blocks"].includes(variant)) throw new Error(`Unknown variant ${variant}`);
  if (!instances.has(variant)) {
    const promise = (async () => {
      const response = await fetch(new URL(`${variant}.wasm`, import.meta.url));
      if (!response.ok) throw new Error(`${variant}.wasm: HTTP ${response.status}`);
      const bytes = await response.arrayBuffer();
      const { instance } = await WebAssembly.instantiate(bytes, { env: { profile_now: () => performance.now() * 1e6 } });
      return instance.exports;
    })();
    instances.set(variant, promise);
    promise.catch(() => instances.delete(variant));
  }
  return instances.get(variant);
}

export async function runTrial({ variant, mode = "off", workload, batches = 100, warmupBatches = 100, interval = 4096, seed = 1 }) {
  if (running) throw new Error("A benchmark trial is already running");
  if (!workloads.has(workload)) throw new Error(`Unknown workload ${workload}`);
  if (!modes.has(mode)) throw new Error(`Unknown profiling mode ${mode}`);
  unsigned(batches, "batches", true);
  unsigned(warmupBatches, "warmupBatches");
  unsigned(interval, "interval", true);
  unsigned(seed, "seed");
  if (batches > Math.floor(0xffffffff / 10000) || warmupBatches > Math.floor(0xffffffff / 10000)) {
    throw new Error("Instruction count exceeds the benchmark's 32-bit limit");
  }
  running = true;
  try {
    const api = await prepare(variant);
    const setup = () => {
      check(api, api.bench_setup(workloads.get(workload)));
      check(api, api.bench_configure(modes.get(mode), interval, seed));
    };
    if (warmupBatches > 0) {
      setup();
      check(api, api.bench_run(warmupBatches));
      check(api, api.bench_validate());
      await new Promise(resolve => setTimeout(resolve, 0));
    }
    setup();
    const start = performance.now();
    const status = api.bench_run(batches);
    const elapsedMs = performance.now() - start;
    check(api, status);
    check(api, api.bench_validate());
    const result = report(api);
    if (variant === "blocks" && (result.experimental_thumb_blocks !== true || result.full_profiling_compiled || result.throughput !== null)) throw new Error("Block experiment requires blocks without instrumentation");
    if (variant === "table" && (result.experimental_thumb_table !== true || result.experimental_pc_local || result.full_profiling_compiled || result.throughput !== null)) {
      throw new Error("Table experiment requires the table decoder and no CPU instrumentation");
    }
    if (variant === "pc_local" && (result.experimental_pc_local !== true || result.experimental_thumb_table || result.full_profiling_compiled || result.throughput !== null)) {
      throw new Error("PC-local experiment requires local PC reuse without the table decoder or CPU instrumentation");
    }
    return {
      formatVersion: 1,
      runtime: "wasm",
      variant,
      mode,
      workload,
      batches,
      warmupBatches,
      interval,
      seed,
      elapsedMs,
      guestInstructions: batches * 10000,
      ...result,
    };
  } finally {
    running = false;
  }
}

export const protocol = {
  instructionsPerBatch: 10000,
  workloads: [...workloads.keys()],
  modes: [...modes.keys()],
  variants: ["off", "profiled", "throughput", "table", "pc_local", "blocks"],
};

export async function calibrateClock({ pairs = 10000 } = {}) {
  if (running) throw new Error("A benchmark trial is already running");
  unsigned(pairs, "pairs", true);
  running = true;
  try {
    const api = await prepare("profiled");
    check(api, api.bench_calibrate_clock(pairs));
    return report(api);
  } finally {
    running = false;
  }
}

window.wieBench = { prepare, runTrial, calibrateClock, protocol };
