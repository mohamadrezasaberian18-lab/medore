<script lang="ts">
  import PageHeader from "../PageHeader.svelte";
  import PosterCard from "../PosterCard.svelte";
  import Segmented from "../../components/Segmented.svelte";
  import Empty from "../Empty.svelte";
  import { movies, sortItems, sortState, fraction, watched, posterFor, posterKeyMovie, lib, type SortKey } from "../../lib/library.svelte";
  import { playItem, itemMenu } from "../../lib/play";
  import { t, num, tn } from "../../lib/i18n.svelte";
  import { duration, resolutionLabel } from "../../lib/format";
  import { openMenu } from "../../lib/ui.svelte";

  let filter = $state("all");
  const sorts = $derived([
    { key: "added", label: t("Date added"), desc: true },
    { key: "title", label: t("Title") },
    { key: "year", label: t("Release year"), desc: true },
    { key: "duration", label: t("Length") },
    { key: "watched", label: t("Last watched"), desc: true },
    { key: "size", label: t("File size"), desc: true },
  ]);
  const st = $derived(sortState("movies", "added", true));
  const list = $derived.by(() => {
    let l = movies();
    if (filter === "unwatched") l = l.filter((m) => !lib.progress[m.id]?.finished);
    else if (filter === "progress") l = l.filter((m) => fraction(m) > 0 && fraction(m) < 1);
    else if (filter === "watched") l = l.filter((m) => lib.progress[m.id]?.finished);
    return sortItems(l, st.by as SortKey, st.desc);
  });
</script>

<PageHeader title={t("Movies")} count={tn("{n} movie", "{n} movies", list.length)} page="movies" {sorts} defaultSort="added">
  <Segmented
    value={filter}
    onchange={(v) => (filter = v)}
    options={[
      { value: "all", label: t("All") },
      { value: "unwatched", label: t("Unwatched") },
      { value: "progress", label: t("In progress") },
      { value: "watched", label: t("Watched") },
    ]}
  />
</PageHeader>

{#if !movies().length}
  <Empty icon="film" title={t("No movies yet")} text={t("Movies in your library folders appear here, with posters.")} />
{:else}
  <div class="grid">
    {#each list as m (m.id)}
      <PosterCard
        title={m.title}
        sub={[m.year ? num(m.year) : "", duration(m.duration)].filter(Boolean).join(" · ")}
        poster={posterFor(posterKeyMovie(m.title, m.year))}
        thumb={m.thumb}
        progress={fraction(m)}
        watched={watched(m)}
        badge={m.hdr ?? (resolutionLabel(m.width, m.height) === "4K" ? "4K" : "")}
        onplay={() => playItem(m)}
        onmenu={(e) => openMenu(itemMenu(m), { x: e.clientX, y: e.clientY })}
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
