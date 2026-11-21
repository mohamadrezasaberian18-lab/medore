// Library data and the views built from it.
import { api, on, dirName, fileName, type Item, type Progress, type Poster } from "./api";
import { compareText } from "./format";
import { settings } from "./settings.svelte";

/** The remembered sort order of a page. */
export function sortState(page: string, def: string, defDesc = false): { by: string; desc: boolean } {
  const all = (settings.v["library.sort"] ?? {}) as Record<string, { by: string; desc: boolean }>;
  return all[page] ?? { by: def, desc: defDesc };
}

export const lib = $state({
  items: [] as Item[],
  progress: {} as Record<string, Progress>,
  posters: {} as Record<string, Poster>,
  status: { state: "idle" } as any,
  loaded: false,
  version: 0,
});

let reloadTimer: any = 0;
async function reload() {
  const s = await api.librarySnapshot();
  lib.items = s.items;
  lib.progress = s.progress;
  lib.posters = s.posters;
  lib.loaded = true;
  lib.version++;
}

export async function initLibrary() {
  await reload();
  try {
    lib.status = await api.libraryStatus();
  } catch {}
  on("library://changed", () => {
    clearTimeout(reloadTimer);
    reloadTimer = setTimeout(reload, 250);
  });
  on("library://status", (s) => (lib.status = s));
}

// ---- helpers ------------------------------------------------------------------

export function progressOf(it: { id: string }): Progress | undefined {
  return lib.progress[it.id];
}

/** 0..1 watched share for a progress bar, or -1 when not started. */
export function fraction(it: { id: string; duration?: number }): number {
  const p = lib.progress[it.id];
  if (!p) return -1;
  if (p.finished) return 1;
  const d = p.duration || it.duration || 0;
  return d > 0 && p.pos > 0 ? Math.min(1, p.pos / d) : -1;
}

export function watched(it: { id: string }): boolean {
  return !!lib.progress[it.id]?.finished;
}

export function posterKeyMovie(title: string, year?: number | null) {
  return `movie:${title.toLowerCase()}:${year ?? ""}`;
}
export function posterKeyShow(name: string) {
  return `show:${name.toLowerCase()}`;
}
export function posterFor(key: string): string | undefined {
  const p = lib.posters[key];
  return p?.found && p.path ? p.path : undefined;
}

export interface Show {
  key: string;
  name: string;
  episodes: Item[];
  seasons: number[];
  poster?: string;
  thumb?: string;
  added: number;
  year?: number | null;
  lastWatched: number;
  unwatched: number;
}

export function sortEpisodes(a: Item, b: Item): number {
  return (a.season ?? 1) - (b.season ?? 1) || (a.episode ?? 0) - (b.episode ?? 0) || compareText(fileName(a.path), fileName(b.path));
}

export function visibleItems(): Item[] {
  return lib.items.filter((i) => !i.extra);
}

export function movies(): Item[] {
  return visibleItems().filter((i) => i.kind === "movie");
}

export function shows(): Show[] {
  const map = new Map<string, Show>();
  for (const it of visibleItems()) {
    if (it.kind !== "episode") continue;
    const name = it.show || it.title;
    const key = name.toLowerCase();
    let s = map.get(key);
    if (!s) {
      s = { key, name, episodes: [], seasons: [], added: 0, lastWatched: 0, unwatched: 0 };
      map.set(key, s);
    }
    s.episodes.push(it);
    s.added = Math.max(s.added, it.added);
    if (it.year && !s.year) s.year = it.year;
    const p = lib.progress[it.id];
    if (p) s.lastWatched = Math.max(s.lastWatched, p.updated);
    if (!p?.finished) s.unwatched++;
  }
  for (const s of map.values()) {
    s.episodes.sort(sortEpisodes);
    s.seasons = [...new Set(s.episodes.map((e) => e.season ?? 1))].sort((a, b) => a - b);
    s.poster = posterFor(posterKeyShow(s.name));
    s.thumb = s.episodes.find((e) => e.thumb)?.thumb ?? undefined;
  }
  return [...map.values()];
}

