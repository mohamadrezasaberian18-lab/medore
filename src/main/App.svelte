<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import Icon from "../components/Icon.svelte";
  import MenuHost from "../components/MenuHost.svelte";
  import Toasts from "../components/Toasts.svelte";
  import ShortcutsHelp from "../components/ShortcutsHelp.svelte";
  import NowPlaying from "./NowPlaying.svelte";
  import Home from "./pages/Home.svelte";
  import Movies from "./pages/Movies.svelte";
  import Shows from "./pages/Shows.svelte";
  import ShowDetail from "./pages/ShowDetail.svelte";
  import Videos from "./pages/Videos.svelte";
  import Music from "./pages/Music.svelte";
  import Search from "./pages/Search.svelte";
  import Settings from "./pages/Settings.svelte";
  import { nav, go, back, root, type Page } from "../lib/nav.svelte";
  import { player, P } from "../lib/player.svelte";
  import { lib, movies, shows, music, videos } from "../lib/library.svelte";
  import { settings, set } from "../lib/settings.svelte";
  import { api, on, isSubtitle } from "../lib/api";
  import { t, num } from "../lib/i18n.svelte";
  import { handlePlayerKey } from "../lib/keys";
  import { pickMedia } from "../lib/menus";
  import { toast } from "../lib/ui.svelte";

  let scrollEl: HTMLElement;
  let searchEl: HTMLInputElement;
  let help = $state(false);
  let dragging = $state(false);
  let update = $state<null | { version: string; ready: boolean }>(null);

  const collapsed = $derived(!!settings.v["appearance.sidebar_collapsed"]);
  const showBar = $derived(!player.mode && !P.idle && !!P.path);
  const navItems = $derived([
    { page: "home" as Page, icon: "house", label: t("Home"), n: 0 },
    { page: "movies" as Page, icon: "film", label: t("Movies"), n: movies().length },
    { page: "shows" as Page, icon: "tv", label: t("TV Shows"), n: shows().length },
    { page: "videos" as Page, icon: "folder", label: t("Videos"), n: videos().length },
    { page: "music" as Page, icon: "music", label: t("Music"), n: music().length },
  ]);
  const status = $derived(lib.status as any);

  $effect(() => {
    nav.scrollEl = scrollEl;
  });

  function onSearch(q: string) {
    nav.query = q;
    if (q.trim() && nav.page !== "search") go("search");
    else if (!q.trim() && nav.page === "search") back();
  }

  function key(e: KeyboardEvent) {
    if (player.mode) {
      // the video covers this window; it still gets the keys when focused
      handlePlayerKey(e, { toggleQueue() {}, toggleInfo() {}, toggleShortcuts: () => (help = !help), openSettings: () => api.uiPlayer(false), osd() {} });
      return;
    }
    const ctrl = e.ctrlKey || e.metaKey;
    const typing = (e.target as HTMLElement)?.tagName === "INPUT" || (e.target as HTMLElement)?.tagName === "TEXTAREA";
    if (ctrl && (e.key === "o" || e.key === "O")) pickMedia("replace");
    else if (ctrl && (e.key === "f" || e.key === "F")) searchEl?.focus();
    else if (ctrl && e.key === ",") go("settings");
    else if ((e.altKey && e.key === "ArrowLeft") || (e.key === "Backspace" && !typing)) back();
    else if (e.key === "F1") help = !help;
    else if (e.key === "Escape" && typing) {
      (e.target as HTMLInputElement).blur();
      if (nav.page === "search") onSearch("");
    } else if (e.key === " " && !typing && showBar) {
      api.cmd("cycle", "pause");
    } else return;
    e.preventDefault();
  }

  function mouse(e: MouseEvent) {
    if (e.button === 3) back();
  }

  onMount(() => {
    const start = (settings.v["general.start_page"] as Page) || "home";
    if (start !== "home") root(start);
    on<{ page: Page }>("app://navigate", ({ page }) => go(page));
    on<string[]>("app://dropped", async (paths) => {
      dragging = false;
      if (!paths.length) return;
      if (paths.every(isSubtitle) && P.path) {
        for (const p of paths) await api.addSubtitle(p);
        toast(t("Subtitle added"), { icon: "captions" });
        return;
      }
      await api.open(paths, { mode: "replace", sort: true });
    });
    on("update://ready", (u: any) => (update = { version: u.version, ready: true }));
    const enter = () => (dragging = true);
    const leave = (e: DragEvent) => {
      if (!e.relatedTarget) dragging = false;
    };
    window.addEventListener("dragenter", enter);
    window.addEventListener("dragleave", leave);
    window.addEventListener("drop", () => (dragging = false));
    return () => {
      window.removeEventListener("dragenter", enter);
      window.removeEventListener("dragleave", leave);
    };
  });
