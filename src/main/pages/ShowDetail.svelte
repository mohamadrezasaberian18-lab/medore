<script lang="ts">
  import Icon from "../../components/Icon.svelte";
  import Segmented from "../../components/Segmented.svelte";
  import { shows, nextUp, fraction, watched, lib } from "../../lib/library.svelte";
  import { playItem, itemMenu } from "../../lib/play";
  import { api, fileSrc, fileName } from "../../lib/api";
  import { t, num, tn } from "../../lib/i18n.svelte";
  import { duration, episodeTag } from "../../lib/format";
  import { openMenu } from "../../lib/ui.svelte";
  import { back } from "../../lib/nav.svelte";

  let { key }: { key: string } = $props();
  const show = $derived(shows().find((s) => s.key === key));
  const nu = $derived(show ? nextUp(show) : undefined);
  let season = $state<number | null>(null);
  const seasonNow = $derived(season ?? (nu?.season ?? show?.seasons[0] ?? 1));
  const eps = $derived(show ? show.episodes.filter((e) => (e.season ?? 1) === seasonNow) : []);
  const backdrop = $derived(show?.poster || show?.thumb);
  const started = $derived(nu ? (lib.progress[nu.id]?.pos ?? 0) > 15 && !lib.progress[nu.id]?.finished : false);
</script>

