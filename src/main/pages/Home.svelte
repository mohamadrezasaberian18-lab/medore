<script lang="ts">
  import Icon from "../../components/Icon.svelte";
  import Row from "../Row.svelte";
  import WideCard from "../WideCard.svelte";
  import PosterCard from "../PosterCard.svelte";
  import { lib, continueWatching, recentlyAdded, shows, nextUp, fraction, watched, posterFor, posterKeyMovie, recentFiles } from "../../lib/library.svelte";
  import { playItem, playPath, itemMenu } from "../../lib/play";
  import { api, fileName } from "../../lib/api";
  import { t, num } from "../../lib/i18n.svelte";
  import { duration, episodeTag, clock } from "../../lib/format";
  import { openMenu } from "../../lib/ui.svelte";
  import { go } from "../../lib/nav.svelte";
  import { pickMedia } from "../../lib/menus";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";

  const cont = $derived(continueWatching());
  const added = $derived(recentlyAdded(24));
  const upNext = $derived(
    shows()
      .filter((s) => s.lastWatched > 0)
      .sort((a, b) => b.lastWatched - a.lastWatched)
      .map((s) => ({ s, e: nextUp(s) }))
      .filter((x) => x.e && !cont.some((c) => c.id === x.e!.id))
      .slice(0, 16),
  );
  const recent = $derived(recentFiles(16).filter((r) => !r.item && !cont.some((c) => c.id === r.id)));
  const empty = $derived(lib.loaded && lib.items.length === 0);
  const hour = new Date().getHours();
  const greeting = $derived(hour < 5 ? t("Good evening") : hour < 12 ? t("Good morning") : hour < 18 ? t("Good afternoon") : t("Good evening"));

  async function addFolder() {
    const d = await openDialog({ directory: true, multiple: false });
    if (typeof d === "string") await api.libraryAddFolder(d);
  }
</script>

<div class="hero">
  <div class="greet">{greeting}</div>
  <h1>{cont.length ? t("Pick up where you left off") : t("What will you watch today?")}</h1>
</div>

{#if empty}
  <div class="empty">
    <img src="/brand/medore-icon-dark.svg" alt="" width="84" height="84" />
    <h2>{t("Your library is empty")}</h2>
    <p class="muted">{t("Add the folders where you keep movies, shows and music. MEDORE finds posters, episodes and subtitles by itself.")}</p>
    <div class="acts">
      <button class="btn primary" onclick={addFolder}><Icon name="folder-plus" size={17} />{t("Add a folder")}</button>
      <button class="btn secondary" onclick={() => pickMedia("replace")}><Icon name="file-plus" size={17} />{t("Open a file")}</button>
    </div>
  </div>
{/if}

{#if cont.length}
  <Row title={t("Continue watching")} min={300}>
    {#each cont as c (c.id)}
      <WideCard
        size="l"
        title={c.item?.kind === "episode" ? c.item.show || c.title : c.title || fileName(c.path)}
        sub={(c.item?.kind === "episode" ? episodeTag(c.item.season, c.item.episode) + " · " : "") + t("{time} left", { time: duration(Math.max(0, c.duration - c.pos)) || clock(c.duration - c.pos) })}
        thumb={c.thumb}
        progress={c.duration > 0 ? c.pos / c.duration : -1}
        onplay={() => playPath(c.path)}
        onmenu={(e) =>
          openMenu(
            c.item
              ? itemMenu(c.item)
              : [
                  { label: t("Resume"), icon: "play", onSelect: () => api.open([c.path]) },
                  { label: t("Remove from history"), icon: "history", onSelect: () => api.libraryForget([c.id]) },
                  { label: t("Show in folder"), icon: "folder-open", onSelect: () => api.reveal(c.path) },
                ],
            { x: e.clientX, y: e.clientY },
          )}
      />
    {/each}
  </Row>
{/if}

{#if upNext.length}
  <Row title={t("Next up")} min={260}>
    {#each upNext as { s, e } (s.key)}
      <WideCard
        title={s.name}
        sub={[episodeTag(e!.season, e!.episode), e!.episode_title].filter(Boolean).join(" · ")}
        thumb={e!.thumb}
        progress={fraction(e!)}
        onplay={() => playItem(e!)}
        onmenu={(ev) => openMenu(itemMenu(e!, [{ label: t("Open show"), icon: "tv", onSelect: () => go("show", s.key) }]), { x: ev.clientX, y: ev.clientY })}
      />
    {/each}
  </Row>
{/if}

{#if added.length}
  <Row title={t("Recently added")} min={170}>
    {#each added as it (it.id)}
      {#if it.kind === "movie"}
        <PosterCard
          title={it.title}
          sub={it.year ? num(it.year) : duration(it.duration)}
          poster={posterFor(posterKeyMovie(it.title, it.year))}
          thumb={it.thumb}
          progress={fraction(it)}
          watched={watched(it)}
          badge={it.hdr ?? ""}
          onplay={() => playItem(it)}
          onmenu={(e) => openMenu(itemMenu(it), { x: e.clientX, y: e.clientY })}
        />
      {:else}
        {@const s = shows().find((x) => x.key === (it.show || it.title).toLowerCase())}
        <PosterCard
          title={it.show || it.title}
          sub={episodeTag(it.season, it.episode)}
          poster={s?.poster}
          thumb={it.thumb}
          progress={fraction(it)}
          watched={watched(it)}
          onopen={() => s && go("show", s.key)}
          onplay={() => playItem(it)}
          onmenu={(e) => openMenu(itemMenu(it), { x: e.clientX, y: e.clientY })}
        />
      {/if}
    {/each}
  </Row>
{/if}

{#if recent.length}
  <Row title={t("Recently played files")} min={240}>
    {#each recent as r (r.id)}
      <WideCard
        title={r.title || fileName(r.path)}
        sub={r.finished ? t("Watched") : r.pos > 0 ? t("{time} left", { time: duration(Math.max(0, r.duration - r.pos)) }) : ""}
        thumb={r.thumb}
        watched={r.finished}
        progress={!r.finished && r.duration > 0 ? r.pos / r.duration : -1}
        onplay={() => api.open([r.path])}
        onmenu={(e) =>
          openMenu(
            [
              { label: t("Play"), icon: "play", onSelect: () => api.open([r.path]) },
              { label: t("Remove from history"), icon: "history", onSelect: () => api.libraryForget([r.id]) },
              { label: t("Show in folder"), icon: "folder-open", onSelect: () => api.reveal(r.path) },
            ],
            { x: e.clientX, y: e.clientY },
          )}
      />
    {/each}
  </Row>
{/if}

<style>
  .hero {
    padding: 22px 40px 0;
  }
  .greet {
    color: var(--text-secondary);
    font-size: 14px;
  }
  h1 {
    margin: 4px 0 0;
    font-family: var(--font-display);
    font-size: 32px;
    font-weight: 700;
    letter-spacing: -0.015em;
  }
  .empty {
    margin: 48px auto 0;
    max-width: 520px;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 0 24px;
  }
  .empty img {
    border-radius: 20px;
    margin-bottom: 12px;
    box-shadow: var(--shadow-card);
  }
  .empty h2 {
    margin: 0;
    font-size: 21px;
  }
  .empty p {
    margin: 4px 0 18px;
  }
  .acts {
    display: flex;
    gap: 10px;
  }
</style>
