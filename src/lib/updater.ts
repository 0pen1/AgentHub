import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export interface UpdateProgress {
  received: number;
  total?: number;
}

/**
 * Check GitHub Releases (via latest.json) for a newer version.
 * Resolves null when the app is up to date.
 */
export async function checkForUpdate(): Promise<Update | null> {
  return check();
}

/**
 * Download the signed update package and install it.
 * - Windows: the installer takes over and the app exits automatically.
 * - macOS/Linux: the new version is staged — call relaunch() to apply.
 */
export async function downloadAndInstall(
  update: Update,
  onProgress?: (p: UpdateProgress) => void,
): Promise<void> {
  await update.downloadAndInstall((event) => {
    switch (event.event) {
      case "Started":
        onProgress?.({ received: 0, total: event.data.contentLength });
        break;
      case "Progress":
        onProgress?.({ received: event.data.chunkLength });
        break;
      case "Finished":
        onProgress?.({ received: -1 });
        break;
    }
  });
}

/** Restart the app into the freshly installed version (macOS/Linux). */
export async function relaunchApp(): Promise<void> {
  await relaunch();
}
