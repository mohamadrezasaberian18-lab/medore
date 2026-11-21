// Menus shared by the player's "more" (⋯) button and its right-click menu,
// so both always offer the same actions.
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { api, SUB_EXTS, VIDEO_EXTS, AUDIO_EXTS } from "./api";
import { player, P, act, type Track } from "./player.svelte";
import { t, num } from "./i18n.svelte";
import { toast, type MenuItem } from "./ui.svelte";
import { languageName } from "./languages";
import { settings, set } from "./settings.svelte";

export function trackLabel(tr: Track, i: number): string {
  const parts: string[] = [];
  const lang = tr.lang ? languageName(tr.lang) : "";
  if (tr.title && tr.title.trim() && tr.title.toLowerCase() !== (tr.lang || "").toLowerCase()) {
    parts.push(lang && !tr.title.toLowerCase().includes(lang.toLowerCase()) ? `${lang} · ${tr.title}` : tr.title);
  } else if (lang) parts.push(lang);
  if (!parts.length) parts.push(tr.type === "sub" ? t("Subtitle {n}", { n: i + 1 }) : t("Track {n}", { n: i + 1 }));
  if (tr.forced && !parts[0].toLowerCase().includes("forced")) parts.push(t("Forced"));
  if (tr["hearing-impaired"]) parts.push("SDH");
  return parts.join(" · ");
}

function trackHint(tr: Track): string {
  const bits: string[] = [];
  if (tr.external) bits.push(t("File"));
  if (tr.type === "audio") {
    const ch = tr["demux-channel-count"] ?? tr["audio-channels"];
    if (ch) bits.push(ch === 6 ? "5.1" : ch === 8 ? "7.1" : ch === 2 ? t("Stereo") : ch === 1 ? t("Mono") : num(ch) + "ch");
    if (tr.codec) bits.push(tr.codec.toUpperCase().replace("TRUEHD", "TrueHD"));
  } else if (tr.codec && tr.type === "sub") {
    const c = tr.codec.toLowerCase();
    bits.push(c.includes("pgs") ? "PGS" : c.includes("dvd") ? "VobSub" : c === "subrip" ? "SRT" : c.toUpperCase());
  }
  return bits.join(" · ");
}

export async function pickSubtitle() {
  const f = await openDialog({ multiple: false, directory: false, filters: [{ name: t("Subtitles"), extensions: SUB_EXTS.filter((e) => e !== "zip") }] });
  if (typeof f === "string") {
    await api.addSubtitle(f);
    toast(t("Subtitle added"), { icon: "captions" });
  }
}

export async function pickMedia(mode: "replace" | "append" = "replace") {
  const f = await openDialog({
    multiple: true,
    directory: false,
    filters: [
      { name: t("Media files"), extensions: [...VIDEO_EXTS, ...AUDIO_EXTS, "m3u", "m3u8", "pls"] },
      { name: t("All files"), extensions: ["*"] },
    ],
  });
  const files = Array.isArray(f) ? f : typeof f === "string" ? [f] : [];
  if (files.length) await api.open(files, { mode, sort: true });
}

export function subtitleItems(): MenuItem[] {
  const subs = P.tracks.filter((x) => x.type === "sub");
  const sid = player.p["sid"];
  const delay = player.p["sub-delay"] ?? 0;
  const items: MenuItem[] = [{ label: t("Off"), radio: true, checked: sid === false || sid == null, onSelect: () => api.set("sid", "no") }];
  subs.forEach((s, i) => items.push({ label: trackLabel(s, i), hint: trackHint(s), radio: true, checked: sid === s.id, onSelect: () => api.set("sid", s.id) }));
  items.push(
    { separator: true },
    { label: t("Add subtitle file…"), icon: "file-plus", onSelect: pickSubtitle },
    { separator: true },
    { header: t("Delay") + "  " + num((delay >= 0 ? "+" : "") + delay.toFixed(1)) + " " + t("s") },
    { label: t("Earlier (−0.1 s)"), hint: "Z", keepOpen: true, onSelect: () => act.subDelayBy(-0.1) },
    { label: t("Later (+0.1 s)"), hint: "X", keepOpen: true, onSelect: () => act.subDelayBy(0.1) },
    { label: t("Reset delay"), disabled: Math.abs(delay) < 0.001, onSelect: () => api.set("sub-delay", 0) },
    { separator: true },
    {
      label: t("Size"),
      submenu: () =>
        [70, 85, 100, 115, 130, 150].map((p) => ({
          label: num(p) + "%",
          radio: true,
          checked: Math.round((player.p["sub-scale"] ?? 1) * 100) === p,
          onSelect: () => api.set("sub-scale", p / 100),
        })),
    },
    {
      label: t("Position"),
      submenu: () => [
        { label: t("Bottom"), radio: true, checked: (player.p["sub-pos"] ?? 100) >= 99, onSelect: () => api.set("sub-pos", 100) },
        { label: t("Higher"), radio: true, checked: (player.p["sub-pos"] ?? 100) < 99 && (player.p["sub-pos"] ?? 100) >= 85, onSelect: () => api.set("sub-pos", 90) },
        { label: t("Much higher"), radio: true, checked: (player.p["sub-pos"] ?? 100) < 85, onSelect: () => api.set("sub-pos", 78) },
      ],
    },
    { label: t("Show subtitles"), checked: player.p["sub-visibility"] !== false, onSelect: () => act.toggleSubs(), hint: "V" },
  );
  return items;
}

