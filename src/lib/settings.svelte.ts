import { api, on } from "./api";
import { setLanguage } from "./i18n.svelte";

export const settings = $state({ v: {} as Record<string, any>, ready: false });

function applyMotion() {
  const m = settings.v["appearance.reduce_motion"];
  document.documentElement.classList.toggle("reduce-motion", m === "on");
  document.documentElement.classList.toggle("motion-on", m === "off");
}

export async function initSettings() {
  settings.v = await api.settingsAll();
  settings.ready = true;
  setLanguage(settings.v["general.language"] ?? "auto");
  applyMotion();
  on<{ key: string; value: any }>("settings://changed", ({ key, value }) => {
    settings.v[key] = value;
    if (key === "general.language") setLanguage(value);
    if (key === "appearance.reduce_motion") applyMotion();
  });
}

export function get<T = any>(key: string): T {
  return settings.v[key] as T;
}

export async function set(key: string, value: any) {
  settings.v[key] = value;
  try {
    settings.v[key] = await api.settingsSet(key, value);
  } catch (e) {
    console.warn("setting rejected", key, e);
    settings.v = await api.settingsAll();
  }
}

export function reducedMotion(): boolean {
  const m = settings.v["appearance.reduce_motion"];
  if (m === "on") return true;
  if (m === "off") return false;
  return matchMedia("(prefers-reduced-motion: reduce)").matches;
}
