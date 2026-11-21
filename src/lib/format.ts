import { num, t, i18n } from "./i18n.svelte";

/** 1:23:45 / 3:07 */
export function clock(sec: number | null | undefined): string {
  if (sec == null || !isFinite(sec) || sec < 0) sec = 0;
  const s = Math.floor(sec);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = s % 60;
  const out = h > 0 ? `${h}:${String(m).padStart(2, "0")}:${String(r).padStart(2, "0")}` : `${m}:${String(r).padStart(2, "0")}`;
  return num(out);
}

/** "1 h 42 min" */
export function duration(sec: number | null | undefined): string {
  if (!sec || sec < 1) return "";
  const m = Math.round(sec / 60);
  if (m < 1) return t("{n} sec", { n: Math.round(sec) });
  const h = Math.floor(m / 60);
  const mm = m % 60;
  if (h === 0) return t("{n} min", { n: mm });
  if (mm === 0) return t("{n} h", { n: h });
  return t("{h} h {m} min", { h, m: mm });
}

export function bytes(n: number | null | undefined): string {
  if (!n) return "";
  const u = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < u.length - 1) {
    v /= 1024;
    i++;
  }
  return num(`${v >= 100 || i === 0 ? Math.round(v) : v.toFixed(1)} ${u[i]}`);
}

export function bitrate(bps: number | null | undefined): string {
  if (!bps) return "";
  return bps >= 1e6 ? num(`${(bps / 1e6).toFixed(1)} Mb/s`) : num(`${Math.round(bps / 1e3)} kb/s`);
}

export function resolutionLabel(w: number, h: number): string {
  if (!w || !h) return "";
  const p = Math.min(w, h) === h ? h : w;
  if (w >= 7000 || p >= 4000) return "8K";
  if (w >= 3200 || p >= 2000) return "4K";
  if (w >= 2400 || p >= 1400) return "1440p";
  if (w >= 1800 || p >= 1000) return "1080p";
  if (w >= 1200 || p >= 700) return "720p";
  if (p >= 560) return "576p";
  if (p >= 460) return "480p";
  return `${p}p`;
}

export function relativeDay(unix: number): string {
  if (!unix) return "";
  const d = new Date(unix * 1000);
  const now = new Date();
  const days = Math.floor((new Date(now.toDateString()).getTime() - new Date(d.toDateString()).getTime()) / 86400000);
  if (days <= 0) return t("Today");
  if (days === 1) return t("Yesterday");
  if (days < 7) return t("{n} days ago", { n: days });
  return d.toLocaleDateString(i18n.lang === "fa" ? "fa-IR" : undefined, { year: "numeric", month: "short", day: "numeric" });
}

export function episodeTag(season?: number | null, episode?: number | null, end?: number | null): string {
  if (episode == null) return "";
  let s = `S${String(season ?? 1).padStart(2, "0")}E${String(episode).padStart(2, "0")}`;
  if (end && end !== episode) s += `–E${String(end).padStart(2, "0")}`;
  return s;
}

/** Locale-aware natural comparison (Persian and English, numbers in order). */
const collators: Record<string, Intl.Collator> = {};
export function compareText(a: string, b: string): number {
  const key = i18n.lang;
  const c = (collators[key] ??= new Intl.Collator(key === "fa" ? ["fa", "en"] : ["en", "fa"], { numeric: true, sensitivity: "base" }));
  return c.compare(a || "", b || "");
}