export function audioItems(): MenuItem[] {
  const aud = P.tracks.filter((x) => x.type === "audio");
  const aid = player.p["aid"];
  const devices: { name: string; description: string }[] = player.p["audio-device-list"] ?? [];
  const dev = player.p["audio-device"] ?? "auto";
  const items: MenuItem[] = aud.length
    ? aud.map((a, i) => ({ label: trackLabel(a, i), hint: trackHint(a), radio: true, checked: aid === a.id, onSelect: () => api.set("aid", a.id) }))
    : [{ label: t("No audio"), disabled: true }];
  items.push(
    { separator: true },
    {
      label: t("Output device"),
      submenu: () =>
        devices.map((d) => ({
          label: d.name === "auto" ? t("System default") : d.description.replace(/^Windows Audio Session \(WASAPI\)\s*:?\s*/i, "").replace(/\(WASAPI\)/i, "").trim() || d.name,
          radio: true,
          checked: d.name === dev,
          onSelect: () => {
            api.set("audio-device", d.name);
            set("audio.device", d.name);
          },
        })),
    },
    { label: t("Normalize volume"), checked: !!settings.v["audio.normalize"], onSelect: () => set("audio.normalize", !settings.v["audio.normalize"]) },
    {
      label: t("Audio delay"),
      submenu: () => {
        const d = player.p["audio-delay"] ?? 0;
        return [
          { header: num((d >= 0 ? "+" : "") + d.toFixed(2)) + " " + t("s") },
          { label: t("Earlier (−0.05 s)"), keepOpen: true, onSelect: () => api.cmd("add", "audio-delay", -0.05) },
          { label: t("Later (+0.05 s)"), keepOpen: true, onSelect: () => api.cmd("add", "audio-delay", 0.05) },
          { label: t("Reset"), onSelect: () => api.set("audio-delay", 0) },
        ];
      },
    },
  );
  return items;
}

export const SPEEDS = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 2.5, 3];

export function speedItems(): MenuItem[] {
  const cur = player.p.speed ?? 1;
  return SPEEDS.map((s) => ({ label: s === 1 ? t("Normal") : num(s) + "×", radio: true, checked: Math.abs(cur - s) < 0.001, onSelect: () => act.setSpeed(s) }));
}

export function videoItems(): MenuItem[] {
  const aspect = player.p["video-aspect-override"];
  const rot = player.p["video-rotate"] ?? 0;
  const zoom = player.p["video-zoom"] ?? 0;
  const aspects: [string, string][] = [
    [t("Original"), "no"],
    ["16:9", "16:9"],
    ["4:3", "4:3"],
    ["21:9", "2.33"],
    ["1.85:1", "1.85"],
    ["2.39:1", "2.39"],
  ];
  return [
    {
      label: t("Aspect ratio"),
      submenu: () =>
        aspects.map(([l, v]) => ({
          label: l,
          radio: true,
          checked: v === "no" ? aspect == null || aspect === -1 || aspect === "no" || aspect === 0 : String(aspect).startsWith(v.split(":")[0]),
          onSelect: () => api.set("video-aspect-override", v),
        })),
    },
    {
      label: t("Zoom"),
      submenu: () => [
        { label: t("Fit"), radio: true, checked: Math.abs(zoom) < 0.01 && !player.p["panscan"], onSelect: () => (api.set("video-zoom", 0), api.set("panscan", 0)) },
        { label: t("Fill (crop edges)"), radio: true, checked: !!player.p["panscan"], onSelect: () => (api.set("video-zoom", 0), api.set("panscan", 1)) },
        { label: num(110) + "%", radio: true, checked: Math.abs(zoom - Math.log2(1.1)) < 0.01, onSelect: () => api.set("video-zoom", Math.log2(1.1)) },
        { label: num(125) + "%", radio: true, checked: Math.abs(zoom - Math.log2(1.25)) < 0.01, onSelect: () => api.set("video-zoom", Math.log2(1.25)) },
      ],
    },
    {
      label: t("Rotate"),
      submenu: () =>
        [0, 90, 180, 270].map((r) => ({ label: num(r) + "°", radio: true, checked: rot === r, onSelect: () => api.set("video-rotate", r) })),
    },
    {
      label: t("Deinterlace"),
      submenu: () =>
        [
          ["auto", t("Automatic")],
          ["yes", t("On")],
          ["no", t("Off")],
        ].map(([v, l]) => ({ label: l, radio: true, checked: String(player.p.deinterlace ?? "auto") === v || (v === "yes" && player.p.deinterlace === true) || (v === "no" && player.p.deinterlace === false), onSelect: () => api.set("deinterlace", v) })),
    },
  ];
}

