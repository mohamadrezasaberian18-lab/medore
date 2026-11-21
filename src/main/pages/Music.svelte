<script lang="ts">
  import Icon from "../../components/Icon.svelte";
  import PageHeader from "../PageHeader.svelte";
  import Segmented from "../../components/Segmented.svelte";
  import Empty from "../Empty.svelte";
  import { albums, music, sortItems, sortState, type SortKey } from "../../lib/library.svelte";
  import { playList, itemMenu } from "../../lib/play";
  import { fileSrc } from "../../lib/api";
  import { t, num, tn } from "../../lib/i18n.svelte";
  import { clock, compareText } from "../../lib/format";
  import { openMenu } from "../../lib/ui.svelte";
  import { P } from "../../lib/player.svelte";

  let view = $state<"albums" | "songs">("albums");
  let openAlbum = $state<string | null>(null);
  const st = $derived(sortState("music", "title"));
  const sorts = $derived([
    { key: "title", label: t("Title") },
    { key: "added", label: t("Date added"), desc: true },
    { key: "duration", label: t("Length") },
    { key: "watched", label: t("Last played"), desc: true },
  ]);
  const albumList = $derived([...albums()].sort((a, b) => (st.by === "added" ? b.added - a.added : compareText(a.name, b.name))));
  const songs = $derived(sortItems(music(), st.by as SortKey, st.desc));
  const album = $derived(openAlbum ? albumList.find((a) => a.key === openAlbum) : undefined);
</script>

<PageHeader title={t("Music")} count={tn("{n} song", "{n} songs", music().length)} page="music" {sorts} defaultSort="title">
  <Segmented value={view} onchange={(v) => ((view = v as any), (openAlbum = null))} options={[{ value: "albums", label: t("Albums") }, { value: "songs", label: t("Songs") }]} />
</PageHeader>

{#if !music().length}
  <Empty icon="music" title={t("No music yet")} text={t("Songs in your library folders appear here, grouped by album.")} />
{:else if view === "albums" && !album}
  <div class="albums">
    {#each albumList as a (a.key)}
      <button class="album" onclick={() => (openAlbum = a.key)} oncontextmenu={(e) => (e.preventDefault(), openMenu([{ label: t("Play album"), icon: "play", onSelect: () => playList(a.tracks, 0) }], { x: e.clientX, y: e.clientY }))}>
        <div class="art">
          {#if a.art}<img src={fileSrc(a.art)} alt="" loading="lazy" />{:else}<Icon name="music" size={34} stroke={1.3} />{/if}
          <span class="play" role="button" tabindex="-1" onclick={(e) => (e.stopPropagation(), playList(a.tracks, 0))} onkeydown={() => {}}><Icon name="play" size={20} fill /></span>
        </div>
        <div class="an ellipsis"><bdi>{a.name || t("Unknown album")}</bdi></div>
        <div class="aa ellipsis muted"><bdi>{a.artist || tn("{n} song", "{n} songs", a.tracks.length)}</bdi></div>
      </button>
    {/each}
  </div>
{:else}
  {@const list = album ? album.tracks : songs}
  {#if album}
    <div class="ahead">
      <button class="btn ghost sm" onclick={() => (openAlbum = null)}><Icon name="arrow-left" size={16} />{t("Albums")}</button>
      <div class="ahero">
        <div class="art big">{#if album.art}<img src={fileSrc(album.art)} alt="" />{:else}<Icon name="music" size={40} />{/if}</div>
        <div>
          <div class="overline">{t("Album")}</div>
          <h2><bdi>{album.name || t("Unknown album")}</bdi></h2>
          <div class="muted num">{[album.artist, album.year ? num(album.year) : "", tn("{n} song", "{n} songs", album.tracks.length)].filter(Boolean).join(" · ")}</div>
          <button class="btn primary" style="margin-top:14px" onclick={() => playList(album.tracks, 0)}><Icon name="play" size={16} fill />{t("Play")}</button>
        </div>
      </div>
    </div>
  {/if}
  <div class="songs">
    {#each list as s, i (s.id)}
      <div class="song" class:now={P.path.toLowerCase() === s.path.toLowerCase()} role="button" tabindex="0" ondblclick={() => playList(list, i)} onkeydown={(e) => e.key === "Enter" && playList(list, i)} oncontextmenu={(e) => (e.preventDefault(), openMenu(itemMenu(s), { x: e.clientX, y: e.clientY }))}>
        <button class="idx num" onclick={() => playList(list, i)} aria-label={t("Play")}>
          <span class="n">{num(album ? (s.track ?? i + 1) : i + 1)}</span>
          <span class="p"><Icon name="play" size={14} fill /></span>
        </button>
        <div class="st ellipsis"><bdi>{s.title}</bdi></div>
        <div class="sa ellipsis muted"><bdi>{s.artist ?? ""}</bdi></div>
        <div class="sb ellipsis muted"><bdi>{album ? "" : s.album ?? ""}</bdi></div>
        <div class="sd muted num">{clock(s.duration)}</div>
      </div>
    {/each}
  </div>
{/if}

<style>
  .albums {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 24px 20px;
    padding: 18px 40px 0;
  }
  .album {
    text-align: start;
    min-width: 0;
  }
  .art {
    position: relative;
    aspect-ratio: 1;
    border-radius: var(--r-l);
    overflow: hidden;
    background: var(--bg-surface);
    display: grid;
    place-items: center;
    color: var(--text-disabled);
    box-shadow: 0 0 0 1px var(--hairline) inset;
    transition: transform var(--t) var(--ease), box-shadow var(--t) var(--ease);
  }
  .album:hover .art {
    transform: translateY(-3px);
    box-shadow: var(--shadow-card);
  }
  .art img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .art.big {
    width: 180px;
    flex: none;
  }
  .play {
    position: absolute;
    inset-inline-end: 10px;
    bottom: 10px;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--accent-hover);
    color: #000;
    opacity: 0;
    transform: translateY(6px);
    transition: opacity var(--t), transform var(--t) var(--ease);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.5);
  }
  .album:hover .play {
    opacity: 1;
    transform: none;
  }
  .an {
    margin-top: 9px;
    font-weight: 600;
  }
  .aa {
    font-size: 12.5px;
  }
  .ahead {
    padding: 8px 34px 0;
  }
  .ahero {
    display: flex;
    gap: 24px;
    align-items: flex-end;
    padding: 14px 6px 10px;
  }
  .ahero h2 {
    margin: 4px 0;
    font-size: 30px;
    font-family: var(--font-display);
  }
  .songs {
    padding: 12px 28px 0;
  }
  .song {
    display: grid;
    grid-template-columns: 40px minmax(0, 2fr) minmax(0, 1.2fr) minmax(0, 1.2fr) 64px;
    align-items: center;
    gap: 12px;
    height: 48px;
    padding: 0 12px;
    border-radius: var(--r-m);
  }
  .song:hover {
    background: var(--bg-surface);
  }
  .song.now .st {
    font-weight: 700;
  }
  .idx {
    position: relative;
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    color: var(--text-secondary);
  }
  .idx .p {
    position: absolute;
    opacity: 0;
    color: var(--text-primary);
  }
  .song:hover .idx .n {
    opacity: 0;
  }
  .song:hover .idx .p {
    opacity: 1;
  }
  .sd {
    text-align: end;
    font-size: 13px;
  }
</style>
