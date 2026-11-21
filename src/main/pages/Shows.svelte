<script lang="ts">
  import PageHeader from "../PageHeader.svelte";
  import PosterCard from "../PosterCard.svelte";
  import Empty from "../Empty.svelte";
  import { shows, sortState, nextUp, lib } from "../../lib/library.svelte";
  import { playItem } from "../../lib/play";
  import { t, tn } from "../../lib/i18n.svelte";
  import { compareText } from "../../lib/format";
  import { openMenu } from "../../lib/ui.svelte";
  import { go } from "../../lib/nav.svelte";
  import { api } from "../../lib/api";

  const sorts = $derived([
    { key: "added", label: t("Date added"), desc: true },
    { key: "title", label: t("Title") },
    { key: "watched", label: t("Last watched"), desc: true },
    { key: "episodes", label: t("Number of episodes"), desc: true },
  ]);
  const st = $derived(sortState("shows", "added", true));
  const list = $derived.by(() => {
    const l = shows();
    const f: Record<string, (a: (typeof l)[0], b: (typeof l)[0]) => number> = {
      added: (a, b) => a.added - b.added,
      title: (a, b) => compareText(a.name, b.name),
      watched: (a, b) => a.lastWatched - b.lastWatched,
      episodes: (a, b) => a.episodes.length - b.episodes.length,
    };
    const cmp = f[st.by] ?? f.title;
    return [...l].sort((a, b) => {
      const r = cmp(a, b) || compareText(a.name, b.name);
      return st.desc ? -r : r;
    });
  });
</script>

<PageHeader title={t("TV Shows")} count={tn("{n} show", "{n} shows", list.length)} page="shows" {sorts} defaultSort="added" />

{#if !list.length}
  <Empty icon="tv" title={t("No TV shows yet")} text={t("Episodes such as Show.S01E02.mkv or Show/Season 1/02.mkv are grouped into shows automatically.")} />
{:else}
  <div class="grid">
    {#each list as s (s.key)}
      {@const nu = nextUp(s)}
      <PosterCard
        title={s.name}
        sub={[tn("{n} season", "{n} seasons", s.seasons.length), tn("{n} episode", "{n} episodes", s.episodes.length)].join(" · ")}
        poster={s.poster}
        thumb={s.thumb}
        watched={s.unwatched === 0}
        badge={s.unwatched > 0 && s.lastWatched > 0 ? tn("{n} new", "{n} new", s.unwatched) : ""}
        onopen={() => go("show", s.key)}
        onplay={() => nu && playItem(nu)}
        onmenu={(e) =>
          openMenu(
            [
              { label: t("Open"), icon: "tv", onSelect: () => go("show", s.key) },
              ...(nu ? [{ label: t("Play next episode"), icon: "play", onSelect: () => playItem(nu) }] : []),
              { separator: true },
              { label: t("Mark show as watched"), icon: "circle-check", onSelect: () => api.librarySetWatched(s.episodes.map((x) => x.id), true) },
              { label: t("Mark show as unwatched"), icon: "eye-off", onSelect: () => api.librarySetWatched(s.episodes.map((x) => x.id), false) },
            ],
            { x: e.clientX, y: e.clientY },
          )}
      />
    {/each}
  </div>
{/if}

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(168px, 1fr));
    gap: 26px 20px;
    padding: 18px 40px 0;
  }
</style>
