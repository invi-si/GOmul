interface ThroughputCpu {
  throughput_reset(): void;
  throughput_snapshot(): unknown;
}

// Removed from normal builds. The separate feature omits detailed stage hooks.
export function installCpuThroughput(emulator: object): () => void {
  const cpu = emulator as ThroughputCpu;
  const throughput = {
    reset: () => cpu.throughput_reset(),
    snapshot: () => cpu.throughput_snapshot(),
  };
  const target = window as Window & { __wieCpuThroughput?: typeof throughput };
  target.__wieCpuThroughput = throughput;
  return () => {
    if (target.__wieCpuThroughput === throughput) delete target.__wieCpuThroughput;
  };
}
