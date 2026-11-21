<script lang="ts">
  // The queue: what plays next. Sort it (natural name order, date, reverse,
  // shuffle), drag to reorder, remove, or jump to any entry.
  import Icon from "../components/Icon.svelte";
  import { player, P } from "../lib/player.svelte";
  import { api, fileName, fileSrc } from "../lib/api";
  import { lib } from "../lib/library.svelte";
  import { t, num } from "../lib/i18n.svelte";
  import { openMenu } from "../lib/ui.svelte";
  import { duration as fmtDuration } from "../lib/format";

  let { onclose }: { onclose: () => void } = $props();

  const byPath = $derived(new Map(lib.items.map((i) => [i.path.toLowerCase(), i])));
  const entries = $derived(
    P.playlist.map((e, i) => {
      const it = byPath.get(e.filename.toLowerCase());
      return {
        i,
        name: it ? (it.kind === "episode" && it.episode != null ? `${it.show ?? it.title} · ${it.episode_title ?? ""}`.replace(/ · $/, "") : it.title) : e.title || fileName(e.filename).replace(/\.[^.]+$/, ""),
        tag: it?.kind === "episode" && it.episode != null ? `S${String(it.season ?? 1).padStart(2, "0")}E${String(it.episode).padStart(2, "0")}` : "",
        file: fileName(e.filename),
        thumb: it?.thumb,
        dur: it?.duration ?? 0,
        current: !!e.current || i === P.playlistPos,
      };
    }),
  );

  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);
  let list: HTMLDivElement;

  function sortMenu(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    openMenu(
      [
        { header: t("Sort queue") },
        { label: t("Name (A to Z)"), icon: "arrow-down-wide-narrow", onSelect: () => api.playlistSort("name") },
        { label: t("Name (Z to A)"), icon: "arrow-up-narrow-wide", onSelect: () => api.playlistSort("name-desc") },
        { label: t("Date modified (oldest first)"), icon: "clock", onSelect: () => api.playlistSort("date") },
        { label: t("Date modified (newest first)"), icon: "history", onSelect: () => api.playlistSort("date-desc") },
        { label: t("Reverse order"), icon: "arrow-up-down", onSelect: () => api.playlistSort("reverse") },
        { label: t("Shuffle"), icon: "shuffle", onSelect: () => api.playlistSort("shuffle") },
        { separator: true },
        { label: t("Remove everything except what is playing"), icon: "trash-2", onSelect: () => api.cmd("playlist-clear") },
      ],
      { anchor: r, align: "end" },
    );
  }

  function rowMenu(e: MouseEvent, i: number) {
    e.preventDefault();
    e.stopPropagation();
    openMenu(
      [
        { label: t("Play"), icon: "play", onSelect: () => api.set("playlist-pos", i) },
        { label: t("Play next"), icon: "list-start", disabled: i === P.playlistPos || i === P.playlistPos + 1, onSelect: () => api.playlistMove(i, i < P.playlistPos ? P.playlistPos : P.playlistPos + 1) },
        { label: t("Move to the end"), icon: "chevron-down", onSelect: () => api.playlistMove(i, P.playlist.length - 1) },
        { label: t("Show in folder"), icon: "folder-open", onSelect: () => api.reveal(P.playlist[i].filename) },
        { separator: true },
        { label: t("Remove from queue"), icon: "x", danger: true, onSelect: () => api.cmd("playlist-remove", i) },
      ],
      { x: e.clientX, y: e.clientY },
    );
  }

  function startDrag(e: PointerEvent, i: number) {
    e.preventDefault();
    dragFrom = i;
    dragOver = i;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function dragMove(e: PointerEvent) {
    if (dragFrom == null) return;
    const rows = [...list.querySelectorAll<HTMLElement>("[data-row]")];
    let over = rows.length - 1;
    for (let k = 0; k < rows.length; k++) {
      const r = rows[k].getBoundingClientRect();
      if (e.clientY < r.top + r.height / 2) {
        over = k;
        break;
      }
    }
    dragOver = over;
    const lr = list.getBoundingClientRect();
    if (e.clientY < lr.top + 30) list.scrollTop -= 12;
    if (e.clientY > lr.bottom - 30) list.scrollTop += 12;
  }
  function endDrag() {
    if (dragFrom != null && dragOver != null && dragFrom !== dragOver) api.playlistMove(dragFrom, dragOver);
    dragFrom = null;
    dragOver = null;
  }

  $effect(() => {
    // keep the playing entry in view when the panel opens or the item changes
    void P.playlistPos;
    queueMicrotask(() => list?.querySelector(".current")?.scrollIntoView({ block: "nearest" }));
  });
</script>

<aside class="panel" role="dialog" aria-label={t("Queue")}>
  <header>
    <div>
      <div class="title">{t("Queue")}</div>
      <div class="sub muted">{entries.length ? t("{n} of {m}", { n: Math.max(1, P.playlistPos + 1), m: entries.length }) : t("Nothing queued")}</div>
    </div>
    <div class="tools">
      <button class="icon-btn" title={t("Sort and arrange")} onclick={sortMenu}><Icon name="arrow-up-down" size={18} /></button>
      <button class="icon-btn" title={t("Close")} onclick={onclose}><Icon name="x" size={18} /></button>
    </div>
  </header>
  <div class="list" bind:this={list} onpointermove={dragMove} onpointerup={endDrag} role="list">
    {#each entries as e (e.i + ":" + e.file)}
      <div
        data-row
        role="listitem"
        class="row"
        class:current={e.current}
        class:dragging={dragFrom === e.i}
        class:drop-before={dragOver === e.i && dragFrom != null && dragFrom > e.i}
        class:drop-after={dragOver === e.i && dragFrom != null && dragFrom < e.i}
        ondblclick={() => api.set("playlist-pos", e.i)}
        oncontextmenu={(ev) => rowMenu(ev, e.i)}
      >
        <button class="grip" aria-label={t("Drag to reorder")} onpointerdown={(ev) => startDrag(ev, e.i)}><Icon name="grip-vertical" size={16} /></button>
        <button class="main" onclick={() => api.set("playlist-pos", e.i)}>
          <div class="thumb">
            {#if e.thumb}<img src={fileSrc(e.thumb)} alt="" loading="lazy" />{/if}
            {#if e.current}<div class="now"><Icon name={P.paused ? "pause" : "audio-lines"} size={16} /></div>{/if}
          </div>
          <div class="text">
            <div class="name ellipsis">{e.name}</div>
            <div class="meta muted ellipsis num">{[e.tag, e.dur ? fmtDuration(e.dur) : "", e.tag || e.dur ? "" : e.file].filter(Boolean).join(" · ")}</div>
          </div>
        </button>
        <button class="rm icon-btn" title={t("Remove from queue")} onclick={() => api.cmd("playlist-remove", e.i)}><Icon name="x" size={16} /></button>
      </div>
    {/each}
  </div>
</aside>

<style>
  .panel {
    position: absolute;
    top: 0;
    bottom: 0;
    inset-inline-end: 0;
    width: 360px;
    max-width: 92vw;
    display: flex;
    flex-direction: column;
    background: var(--glass);
    backdrop-filter: blur(28px) saturate(1.2);
    border-inline-start: 1px solid var(--hairline);
    animation: slide-in var(--t-slow) var(--ease);
  }
  @keyframes slide-in {
    from {
      transform: translateX(24px);
      opacity: 0;
    }
  }
  :global([dir="rtl"]) .panel {
    animation-name: slide-in-rtl;
  }
  @keyframes slide-in-rtl {
    from {
      transform: translateX(-24px);
      opacity: 0;
    }
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 18px 12px 10px 18px;
  }
  .title {
    font-size: 17px;
    font-weight: 600;
  }
  .sub {
    font-size: 12.5px;
  }
  .tools {
    display: flex;
    gap: 2px;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 8px 16px;
  }
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 4px;
    border-radius: var(--r-m);
    padding: 4px 4px 4px 0;
  }
  .row:hover {
    background: rgba(255, 255, 255, 0.06);
  }
  .row.current {
    background: rgba(255, 255, 255, 0.1);
  }
  .row.dragging {
    opacity: 0.45;
  }
  .row.drop-before::before,
  .row.drop-after::after {
    content: "";
    position: absolute;
    left: 8px;
    right: 8px;
    height: 2px;
    background: var(--accent-hover);
    border-radius: 2px;
  }
  .row.drop-before::before {
    top: -1px;
  }
  .row.drop-after::after {
    bottom: -1px;
  }
  .grip {
    width: 22px;
    height: 40px;
    display: grid;
    place-items: center;
    color: var(--text-disabled);
    cursor: grab;
    opacity: 0;
  }
  .row:hover .grip {
    opacity: 1;
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: start;
  }
  .thumb {
    position: relative;
    width: 72px;
    height: 41px;
    border-radius: 6px;
    background: var(--bg-elevated);
    overflow: hidden;
    flex: none;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .now {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.55);
  }
  .text {
    min-width: 0;
  }
  .name {
    font-size: 13.5px;
    font-weight: 500;
  }
  .current .name {
    font-weight: 600;
  }
  .meta {
    font-size: 12px;
  }
  .rm {
    width: 30px;
    height: 30px;
    opacity: 0;
  }
  .row:hover .rm {
    opacity: 1;
  }
</style>