{#if !show}
  <div class="missing">
    <p class="muted">{t("This show is no longer in your library.")}</p>
    <button class="btn secondary" onclick={back}>{t("Back")}</button>
  </div>
{:else}
  <div class="hero">
    {#if backdrop}<div class="bg" style:background-image={`url("${fileSrc(backdrop)}")`}></div>{/if}
    <div class="fade"></div>
    <div class="inner">
      <div class="poster">
        {#if show.poster}<img src={fileSrc(show.poster)} alt="" />{:else if show.thumb}<img class="frame" src={fileSrc(show.thumb)} alt="" />{:else}<Icon name="tv" size={40} />{/if}
      </div>
      <div class="info">
        <div class="overline">{t("TV show")}</div>
        <h1><bdi>{show.name}</bdi></h1>
        <div class="facts muted num">
          {[show.year ? num(show.year) : "", tn("{n} season", "{n} seasons", show.seasons.length), tn("{n} episode", "{n} episodes", show.episodes.length), show.unwatched ? tn("{n} unwatched", "{n} unwatched", show.unwatched) : t("All watched")].filter(Boolean).join(" · ")}
        </div>
        <div class="acts">
          {#if nu}
            <button class="btn primary" onclick={() => playItem(nu)}>
              <Icon name="play" size={16} fill />
              {started ? t("Resume {ep}", { ep: episodeTag(nu.season, nu.episode) }) : t("Play {ep}", { ep: episodeTag(nu.season, nu.episode) })}
            </button>
          {/if}
          <button class="btn secondary" onclick={() => api.librarySetWatched(show.episodes.map((e) => e.id), show.unwatched > 0)}>
            <Icon name={show.unwatched > 0 ? "circle-check" : "eye-off"} size={16} />
            {show.unwatched > 0 ? t("Mark all watched") : t("Mark all unwatched")}
          </button>
        </div>
      </div>
    </div>
  </div>

  {#if show.seasons.length > 1}
    <div class="seasons">
      <Segmented value={String(seasonNow)} onchange={(v) => (season = Number(v))} options={show.seasons.map((s) => ({ value: String(s), label: t("Season {n}", { n: s }) }))} />
    </div>
  {/if}

  <div class="eps">
    {#each eps as e (e.id)}
      {@const f = fraction(e)}
      <div class="ep" class:next={nu?.id === e.id} role="button" tabindex="0" onclick={() => playItem(e)} onkeydown={(k) => k.key === "Enter" && playItem(e)} oncontextmenu={(ev) => (ev.preventDefault(), openMenu(itemMenu(e), { x: ev.clientX, y: ev.clientY }))}>
        <div class="thumb">
          {#if e.thumb}<img src={fileSrc(e.thumb)} alt="" loading="lazy" />{/if}
          <div class="play"><Icon name="play" size={18} fill /></div>
          {#if f > 0 && f < 1}<div class="bar"><div style:width={f * 100 + "%"}></div></div>{/if}
        </div>
        <div class="num-col num">{e.episode != null ? num(e.episode) : "–"}</div>
        <div class="text">
          <div class="et ellipsis"><bdi>{e.episode_title || t("Episode {n}", { n: e.episode ?? "" })}</bdi></div>
          <div class="es muted ellipsis num">{[episodeTag(e.season, e.episode, e.episode_end), duration(e.duration), fileName(e.path)].filter(Boolean).join(" · ")}</div>
        </div>
        {#if watched(e)}<div class="seen"><Icon name="circle-check" size={18} /></div>{/if}
        <button class="icon-btn more" aria-label={t("More")} onclick={(ev) => (ev.stopPropagation(), openMenu(itemMenu(e), { anchor: (ev.currentTarget as HTMLElement).getBoundingClientRect(), align: "end" }))}>
          <Icon name="ellipsis" size={18} />
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .missing {
    padding: 60px 40px;
  }
  .hero {
    position: relative;
    min-height: 320px;
    overflow: hidden;
  }
  .bg {
    position: absolute;
    inset: -40px;
    background: center 30% / cover no-repeat;
    filter: blur(36px) brightness(0.45) saturate(0.7);
    transform: scale(1.1);
  }
  .fade {
    position: absolute;
    inset: 0;
    background: linear-gradient(to bottom, rgba(8, 8, 8, 0.2), var(--bg-primary));
  }
  .inner {
    position: relative;
    display: flex;
    gap: 28px;
    align-items: flex-end;
    padding: 36px 40px 24px;
  }
  .poster {
    width: 180px;
    aspect-ratio: 2/3;
    border-radius: var(--r-l);
    overflow: hidden;
    background: var(--bg-surface);
    box-shadow: var(--shadow-card);
    display: grid;
    place-items: center;
    color: var(--text-disabled);
    flex: none;
  }
  .poster img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  h1 {
    margin: 4px 0 6px;
    font-family: var(--font-display);
    font-size: 36px;
    font-weight: 700;
    letter-spacing: -0.015em;
  }
  .facts {
    font-size: 14px;
  }
  .acts {
    display: flex;
    gap: 10px;
    margin-top: 18px;
    flex-wrap: wrap;
  }
  .seasons {
    padding: 6px 40px 4px;
  }
  .eps {
    padding: 10px 28px 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .ep {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 10px 12px;
    border-radius: var(--r-l);
    transition: background var(--t-fast);
  }
  .ep:hover {
    background: var(--bg-surface);
  }
  .ep.next {
    background: var(--bg-surface);
    box-shadow: 0 0 0 1px var(--border-subtle) inset;
  }
  .thumb {
    position: relative;
    width: 176px;
    aspect-ratio: 16/9;
    border-radius: 10px;
    overflow: hidden;
    background: var(--bg-elevated);
    flex: none;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .play {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.35);
    opacity: 0;
    transition: opacity var(--t-fast);
  }
  .ep:hover .play {
    opacity: 1;
  }
  .bar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 4px;
    background: rgba(255, 255, 255, 0.2);
  }
  .bar div {
    height: 100%;
    background: var(--progress-active);
  }
  .num-col {
    width: 28px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 17px;
    font-weight: 600;
    flex: none;
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .et {
    font-weight: 600;
    font-size: 14.5px;
  }
  .es {
    font-size: 12.5px;
    margin-top: 2px;
  }
  .seen {
    color: var(--text-secondary);
  }
  .more {
    opacity: 0;
  }
  .ep:hover .more {
    opacity: 1;
  }
</style>