/** The episode to play next in a show: first unfinished after the last watched. */
export function nextUp(s: Show): Item | undefined {
  let lastIdx = -1;
  let lastTime = 0;
  s.episodes.forEach((e, i) => {
    const p = lib.progress[e.id];
    if (p && p.updated >= lastTime) {
      lastTime = p.updated;
      lastIdx = i;
    }
  });
  if (lastIdx >= 0) {
    const last = s.episodes[lastIdx];
    if (!lib.progress[last.id]?.finished) return last;
    return s.episodes.slice(lastIdx + 1).find((e) => !lib.progress[e.id]?.finished);
  }
  return s.episodes.find((e) => !lib.progress[e.id]?.finished) ?? s.episodes[0];
}

export function music(): Item[] {
  return visibleItems().filter((i) => i.kind === "music");
}

export function videos(): Item[] {
  return visibleItems().filter((i) => i.kind === "video");
}

export interface Album {
  key: string;
  name: string;
  artist: string;
  tracks: Item[];
  art?: string;
  year?: number | null;
  added: number;
}

export function albums(): Album[] {
  const map = new Map<string, Album>();
  for (const it of music()) {
    const name = it.album || dirName(it.path).split(/[\\/]/).pop() || "";
    const artist = it.artist || "";
    const key = (name + "|" + (it.album ? artist : dirName(it.path))).toLowerCase();
    let a = map.get(key);
    if (!a) {
      a = { key, name, artist, tracks: [], added: 0 };
      map.set(key, a);
    }
    a.tracks.push(it);
    a.added = Math.max(a.added, it.added);
    if (!a.art && it.thumb) a.art = it.thumb;
    if (!a.year && it.year) a.year = it.year;
  }
  for (const a of map.values()) a.tracks.sort((x, y) => (x.track ?? 999) - (y.track ?? 999) || compareText(fileName(x.path), fileName(y.path)));
  return [...map.values()];
}

/** Continue watching: started, not finished, newest first (also files outside the library). */
export function continueWatching(): { id: string; path: string; title: string; pos: number; duration: number; thumb?: string | null; item?: Item; updated: number }[] {
  const byId = new Map(lib.items.map((i) => [i.id, i]));
  return Object.entries(lib.progress)
    .filter(([, p]) => !p.finished && p.pos > 15 && (p.duration <= 0 || p.pos < p.duration - 20))
    .map(([id, p]) => ({ id, path: p.path, title: byId.get(id)?.title ?? p.title, pos: p.pos, duration: p.duration, thumb: byId.get(id)?.thumb ?? p.thumb, item: byId.get(id), updated: p.updated }))
    .sort((a, b) => b.updated - a.updated)
    .slice(0, 20);
}

export function recentlyAdded(limit = 24): Item[] {
  return visibleItems()
    .filter((i) => i.kind === "movie" || i.kind === "episode")
    .sort((a, b) => b.added - a.added)
    .slice(0, limit);
}

export function recentFiles(limit = 20) {
  const byId = new Map(lib.items.map((i) => [i.id, i]));
  return Object.entries(lib.progress)
    .map(([id, p]) => ({ id, ...p, item: byId.get(id) }))
    .sort((a, b) => b.updated - a.updated)
    .slice(0, limit);
}

// ---- sorting ----------------------------------------------------------------------

export type SortKey = "title" | "added" | "year" | "duration" | "watched" | "size" | "name";

export function sortItems<T extends Item>(list: T[], key: SortKey, desc: boolean): T[] {
  const prog = lib.progress;
  const cmp: Record<SortKey, (a: T, b: T) => number> = {
    title: (a, b) => compareText(a.title, b.title) || (a.year ?? 0) - (b.year ?? 0),
    name: (a, b) => compareText(fileName(a.path), fileName(b.path)),
    added: (a, b) => a.added - b.added,
    year: (a, b) => (a.year ?? 0) - (b.year ?? 0) || compareText(a.title, b.title),
    duration: (a, b) => a.duration - b.duration,
    watched: (a, b) => (prog[a.id]?.updated ?? 0) - (prog[b.id]?.updated ?? 0),
    size: (a, b) => a.size - b.size,
  };
  const f = cmp[key] ?? cmp.title;
  // stable: ties keep a natural file-name order
  return [...list].sort((a, b) => {
    const r = f(a, b) || compareText(a.path, b.path);
    return desc ? -r : r;
  });
}

export function search(q: string): Item[] {
  const words = q.toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return [];
  return visibleItems().filter((i) => {
    const hay = [i.title, i.show, i.episode_title, i.artist, i.album, fileName(i.path), i.year].filter(Boolean).join(" ").toLowerCase();
    return words.every((w) => hay.includes(w));
  });
}
