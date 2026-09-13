import { runApp } from "./app";
import { AppMetadata } from "./app_library_store";
import { initializeLibrary } from "./library";
import { initializeSettings } from "./settings";
import { prepareLaunch } from "@pkg";
import { readLaunch, type LaunchProfile } from "./wasm-settings";
import { withGameSaveLock } from "./browser_save_store";

const originalConsoleError = console.error;
console.error = (...args: unknown[]) => {
  window.alert(String(args[0]));
  originalConsoleError(...args);
};

const main = async () => {
  const libraryView = document.getElementById("library-view") as HTMLDivElement;
  const playerView = document.getElementById("player-view") as HTMLElement;
  const settings = initializeSettings();
  const fontResponse = await fetch(new URL("../../../assets/neodgm.ttf", import.meta.url));
  if (!fontResponse.ok) {
    throw new Error(`Failed to load font: ${fontResponse.status} ${fontResponse.statusText}`);
  }
  const fontData = new Uint8Array(await fontResponse.arrayBuffer());

  const routeToApp = async (app: AppMetadata, archive: Uint8Array) => {
    let restarting: boolean;
    do {
      restarting = false;
      await withGameSaveLock(app.id, async () => {
        const launch = readLaunch(app.id);
        const profile: LaunchProfile = JSON.parse(await prepareLaunch(app.filename, archive, launch.namespace, JSON.stringify(launch.settings)));
        launch.profile = profile;
        launch.settings.phoneNumber = profile.phoneNumber;
        return new Promise<void>((resolve, reject) => {
          libraryView.hidden = true;
          playerView.hidden = false;

          let disposeApp: (() => void | Promise<void>) | undefined;
          let closing = false;
          const routeToLibrary = (error?: unknown) => {
            if (closing) return;
            closing = true;
            void Promise.resolve().then(() => disposeApp?.()).then(() => {
              playerView.hidden = true;
              libraryView.hidden = false;
              if (error !== undefined) reject(error); else resolve();
            }, stopError => {
              playerView.hidden = true;
              libraryView.hidden = false;
              reject(error ?? stopError);
            });
          };

          try {
            disposeApp = runApp(app, archive, fontData, settings, routeToLibrary, undefined, launch, () => {
              restarting = true;
              routeToLibrary();
            });
          } catch (error) {
            playerView.hidden = true;
            libraryView.hidden = false;
            reject(error);
          }
        });
      });
    } while (restarting);
  };

  await initializeLibrary(routeToApp, settings);
};

const start = () => {
  void main().catch((error) => {
    console.error(`라이브러리를 열 수 없습니다. ${String(error)}`, error);
  });
};

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", start);
} else {
  start();
}
