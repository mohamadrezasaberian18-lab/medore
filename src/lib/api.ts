// Typed bridge to the Rust side.
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Json = any;

export interface Item {
  id: string;
  path: string;
  root: string;
  kind: "movie" | "episode" | "video" | "music";
  title: string;
  year?: number | null;
  show?: string | null;
  season?: number | null;
  episode?: number | null;
  episode_end?: number | null;
  episode_title?: string | null;
  extra: boolean;
  artist?: string | null;
  album?: string | null;
  track?: number | null;
  genre?: string | null;
  duration: number;
  width: number;
  height: number;
  video_codec?: string | null;
  audio_codec?: string | null;
  hdr?: string | null;
  size: number;
  mtime: number;
  added: number;
  thumb?: string | null;
  probed: boolean;
  probe_failed: boolean;
  offline: boolean;
}

export interface Progress {
  path: string;
  title: string;
  pos: number;
  duration: number;
  updated: number;
  finished: boolean;
  plays: number;
  thumb?: string | null;
}

export interface Poster {
  path?: string | null;
  imdb?: string | null;
  title?: string | null;
  year?: number | null;
  fetched: number;
  found: boolean;
}

export const api = {
  appInfo: () => invoke<Json>("app_info"),
  settingsAll: () => invoke<Record<string, Json>>("settings_all"),
  settingsSet: (key: string, value: Json) => invoke<Json>("settings_set", { key, value }),
  settingsReset: (key: string) => invoke<Json>("settings_reset", { key }),

  librarySnapshot: () => invoke<{ items: Item[]; progress: Record<string, Progress>; posters: Record<string, Poster> }>("library_snapshot"),
  libraryStatus: () => invoke<Json>("library_status"),
  libraryRescan: () => invoke<void>("library_rescan"),
  libraryAddFolder: (path: string) => invoke<Json>("library_add_folder", { path }),
  libraryRemoveFolder: (path: string) => invoke<Json>("library_remove_folder", { path }),
  librarySetWatched: (ids: string[], watched: boolean) => invoke<void>("library_set_watched", { ids, watched }),
  libraryForget: (ids: string[]) => invoke<void>("library_forget", { ids }),
  libraryClearHistory: () => invoke<void>("library_clear_history"),

  open: (paths: string[], opts: { start?: number; mode?: "replace" | "append" | "next"; sort?: boolean } = {}) =>
    invoke<number>("player_open", { paths, start: opts.start ?? 0, mode: opts.mode ?? "replace", sort: opts.sort ?? false }),
  cmd: (...args: (string | number)[]) => invoke<void>("player_command", { args: args.map(String) }),
  set: (name: string, value: Json) => invoke<void>("player_set", { name, value }),
  get: (name: string) => invoke<Json>("player_get", { name }),
  snapshot: () => invoke<Record<string, Json>>("player_snapshot"),
  stop: () => invoke<void>("player_stop"),
  addSubtitle: (path: string) => invoke<void>("player_add_subtitle", { path }),
  screenshot: (subtitles: boolean) => invoke<string>("player_screenshot", { subtitles }),
  cancelAutoplay: () => invoke<void>("player_cancel_autoplay"),
  playlistSort: (by: string) => invoke<void>("playlist_sort", { by }),
  playlistMove: (from: number, to: number) => invoke<void>("playlist_move", { from, to }),
  preview: (path: string, t: number, duration: number) => invoke<string | null>("preview_frame", { path, t, duration }),

  uiState: () => invoke<{ player: boolean; fullscreen: boolean; pip: boolean }>("ui_state"),
  uiPlayer: (on: boolean) => invoke<void>("ui_player", { on }),
  uiFullscreen: (on?: boolean) => invoke<boolean>("ui_fullscreen", { on }),
  uiPip: (on?: boolean) => invoke<boolean>("ui_pip", { on }),
  uiOnTop: (on: boolean) => invoke<void>("ui_on_top", { on }),
  uiWindow: (action: "minimize" | "toggle-maximize" | "close" | "focus") => invoke<void>("ui_window", { action }),

  reveal: (path: string) => invoke<void>("reveal", { path }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  fonts: () => invoke<string[]>("fonts"),
  updateCheck: () => invoke<Json>("update_check"),
  updateInstall: () => invoke<void>("update_install"),
};

export function on<T = Json>(event: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(event, (e) => cb(e.payload));
}

export function fileSrc(path?: string | null): string | undefined {
  return path ? convertFileSrc(path) : undefined;
}

export const VIDEO_EXTS = "mkv mp4 m4v mov avi webm ts m2ts mts flv wmv mpg mpeg mpe vob 3gp 3g2 ogv divx xvid rm rmvb asf f4v mxf hevc h264 h265 av1".split(" ");
export const AUDIO_EXTS = "mp3 aac m4a m4b flac alac wav aif aiff ogg oga opus wma ape wv mka dsf dff ac3 eac3 dts mpc tta caf mp2 amr".split(" ");
export const SUB_EXTS = "srt ass ssa vtt sub idx sup smi sami ttml dfxp lrc zip".split(" ");

export function extOf(p: string): string {
  const m = /\.([^.\\/]+)$/.exec(p);
  return m ? m[1].toLowerCase() : "";
}
export const isSubtitle = (p: string) => SUB_EXTS.includes(extOf(p)) && extOf(p) !== "zip";
export const isMedia = (p: string) => VIDEO_EXTS.includes(extOf(p)) || AUDIO_EXTS.includes(extOf(p));
export const isAudio = (p: string) => AUDIO_EXTS.includes(extOf(p));
export const fileName = (p: string) => p.split(/[\\/]/).pop() || p;
export const dirName = (p: string) => p.replace(/[\\/][^\\/]*$/, "");
