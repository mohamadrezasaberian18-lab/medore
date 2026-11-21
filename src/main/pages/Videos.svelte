<script lang="ts">
  // Folders: browse library folders exactly as they are on disk, with
  // natural ordering (Episode 2 before Episode 10).
  import Icon from "../../components/Icon.svelte";
  import PageHeader from "../PageHeader.svelte";
  import WideCard from "../WideCard.svelte";
  import Empty from "../Empty.svelte";
  import Segmented from "../../components/Segmented.svelte";
  import { lib, visibleItems, videos, sortItems, sortState, fraction, watched, type SortKey } from "../../lib/library.svelte";
  import { settings } from "../../lib/settings.svelte";
  import { playList, itemMenu } from "../../lib/play";
  import { api, fileName, fileSrc, type Item } from "../../lib/api";
  import { t, num, tn } from "../../lib/i18n.svelte";
  import { duration, compareText } from "../../lib/format";
  import { openMenu } from "../../lib/ui.svelte";
  import { go } from "../../lib/nav.svelte";

  let { path = "" }: { path?: string } = $props();
  let mode = $state<"folders" | "all">("folders");

  const roots = $derived((settings.v["library.folders"] ?? []) as string[]);
  const norm = (p: string) => p.replace(/[\\/]+$/, "").toLowerCase();
  const sorts = $derived([
    { key: "name", label: t("Name") },
    { key: "added", label: t("Date added"), desc: true },
    { key: "duration", label: t("Length") },
    { key: "size", label: t("File size"), desc: true },
    { key: "watched", label: t("Last watched"), desc: true },
  ]);
  const st = $derived(sortState("videos", "name"));

  // everything (videos, movies, episodes, music) inside the current folder
  const here = $derived.by(() => {
    if (!path) return { folders: [] as { path: string; name: string; count: number; thumb?: string | null }[], files: [] as Item[] };
    const base = norm(path) + "\\";
    const folders = new Map<string, { path: string; name: string; count: number; thumb?: string | null }>();
    const files: Item[] = [];
    for (const it of visibleItems()) {
      const p = it.path.toLowerCase();
      if (!p.startsWith(base)) continue;
      const rest = it.path.slice(base.length);
      const cut = rest.search(/[\\/]/);
      if (cut < 0) files.push(it);
      else {
        const name = rest.slice(0, cut);
        const fp = it.path.slice(0, base.length + cut);
        const f = folders.get(fp.toLowerCase()) ?? { path: fp, name, count: 0, thumb: null };
        f.count++;
        if (!f.thumb && it.thumb) f.thumb = it.thumb;
        folders.set(fp.toLowerCase(), f);
      }
    }
    return { folders: [...folders.values()].sort((a, b) => compareText(a.name, b.name)), files: sortItems(files, st.by as SortKey, st.desc) };
  });

  const rootInfo = $derived(
    roots.map((r) => {
      const base = norm(r) + "\\";
      const inside = lib.items.filter((i) => i.path.toLowerCase().startsWith(base) && !i.extra);
      return { path: r, name: fileName(r.replace(/[\\/]+$/, "")) || r, count: inside.length, thumb: inside.find((i) => i.thumb)?.thumb };
    }),
  );
  const all = $derived(sortItems(videos(), st.by as SortKey, st.desc));
  const crumbs = $derived.by(() => {
    if (!path) return [];
    const root = roots.find((r) => norm(path).startsWith(norm(r)));
    if (!root) return [{ name: fileName(path), path }];
    const out = [{ name: fileName(root.replace(/[\\/]+$/, "")) || root, path: root }];
    const rest = path.slice(root.replace(/[\\/]+$/, "").length).split(/[\\/]/).filter(Boolean);
    let acc = root.replace(/[\\/]+$/, "");
    for (const part of rest) {
      acc += "\\" + part;
      out.push({ name: part, path: acc });
    }
    return out;
  });

  function playFrom(list: Item[], it: Item) {
    playList(list, list.indexOf(it));
  }
  function sub(it: Item) {
    return [duration(it.duration), it.kind !== "video" ? t(it.kind === "movie" ? "Movie" : it.kind === "episode" ? "Episode" : "Music") : ""].filter(Boolean).join(" · ");
  }
</script>

