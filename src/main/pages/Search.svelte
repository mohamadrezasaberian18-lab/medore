<script lang="ts">
  import PosterCard from "../PosterCard.svelte";
  import WideCard from "../WideCard.svelte";
  import Icon from "../../components/Icon.svelte";
  import { search, shows, fraction, watched, posterFor, posterKeyMovie } from "../../lib/library.svelte";
  import { playItem, itemMenu } from "../../lib/play";
  import { nav, go } from "../../lib/nav.svelte";
  import { t, num } from "../../lib/i18n.svelte";
  import { duration, episodeTag } from "../../lib/format";
  import { openMenu } from "../../lib/ui.svelte";

  const results = $derived(search(nav.query));
  const movies = $derived(results.filter((r) => r.kind === "movie"));
  const showHits = $derived.by(() => {
    const q = nav.query.toLowerCase().trim();
    return shows().filter((s) => q.split(/\s+/).every((w) => s.name.toLowerCase().includes(w)));
  });
  const episodes = $derived(results.filter((r) => r.kind === "episode").slice(0, 40));
  const other = $derived(results.filter((r) => r.kind === "video" || r.kind === "music").slice(0, 40));
</script>

<div class="head">
  <h1>{t("Results for “{q}”", { q: nav.query.trim() })}</h1>
</div>

{#if !results.length && !showHits.length}
  <div class="none">
    <Icon name="search" size={34} stroke={1.4} />
    <p class="muted">{t("Nothing matches. Try another word, or part of the file name.")}</p>
  </div>
{/if}

{#if showHits.length}
  <h2>{t("TV Shows")}</h2>
  <div class="grid">
    {#each showHits as s (s.key)}
      <PosterCard title={s.name} sub={t("{n} episodes", { n: s.episodes.length })} poster={s.poster} thumb={s.thumb} onopen={() => go("show", s.key)} />
    {/each}
  </div>
{/if}

{#if movies.length}
  <h2>{t("Movies")}</h2>
  <div class="grid">
    {#each movies as m (m.id)}
      <PosterCard title={m.title} sub={m.year ? num(m.year) : ""} poster={posterFor(posterKeyMovie(m.title, m.year))} thumb={m.thumb} progress={fraction(m)} watched={watched(m)} onplay={() => playItem(m)} onmenu={(e) => openMenu(itemMenu(m), { x: e.clientX, y: e.clientY })} />
    {/each}
  </div>
{/if}

{#if episodes.length}
  <h2>{t("Episodes")}</h2>
  <div class="grid wide">
    {#each episodes as e (e.id)}
      <WideCard title={e.show || e.title} sub={[episodeTag(e.season, e.episode), e.episode_title].filter(Boolean).join(" · ")} thumb={e.thumb} progress={fraction(e)} watched={watched(e)} onplay={() => playItem(e)} onmenu={(ev) => openMenu(itemMenu(e), { x: ev.clientX, y: ev.clientY })} />
    {/each}
  </div>
{/if}

{#if other.length}
  <h2>{t("Videos and music")}</h2>
  <div class="grid wide">
    {#each other as o (o.id)}
      <WideCard title={o.title} sub={[o.artist, duration(o.duration)].filter(Boolean).join(" · ")} thumb={o.thumb} progress={fraction(o)} watched={watched(o)} onplay={() => playItem(o)} onmenu={(ev) => openMenu(itemMenu(o), { x: ev.clientX, y: ev.clientY })} />
    {/each}
  </div>
{/if}

<style>
  .head {
    padding: 22px 40px 0;
  }
  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 26px;
  }
  h2 {
    margin: 26px 40px 12px;
    font-size: 17px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 22px 18px;
    padding: 0 40px;
  }
  .grid.wide {
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  }
  .none {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    margin-top: 80px;
    color: var(--text-secondary);
  }
</style>
