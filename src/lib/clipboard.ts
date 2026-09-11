import { writeText } from "@tauri-apps/plugin-clipboard-manager";

/**
 * Copy text to the system clipboard.
 *
 * Primary path is the native clipboard plugin — Linux webkitgtk has no
 * navigator.clipboard at all (wry#719) and macOS WKWebView throttles it
 * outside user gestures. navigator.clipboard stays as a fallback for plain
 * browser (vite dev) contexts where the plugin isn't injected.
 */
export async function copyText(text: string): Promise<void> {
  try {
    await writeText(text);
  } catch {
    await navigator.clipboard.writeText(text);
  }
}