</script>

<svelte:window onkeydown={key} onmouseup={mouse} />

<div class="shell" class:collapsed>
  <aside class="side">
    <div class="brand">
      <img src="/brand/medore-icon-dark.svg" alt="" width="34" height="34" />
      {#if !collapsed}<span class="word">MEDORE</span>{/if}
    </div>
    <nav>
      {#each navItems as n (n.page)}
        <button
          class="nav"
          class:active={nav.page === n.page || (n.page === "shows" && nav.page === "show")}
          title={collapsed ? n.label : ""}
          onclick={() => root(n.page)}
        >
          <span class="ind"></span>
          <Icon name={n.icon} size={20} />
          {#if !collapsed}
            <span class="lbl">{n.label}</span>
            {#if n.n}<span class="n num">{num(n.n)}</span>{/if}
          {/if}
        </button>
      {/each}
    </nav>
    <div class="grow"></div>
    {#if !collapsed && (status?.state === "scanning" || status?.state === "probing")}
      <div class="scan muted" in:fade>
        <Icon name="loader-circle" size={15} class="spin" />
        <span class="ellipsis">{status.state === "scanning" ? t("Looking for media…") : t("Preparing {n} files…", { n: status.remaining ?? "" })}</span>
      </div>
    {/if}
    {#if update?.ready && !collapsed}
      <button class="update" onclick={() => api.updateInstall()}>
        <Icon name="download" size={16} />
        <span>{t("Update {v} ready — restart", { v: update.version })}</span>
      </button>
    {/if}
    <button class="nav" class:active={nav.page === "settings"} title={collapsed ? t("Settings") : ""} onclick={() => root("settings")}>
      <span class="ind"></span>
      <Icon name="settings" size={20} />
      {#if !collapsed}<span class="lbl">{t("Settings")}</span>{/if}
    </button>
    <button class="nav collapse" title={collapsed ? t("Expand sidebar") : t("Collapse sidebar")} onclick={() => set("appearance.sidebar_collapsed", !collapsed)}>
      <span class="ind"></span>
      <Icon name={collapsed ? "chevron-right" : "chevron-left"} size={18} />
      {#if !collapsed}<span class="lbl muted">{t("Collapse")}</span>{/if}
    </button>
  </aside>

  <main class="content">
    <div class="topbar">
      <button class="icon-btn" disabled={!nav.history.length} title={t("Back") + " (Alt+←)"} onclick={back}><Icon name="arrow-left" size={20} /></button>
      <div class="search">
        <Icon name="search" size={17} />
        <input
          bind:this={searchEl}
          type="search"
          placeholder={t("Search movies, shows, videos and music")}
          value={nav.query}
          oninput={(e) => onSearch((e.target as HTMLInputElement).value)}
          spellcheck="false"
        />
        {#if nav.query}<button class="clear" aria-label={t("Clear")} onclick={() => onSearch("")}><Icon name="x" size={15} /></button>{/if}
      </div>
      <div class="grow"></div>
      <button class="btn primary sm" onclick={() => pickMedia("replace")}><Icon name="file-plus" size={16} />{t("Open file")}</button>
    </div>

    <div class="scroll" bind:this={scrollEl}>
      {#key nav.page + nav.param}
        <div class="page" in:fade={{ duration: 160 }}>
          {#if nav.page === "home"}<Home />
          {:else if nav.page === "movies"}<Movies />
          {:else if nav.page === "shows"}<Shows />
          {:else if nav.page === "show"}<ShowDetail key={nav.param} />
          {:else if nav.page === "videos"}<Videos path={nav.param} />
          {:else if nav.page === "music"}<Music />
          {:else if nav.page === "search"}<Search />
          {:else if nav.page === "settings"}<Settings />
          {/if}
        </div>
      {/key}
    </div>

    {#if showBar}<NowPlaying />{/if}
  </main>
</div>

{#if dragging}
  <div class="drop" transition:fade={{ duration: 120 }}>
    <div class="dropbox">
      <Icon name="circle-play" size={44} stroke={1.4} />
      <div>{t("Drop to play")}</div>
      <div class="muted small">{t("Subtitle files are added to what is playing")}</div>
    </div>
  </div>
{/if}

{#if help}<ShortcutsHelp onclose={() => (help = false)} />{/if}
<Toasts bottom={showBar ? 92 : 24} />
<MenuHost />

<style>
  .shell {
    display: grid;
    grid-template-columns: 240px 1fr;
    height: 100vh;
    transition: grid-template-columns var(--t-slow) var(--ease);
  }
  .shell.collapsed {
    grid-template-columns: 72px 1fr;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 16px 12px 12px;
    background: var(--bg-primary);
    border-inline-end: 1px solid var(--hairline);
    min-width: 0;
    overflow: hidden;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 44px;
    padding: 0 7px;
    margin-bottom: 18px;
  }
  .brand img {
    flex: none;
    border-radius: 9px;
  }
  .word {
    font-family: "Segoe UI Variable Display", "Segoe UI", sans-serif;
    font-weight: 700;
    font-size: 17px;
    letter-spacing: 0.24em;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .nav {
    position: relative;
    display: flex;
    align-items: center;
    gap: 14px;
    height: 42px;
    padding: 0 13px;
    border-radius: var(--r-m);
    color: var(--text-secondary);
    font-weight: 500;
    white-space: nowrap;
    transition: background var(--t-fast) var(--ease), color var(--t-fast) var(--ease);
  }
  .nav:hover {
    background: var(--bg-surface);
    color: var(--text-primary);
  }
  .nav.active {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }
  .ind {
    position: absolute;
    inset-inline-start: 0;
    top: 11px;
    bottom: 11px;
    width: 3px;
    border-radius: 3px;
    background: var(--accent-hover);
    transform: scaleY(0);
    transition: transform var(--t) var(--ease);
  }
  .nav.active .ind {
    transform: scaleY(1);
  }
  .lbl {
    flex: 1;
    text-align: start;
  }
  .n {
    font-size: 12px;
    color: var(--text-disabled);
  }
  .grow {
    flex: 1;
  }
  .scan {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 13px;
    font-size: 12px;
  }
  .scan :global(.spin) {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .update {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 6px 0;
    padding: 10px 12px;
    border-radius: var(--r-m);
    background: var(--accent-primary);
    color: var(--on-accent);
    font-weight: 600;
    font-size: 12.5px;
    text-align: start;
  }
  .update:hover {
    background: var(--accent-hover);
  }
  .collapse {
    height: 36px;
  }
  .content {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--bg-primary);
  }
  .topbar {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 64px;
    padding: 0 24px 0 18px;
    flex: none;
  }
  .topbar .icon-btn:disabled {
    opacity: 0.3;
    pointer-events: none;
  }
  .search {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    width: min(460px, 50vw);
    height: 38px;
    padding: 0 12px;
    border-radius: 19px;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    color: var(--text-secondary);
    transition: border-color var(--t-fast), background var(--t-fast);
  }
  .search:focus-within {
    border-color: var(--accent-primary);
    background: var(--bg-elevated);
  }
  .search input {
    flex: 1;
    min-width: 0;
    background: none;
    border: 0;
    outline: 0;
    color: var(--text-primary);
    font-size: 13.5px;
  }
  .search input::-webkit-search-cancel-button {
    display: none;
  }
  .search input::placeholder {
    color: var(--text-disabled);
  }
  .clear {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    color: var(--text-secondary);
  }
  .clear:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .scroll {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding-bottom: 40px;
  }
  .drop {
    position: fixed;
    inset: 0;
    z-index: 700;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.6);
    pointer-events: none;
  }
  .dropbox {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 36px 56px;
    border-radius: var(--r-xl);
    border: 2px dashed var(--border-subtle);
    background: var(--bg-surface);
    font-size: 17px;
    font-weight: 600;
  }
  .small {
    font-size: 13px;
    font-weight: 400;
  }
</style>
