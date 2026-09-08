// This diagnostics surface is removed by webpack from normal builds.
interface ProfiledCpu {
  profile_configure(mode: string, meanInterval: number, seed: number): void;
  profile_reset(): void;
  profile_snapshot(): unknown;
}

export function installCpuProfile(emulator: object): () => void {
  const cpu = emulator as ProfiledCpu;
  const profile = {
    configure: (mode: string, meanInterval = 1024, seed = 1) => cpu.profile_configure(mode, meanInterval, seed),
    reset: () => cpu.profile_reset(),
    snapshot: () => cpu.profile_snapshot(),
  };
  const target = window as Window & { __wieCpuProfile?: typeof profile };
  target.__wieCpuProfile = profile;
  return () => {
    if (target.__wieCpuProfile === profile) delete target.__wieCpuProfile;
  };
}
