import { WieWeb } from "@pkg";
import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Settings, createIcons } from "lucide";

import { AppMetadata } from "./app_library_store";
import { bindGameInput } from "./game_input";
import { SettingsController } from "./settings";
import { installCpuProfile } from "./cpu_profile";
import { installCpuThroughput } from "./cpu_throughput";

declare const __WIE_CPU_PROFILING__: boolean;
declare const __WIE_CPU_THROUGHPUT__: boolean;

const icons = {
  ArrowDown,
  ArrowLeft,
  ArrowRight,
  ArrowUp,
  Settings,
};

export const runApp = (app: AppMetadata, archive: Uint8Array, fontData: Uint8Array, settings: SettingsController, exit: (error?: unknown) => void) => {
  const playerView = document.getElementById("player-view") as HTMLElement;
  const playerTitle = document.getElementById("player-title") as HTMLElement;
  const canvas = document.getElementById("canvas") as HTMLCanvasElement;
  const backToLibrary = document.getElementById("back-to-library") as HTMLButtonElement;
  const appSettings = document.getElementById("app-settings") as HTMLButtonElement;

  canvas.width = 240;
  canvas.height = 320;
  let screenWidth = canvas.width;
  let screenHeight = canvas.height;
  playerView.style.setProperty("--screen-ratio", String(screenWidth / screenHeight));
  const abortController = new AbortController();
  const wieWeb = new WieWeb(app.filename, archive, canvas, fontData);
  const removeCpuProfile = __WIE_CPU_PROFILING__ ? installCpuProfile(wieWeb) : undefined;
  const removeCpuThroughput = __WIE_CPU_THROUGHPUT__ ? installCpuThroughput(wieWeb) : undefined;
  const unsubscribePcmVolume = settings.onPcmVolumeChange((volume) => wieWeb.set_pcm_volume(volume));
  let running = true;

  wieWeb.set_pcm_volume(settings.pcmVolume);
  playerTitle.textContent = app.title;
  createIcons({ icons, root: playerView });
  const input = bindGameInput(playerView, wieWeb);

  backToLibrary.addEventListener("click", () => exit(), { signal: abortController.signal });
  appSettings.addEventListener("click", () => {
    input.releaseAll();
    settings.open();
  }, { signal: abortController.signal });

  const update = () => {
    if (!running) {
      return;
    }

    try {
      wieWeb.update();
      if (canvas.width !== screenWidth || canvas.height !== screenHeight) {
        screenWidth = canvas.width;
        screenHeight = canvas.height;
        playerView.style.setProperty("--screen-ratio", String(screenWidth / screenHeight));
      }
      if (wieWeb.has_exited()) {
        exit();
        return;
      }
      requestAnimationFrame(update);
    } catch (error) {
      exit(error);
    }
  };
  requestAnimationFrame(update);

  return () => {
    running = false;
    input.dispose();
    abortController.abort();
    unsubscribePcmVolume();
    removeCpuProfile?.();
    removeCpuThroughput?.();
    wieWeb.free();
  };
};
