// Live player state, mirrored from libmpv property events.
import { api, on } from "./api";

export interface Track {
  id: number;
  type: "video" | "audio" | "sub";
  title?: string;
  lang?: string;
  codec?: string;
  default?: boolean;
  forced?: boolean;
  external?: boolean;
  selected?: boolean;
  "external-filename"?: string;
  "demux-channel-count"?: number;
  "demux-samplerate"?: number;
  "demux-w"?: number;
  "demux-h"?: number;
  "audio-channels"?: number;
  albumart?: boolean;
  image?: boolean;
  "hearing-impaired"?: boolean;
}

export interface PlaylistEntry {
  filename: string;
  title?: string;
  current?: boolean;
  playing?: boolean;
  id?: number;
}

export const player = $state({
  p: {} as Record<string, any>,
  mode: false, // controls visible (main window shows the video)
  fullscreen: false,
  pip: false,
  onTop: false,
  loading: false,
  error: null as null | { key: string; path: string },
  resumed: null as null | { pos: number; ask: boolean; at: number },
  posAt: 0, // performance.now() when time-pos last changed
  seq: 0,
});

export const P = {
  get pos(): number {
    return player.p["time-pos"] ?? 0;
  },
  get duration(): number {
    return player.p["duration"] ?? 0;
  },
  get paused(): boolean {
    return player.p["pause"] ?? true;
  },
  get idle(): boolean {
    return player.p["idle-active"] ?? true;
  },
  get tracks(): Track[] {
    return player.p["track-list"] ?? [];
  },
  get playlist(): PlaylistEntry[] {
    return player.p["playlist"] ?? [];
  },
  get playlistPos(): number {
    return player.p["playlist-pos"] ?? -1;
  },
  get title(): string {
    return player.p["media-title"] ?? "";
  },
  get path(): string {
    return player.p["path"] ?? "";
  },
  get hasVideo(): boolean {
    const v = player.p["vid"];
    if (v == null || v === false) return false;
    const t = this.tracks.find((x) => x.type === "video" && x.selected);
    return !!t && !t.albumart && !t.image;
  },
  get albumArt(): boolean {
    return this.tracks.some((x) => x.type === "video" && x.selected && (x.albumart || x.image));
  },
};

type Listener = (ev: any) => void;
const listeners: Listener[] = [];
export function onPlayerEvent(cb: Listener) {
  listeners.push(cb);
}

export async function initPlayer() {
  try {
    player.p = (await api.snapshot()) || {};
  } catch {
    player.p = {};
  }
  try {
    const s = await api.uiState();
    player.mode = s.player;
    player.fullscreen = s.fullscreen;
    player.pip = s.pip;
  } catch {}
  on<{ name: string; value: any }>("player://prop", ({ name, value }) => {
    player.p[name] = value;
    if (name === "time-pos") player.posAt = performance.now();
  });
  on<any>("player://event", (ev) => {
    switch (ev.type) {
      case "start-file":
        player.loading = true;
        player.error = null;
        break;
      case "file-loaded":
        player.loading = false;
        break;
      case "restart":
        player.loading = false;
        break;
      case "error":
        player.loading = false;
        player.error = { key: ev.key, path: ev.path };
        break;
      case "resumed":
        player.resumed = { pos: ev.pos, ask: false, at: Date.now() };
        break;
      case "resume-available":
        player.resumed = { pos: ev.pos, ask: true, at: Date.now() };
        break;
      case "idle":
        player.loading = false;
        break;
    }
    player.seq++;
    for (const l of listeners) l(ev);
  });
  on<{ player: boolean }>("ui://mode", ({ player: m }) => (player.mode = m));
  on<boolean>("ui://fullscreen", (v) => (player.fullscreen = v));
  on<boolean>("ui://pip", (v) => (player.pip = v));
  on<boolean>("ui://ontop", (v) => (player.onTop = v));
}

// ---- actions -----------------------------------------------------------------

export const act = {
  togglePause: () => api.cmd("cycle", "pause"),
  play: () => api.set("pause", false),
  pause: () => api.set("pause", true),
  seekBy: (s: number) => api.cmd("seek", s, "relative+exact"),
  seekTo: (s: number) => api.cmd("seek", Math.max(0, s), "absolute+exact"),
  seekPercent: (pct: number) => api.cmd("seek", pct, "absolute-percent+exact"),
  next: () => api.cmd("playlist-next", "force"),
  prev: () => {
    if (P.pos > 5 || P.playlistPos <= 0) return act.seekTo(0);
    return api.cmd("playlist-prev", "force");
  },
  volumeBy: (d: number) => {
    const v = Math.max(0, Math.min(130, Math.round((player.p.volume ?? 100) + d)));
    return api.set("volume", v);
  },
  setVolume: (v: number) => api.set("volume", Math.max(0, Math.min(130, Math.round(v)))),
  toggleMute: () => api.cmd("cycle", "mute"),
  speedBy: (d: number) => {
    const steps = [0.25, 0.5, 0.75, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3];
    const cur = player.p.speed ?? 1;
    let i = steps.findIndex((s) => s >= cur - 0.001);
    if (i < 0) i = steps.length - 1;
    const ni = Math.max(0, Math.min(steps.length - 1, d > 0 ? (steps[i] > cur + 0.001 ? i : i + 1) : i - 1));
    return api.set("speed", steps[ni]);
  },
  setSpeed: (s: number) => api.set("speed", s),
  frameStep: (back: boolean) => api.cmd(back ? "frame-back-step" : "frame-step"),
  cycleSub: () => api.cmd("cycle", "sid"),
  toggleSubs: () => api.cmd("cycle", "sub-visibility"),
  cycleAudio: () => api.cmd("cycle", "aid"),
  subDelayBy: (d: number) => api.cmd("add", "sub-delay", d),
  fullscreen: (on?: boolean) => api.uiFullscreen(on),
  pip: (on?: boolean) => api.uiPip(on),
  onTop: (on: boolean) => api.uiOnTop(on),
  back: () => api.uiPlayer(false),
  stop: () => api.stop(),
  abLoop: () => api.cmd("ab-loop"),
  loopFile: () => api.set("loop-file", player.p["loop-file"] === "inf" || player.p["loop-file"] === true ? "no" : "inf"),
};
