import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { info, error } from "@tauri-apps/plugin-log";

export type UpdateStatus =
  | { stage: "checking" }
  | { stage: "none" }
  | { stage: "available"; version: string }
  | { stage: "downloading"; progress: number; total: number | null }
  | { stage: "installing" }
  | { stage: "done" }
  | { stage: "error"; message: string };

export interface UpdateCallbacks {
  onStatus: (status: UpdateStatus) => void;
}

/**
 * Check for updates on app startup (silent — logs only).
 */
export async function checkForUpdate(): Promise<void> {
  try {
    const update = await check();
    if (!update) {
      info("[updater] No update available");
      return;
    }

    info(`[updater] Update available: ${update.version}`);
    await downloadAndInstall(update, {
      onStatus(s) {
        if (s.stage === "error") error(`[updater] ${s.message}`);
        else info(`[updater] ${s.stage}`);
      },
    });
  } catch (e) {
    error(`[updater] Update check failed: ${e}`);
  }
}

/**
 * Interactive update check — call from the About page.
 * Fires callbacks for each stage so the UI can show toasts.
 */
export async function checkForUpdateInteractive(cbs: UpdateCallbacks): Promise<void> {
  try {
    cbs.onStatus({ stage: "checking" });

    const update = await check();
    if (!update) {
      cbs.onStatus({ stage: "none" });
      return;
    }

    cbs.onStatus({ stage: "available", version: update.version });
    await downloadAndInstall(update, cbs);
  } catch (e) {
    cbs.onStatus({ stage: "error", message: String(e) });
  }
}

async function downloadAndInstall(update: Update, cbs: UpdateCallbacks): Promise<void> {
  let contentLength = 0;

  await update.download((event) => {
    switch (event.event) {
      case "Started":
        contentLength = event.data.contentLength ?? 0;
        cbs.onStatus({ stage: "downloading", progress: 0, total: contentLength });
        break;
      case "Progress":
        cbs.onStatus({
          stage: "downloading",
          progress: contentLength > 0
            ? Math.round((event.data.chunkLength / contentLength) * 100)
            : 0,
          total: contentLength,
        });
        break;
      case "Finished":
        cbs.onStatus({ stage: "installing" });
        break;
    }
  });

  await update.install();
  cbs.onStatus({ stage: "done" });
  await relaunch();
}