<PageHeader title={path ? crumbs.at(-1)?.name ?? t("Videos") : t("Videos")} count={path ? tn("{n} file", "{n} files", here.files.length) : ""} page="videos" {sorts} defaultSort="name">
  {#if !path}
    <Segmented value={mode} onchange={(v) => (mode = v as any)} options={[{ value: "folders", label: t("Folders") }, { value: "all", label: t("All videos") }]} />
  {/if}
</PageHeader>

{#if path}
  <div class="crumbs">
    <button class="crumb" onclick={() => go("videos", "")}><Icon name="hard-drive" size={15} />{t("Library")}</button>
    {#each crumbs as c, i}
      <Icon name="chevron-right" size={14} />
      <button class="crumb" class:last={i === crumbs.length - 1} onclick={() => go("videos", c.path)}>{c.name}</button>
    {/each}
  </div>
{/if}

{#if !path && mode === "folders"}
  {#if !roots.length}
    <Empty icon="folder" title={t("No folders yet")} text={t("Add the folders where your videos are and browse them here.")} />
  {:else}
    <div class="grid">
      {#each rootInfo as r (r.path)}
        <button class="folder" onclick={() => go("videos", r.path)}>
          <div class="fthumb">{#if r.thumb}<img src={fileSrc(r.thumb)} alt="" />{/if}<Icon name="folder" size={30} stroke={1.4} /></div>
          <div class="fname ellipsis"><bdi>{r.name}</bdi></div>
          <div class="fmeta muted ellipsis" dir="ltr">{r.path}</div>
          <div class="fmeta muted num">{tn("{n} item", "{n} items", r.count)}</div>
        </button>
      {/each}
    </div>
  {/if}
{:else if !path && mode === "all"}
  {#if !all.length}
    <Empty icon="folder" title={t("No personal videos")} text={t("Videos that are not movies or episodes appear here.")} />
  {:else}
    <div class="grid wide">
      {#each all as it (it.id)}
        <WideCard title={it.title} sub={sub(it)} thumb={it.thumb} progress={fraction(it)} watched={watched(it)} corner={duration(it.duration) ? "" : ""} onplay={() => playFrom(all, it)} onmenu={(e) => openMenu(itemMenu(it), { x: e.clientX, y: e.clientY })} />
      {/each}
    </div>
  {/if}
{:else}
  {#if here.folders.length}
    <div class="sub-folders">
      {#each here.folders as f (f.path)}
        <button class="chip" onclick={() => go("videos", f.path)}>
          <Icon name="folder" size={18} />
          <span class="ellipsis"><bdi>{f.name}</bdi></span>
          <span class="muted num">{num(f.count)}</span>
        </button>
      {/each}
    </div>
  {/if}
  {#if here.files.length}
    <div class="grid wide">
      {#each here.files as it (it.id)}
        <WideCard title={fileName(it.path).replace(/\.[^.]+$/, "")} sub={sub(it)} thumb={it.thumb} progress={fraction(it)} watched={watched(it)} onplay={() => playFrom(here.files, it)} onmenu={(e) => openMenu(itemMenu(it), { x: e.clientX, y: e.clientY })} />
      {/each}
    </div>
  {:else if !here.folders.length}
    <Empty icon="folder-open" title={t("This folder is empty")} text={t("No video or audio files were found here.")} folderAction={false} />
  {/if}
{/if}

<style>
  .crumbs {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px;
    padding: 0 34px 8px;
    color: var(--text-disabled);
  }
  .crumb {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 8px;
    border-radius: 7px;
    color: var(--text-secondary);
    font-size: 13px;
  }
  .crumb:hover {
    background: var(--bg-surface);
    color: var(--text-primary);
  }
  .crumb.last {
    color: var(--text-primary);
    font-weight: 600;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 22px 20px;
    padding: 16px 40px 0;
  }
  .grid.wide {
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  }
  .folder {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 12px;
    border-radius: var(--r-l);
    background: var(--bg-surface);
    border: 1px solid var(--hairline);
    text-align: start;
    transition: background var(--t-fast), transform var(--t) var(--ease);
  }
  .folder:hover {
    background: var(--bg-elevated);
    transform: translateY(-2px);
  }
  .fthumb {
    position: relative;
    width: 100%;
    aspect-ratio: 16/9;
    border-radius: 10px;
    overflow: hidden;
    background: var(--bg-elevated);
    display: grid;
    place-items: center;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }
  .fthumb img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0.55;
  }
  .fthumb :global(svg) {
    position: relative;
  }
  .fname {
    font-weight: 600;
    max-width: 100%;
  }
  .fmeta {
    font-size: 12px;
    max-width: 100%;
  }
  .sub-folders {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 10px 40px 4px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    max-width: 320px;
    height: 38px;
    padding: 0 14px;
    border-radius: 19px;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
  }
  .chip:hover {
    background: var(--bg-elevated);
  }
</style>
