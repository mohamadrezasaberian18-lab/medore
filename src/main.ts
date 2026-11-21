import { mount } from "svelte";
import { invoke } from "@tauri-apps/api/core";
import "vazirmatn/Vazirmatn-Variable-font-face.css";
import "./app.css";

const isOverlay = location.hash === "#overlay";
const where = isOverlay ? "overlay" : "main";
document.body.classList.add(isOverlay ? "overlay" : "main");
// the built-in browser menu is never wanted
window.addEventListener("contextmenu", (e) => e.preventDefault());

// Interface errors go to MEDORE's log file.
function log(level: string, msg: string) {
  invoke("js_log", { level, msg: `[${where}] ${msg}` }).catch(() => {});
}
window.addEventListener("error", (e) => log("error", `${e.message} at ${e.filename}:${e.lineno}`));
window.addEventListener("unhandledrejection", (e) => log("error", `unhandled: ${e.reason?.stack ?? e.reason}`));

async function start() {
  log("info", "start");
  await invoke("wait_ready");
  const { initSettings } = await import("./lib/settings.svelte");
  const { initPlayer } = await import("./lib/player.svelte");
  const { initLibrary } = await import("./lib/library.svelte");
  await initSettings();
  log("info", "settings ready");
  await Promise.all([initPlayer().then(() => log("info", "player ready")), initLibrary().then(() => log("info", "library ready"))]);
  const target = document.getElementById("app")!;
  if (isOverlay) {
    const { default: Overlay } = await import("./overlay/Overlay.svelte");
    mount(Overlay, { target });
  } else {
    const { default: App } = await import("./main/App.svelte");
    mount(App, { target });
  }
  log("info", "mounted");
  hideSplash();
}

/** Let the opening animation finish (about 1.6 s), then fade into the app. */
function hideSplash() {
  const el = document.getElementById("splash");
  if (!el) return;
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const elapsed = performance.now() - ((window as any).__splashStart ?? 0);
  const wait = Math.max(0, (reduced ? 300 : 1700) - elapsed);
  setTimeout(() => {
    el.classList.add("out");
    setTimeout(() => el.remove(), 450);
  }, wait);
}

start().catch((e) => {
  log("error", `start failed: ${e?.stack ?? e}`);
  document.getElementById("splash")?.remove();
  const box = document.getElementById("app");
  if (box && !isOverlay) {
    box.innerHTML = `<div style="height:100vh;display:grid;place-items:center;text-align:center;color:#a6a6a6;font:14px 'Segoe UI',sans-serif">
      <div><div style="color:#fff;font-size:18px;font-weight:600;margin-bottom:8px">MEDORE could not open its window</div>
      <button style="margin-top:12px;height:36px;padding:0 16px;border-radius:10px;border:0;background:#e0e0e0;color:#080808;font-weight:600" onclick="location.reload()">Reload</button></div></div>`;
  }
});