export function moreItems(opts: { onInfo: () => void; onQueue: () => void; onShortcuts: () => void; onSettings: () => void }): MenuItem[] {
  const loop = player.p["loop-file"];
  const ab = player.p["ab-loop-a"] != null && player.p["ab-loop-a"] !== "no";
  return [
    { label: t("Open file…"), icon: "file-plus", hint: "Ctrl+O", onSelect: () => pickMedia("replace") },
    { label: t("Add to queue…"), icon: "list-plus", onSelect: () => pickMedia("append") },
    { label: t("Add subtitle file…"), icon: "captions", onSelect: pickSubtitle },
    { separator: true },
    { label: t("Subtitles"), icon: "subtitles", submenu: subtitleItems },
    { label: t("Audio"), icon: "audio-lines", submenu: audioItems },
    { label: t("Speed"), icon: "gauge", submenu: speedItems },
    { label: t("Picture"), icon: "monitor", submenu: videoItems },
    { separator: true },
    { label: t("Take screenshot"), icon: "camera", hint: "S", onSelect: screenshot },
    { label: t("Picture in picture"), icon: "picture-in-picture-2", hint: "P", checked: player.pip, onSelect: () => act.pip() },
    { label: t("Always on top"), icon: "pin", hint: "T", checked: player.onTop, onSelect: () => act.onTop(!player.onTop) },
    { label: t("Repeat this file"), icon: "repeat-1", checked: loop === "inf" || loop === true, onSelect: act.loopFile },
    { label: ab ? t("Clear A-B loop") : t("A-B loop (set start, then end)"), icon: "repeat", hint: "L", onSelect: act.abLoop },
    { separator: true },
    { label: t("Queue"), icon: "list-video", hint: "Q", onSelect: opts.onQueue },
    { label: t("Media info"), icon: "info", hint: "I", onSelect: opts.onInfo },
    { label: t("Keyboard shortcuts"), icon: "keyboard", hint: "F1", onSelect: opts.onShortcuts },
    { label: t("Settings"), icon: "settings", hint: "Ctrl+,", onSelect: opts.onSettings },
  ];
}

export function contextItems(opts: { onInfo: () => void; onQueue: () => void; onShortcuts: () => void; onSettings: () => void }): MenuItem[] {
  return [
    { label: P.paused ? t("Play") : t("Pause"), icon: P.paused ? "play" : "pause", hint: t("Space"), onSelect: act.togglePause },
    { label: t("Next"), icon: "skip-forward", hint: "Shift+N", disabled: P.playlistPos >= P.playlist.length - 1, onSelect: act.next },
    { label: t("Previous"), icon: "skip-back", hint: "Shift+B", onSelect: act.prev },
    { separator: true },
    { label: t("Subtitles"), icon: "subtitles", submenu: subtitleItems },
    { label: t("Audio"), icon: "audio-lines", submenu: audioItems },
    { label: t("Speed"), icon: "gauge", submenu: speedItems },
    { label: t("Picture"), icon: "monitor", submenu: videoItems },
    { separator: true },
    { label: player.fullscreen ? t("Exit full screen") : t("Full screen"), icon: player.fullscreen ? "minimize" : "maximize", hint: "F", onSelect: () => act.fullscreen() },
    { label: t("Picture in picture"), icon: "picture-in-picture-2", hint: "P", checked: player.pip, onSelect: () => act.pip() },
    { label: t("Always on top"), icon: "pin", hint: "T", checked: player.onTop, onSelect: () => act.onTop(!player.onTop) },
    { label: t("Take screenshot"), icon: "camera", hint: "S", onSelect: screenshot },
    { separator: true },
    { label: t("Queue"), icon: "list-video", hint: "Q", onSelect: opts.onQueue },
    { label: t("Media info"), icon: "info", hint: "I", onSelect: opts.onInfo },
    { label: t("Open file…"), icon: "file-plus", hint: "Ctrl+O", onSelect: () => pickMedia("replace") },
    { label: t("Back to library"), icon: "arrow-left", hint: "Backspace", onSelect: act.back },
    { label: t("Settings"), icon: "settings", onSelect: opts.onSettings },
  ];
}

export async function screenshot() {
  try {
    const path = await api.screenshot(true);
    toast(t("Screenshot saved"), { icon: "camera", action: { label: t("Show"), run: () => api.reveal(path) } });
  } catch (e) {
    toast(t("Could not take a screenshot"), { icon: "triangle-alert" });
  }
}
