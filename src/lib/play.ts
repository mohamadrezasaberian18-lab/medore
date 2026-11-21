// Playing things from the library, and the menus on library items.
import { api, type Item } from "./api";
import { lib, shows, sortEpisodes, albums, progressOf } from "./library.svelte";
import { t } from "./i18n.svelte";
import { toast, type MenuItem } from "./ui.svelte";

/** Play an item; episodes queue their whole show in order, songs their album. */
export async function playItem(it: Item, fromStart = false) {
  if (fromStart && progressOf(it)) await api.libraryForget([it.id]);
  if (it.kind === "episode") {
    const s = shows().find((x) => x.key === (it.show || it.title).toLowerCase());
    if (s && s.episodes.length > 1) {
      const eps = [...s.episodes].sort(sortEpisodes);
      return api.open(
        eps.map((e) => e.path),
        { start: Math.max(0, eps.findIndex((e) => e.id === it.id)) },
      );
    }
  }
  if (it.kind === "music") {
    const a = albums().find((x) => x.tracks.some((tr) => tr.id === it.id));
    if (a && a.tracks.length > 1) {
      return api.open(
        a.tracks.map((x) => x.path),
        { start: Math.max(0, a.tracks.findIndex((x) => x.id === it.id)) },
      );
    }
  }
  return api.open([it.path]);
}

export function playPath(path: string) {
  const it = lib.items.find((i) => i.path.toLowerCase() === path.toLowerCase());
  return it ? playItem(it) : api.open([path]);
}

export function playList(items: Item[], start = 0) {
  return api.open(
    items.map((i) => i.path),
    { start },
  );
}

export function itemMenu(it: Item, extra: MenuItem[] = []): MenuItem[] {
  const p = progressOf(it);
  const watched = !!p?.finished;
  const started = !!p && !p.finished && p.pos > 15;
  return [
    { label: started ? t("Resume") : t("Play"), icon: "play", onSelect: () => playItem(it) },
    ...(started ? [{ label: t("Play from the beginning"), icon: "undo-2", onSelect: () => playItem(it, true) }] : []),
    { label: t("Play next"), icon: "list-start", onSelect: () => (api.open([it.path], { mode: "next" }), toast(t("Will play next"), { icon: "list-start" })) },
    { label: t("Add to queue"), icon: "list-plus", onSelect: () => (api.open([it.path], { mode: "append" }), toast(t("Added to queue"), { icon: "list-plus" })) },
    { separator: true },
    { label: watched ? t("Mark as unwatched") : t("Mark as watched"), icon: watched ? "eye-off" : "circle-check", onSelect: () => api.librarySetWatched([it.id], !watched) },
    ...(p ? [{ label: t("Remove from history"), icon: "history", onSelect: () => api.libraryForget([it.id]) }] : []),
    ...extra,
    { separator: true },
    { label: t("Show in folder"), icon: "folder-open", onSelect: () => api.reveal(it.path) },
  ];
}
