<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { emit } from "@tauri-apps/api/event";
  import Icon from "../components/Icon.svelte";
  import MenuHost from "../components/MenuHost.svelte";
  import Toasts from "../components/Toasts.svelte";
  import Slider from "../components/Slider.svelte";
  import ShortcutsHelp from "../components/ShortcutsHelp.svelte";
  import Timeline from "./Timeline.svelte";
  import QueuePanel from "./QueuePanel.svelte";
  import InfoPanel from "./InfoPanel.svelte";
  import { player, P, act, onPlayerEvent } from "../lib/player.svelte";
  import { settings, reducedMotion } from "../lib/settings.svelte";
  import { api, isSubtitle, fileName, fileSrc } from "../lib/api";
  import { t, num } from "../lib/i18n.svelte";
  import { clock, episodeTag } from "../lib/format";
  import { openMenu, ui, toast } from "../lib/ui.svelte";
  import { moreItems, contextItems, subtitleItems, audioItems, speedItems } from "../lib/menus";
  import { handlePlayerKey } from "../lib/keys";
  import { lib } from "../lib/library.svelte";

  let visible = $state(true);
  let overControls = $state(false);
  let scrubbing = $state(false);
  let panel = $state<null | "queue" | "info">(null);
  let help = $state(false);
  let osdText = $state("");
  let osdKey = $state(0);
  let flash = $state<null | { icon: string; k: number }>(null);
  let upNextDismissed = $state(false);
  let hideTimer: any = 0;
  let clickTimer: any = 0;
  let osdTimer: any = 0;
  let bottomEl: HTMLDivElement;

  const timeout = $derived(Number(settings.v["appearance.controls_timeout"] ?? 2.5) * 1000);
  const showRemaining = $derived(!!settings.v["appearance.show_remaining"]);
  const volume = $derived(Math.round(player.p.volume ?? 100));
  const muted = $derived(!!player.p.mute);
  const speed = $derived(player.p.speed ?? 1);
  const subsOn = $derived(player.p.sid !== false && player.p.sid != null && player.p["sub-visibility"] !== false);
  const hasNext = $derived(P.playlistPos >= 0 && P.playlistPos < P.playlist.length - 1);
  const item = $derived(lib.items.find((i) => i.path.toLowerCase() === P.path.toLowerCase()));
  const titleText = $derived(item ? (item.kind === "episode" ? item.show || item.title : item.title) : player.p["medore-title"]?.title || P.title || fileName(P.path));
  const subtitleText = $derived.by(() => {
    if (!item) return [player.p["medore-title"]?.sub, P.playlist.length > 1 ? t("{n} of {m} in queue", { n: P.playlistPos + 1, m: P.playlist.length }) : ""].filter(Boolean).join(" · ");
    if (item.kind === "episode") return [episodeTag(item.season, item.episode, item.episode_end), item.episode_title].filter(Boolean).join(" · ");
    if (item.kind === "music") return [item.artist, item.album].filter(Boolean).join(" · ");
    return item.year ? num(item.year) : "";
  });
  const remaining = $derived(Math.max(0, P.duration - P.pos));
  const upNextSeconds = $derived(Number(settings.v["playback.up_next_seconds"] ?? 10));
  const nextEntry = $derived(hasNext ? P.playlist[P.playlistPos + 1] : null);
  const nextItem = $derived(nextEntry ? lib.items.find((i) => i.path.toLowerCase() === nextEntry.filename.toLowerCase()) : undefined);
  const showUpNext = $derived(
    !!nextEntry && !upNextDismissed && settings.v["playback.autoplay_next"] !== false && upNextSeconds > 0 && P.duration > 90 && remaining <= upNextSeconds && remaining > 0.3 && !P.paused,
  );
  const busy = $derived(player.loading || !!player.p["paused-for-cache"] || (!!player.p["seeking"] && !P.paused));
  const idle = $derived(!visible && !P.paused && !panel && !ui.menu && !help);

  function poke() {
    visible = true;
    clearTimeout(hideTimer);
    hideTimer = setTimeout(maybeHide, timeout);
  }
  function maybeHide() {
    if (P.paused || overControls || scrubbing || ui.menu || panel || help) {
      hideTimer = setTimeout(maybeHide, timeout);
      return;
    }
    visible = false;
  }

  function osd(text: string) {
    osdText = text;
    osdKey++;
    clearTimeout(osdTimer);
    osdTimer = setTimeout(() => (osdText = ""), 900);
  }

  function stageClick(e: MouseEvent) {
    if (e.button !== 0) return;
    if (ui.menu) return;
    if (panel) {
      panel = null;
      return;
    }
    if (!settings.v["playback.click_to_pause"]) return;
    clearTimeout(clickTimer);
    clickTimer = setTimeout(() => {
      act.togglePause();
      flash = { icon: P.paused ? "play" : "pause", k: Date.now() };
    }, 230);
  }
  function stageDbl() {
    clearTimeout(clickTimer);
    if (settings.v["playback.double_click_fullscreen"] !== false) act.fullscreen();
  }
  function hosts() {
    return {
      onInfo: () => (panel = panel === "info" ? null : "info"),
      onQueue: () => (panel = panel === "queue" ? null : "queue"),
      onShortcuts: () => (help = !help),
      onSettings: openSettings,
    };
  }
  function context(e: MouseEvent) {
    e.preventDefault();
    poke();
    openMenu(() => contextItems(hosts()), { x: e.clientX, y: e.clientY });
  }
  function more(e: MouseEvent) {
    poke();
    openMenu(() => moreItems(hosts()), { anchor: (e.currentTarget as HTMLElement).getBoundingClientRect(), align: "end" });
  }
  function menuAt(e: MouseEvent, items: () => any[]) {
    poke();
    openMenu(items, { anchor: (e.currentTarget as HTMLElement).getBoundingClientRect(), align: "center", above: true });
  }
  async function openSettings() {
    await emit("app://navigate", { page: "settings" });
    if (!P.paused && P.hasVideo) act.pause();
    act.back();
  }
  function goBack() {
    if (!P.paused && P.hasVideo) act.pause();
    act.back();
  }
  function wheel(e: WheelEvent) {
    if (panel || ui.menu || (e.target as HTMLElement).closest(".panel, .menu, .bottom")) return;
    poke();
    if (settings.v["playback.wheel"] === "seek") act.seekBy(e.deltaY < 0 ? 5 : -5);
    else {
      act.volumeBy(e.deltaY < 0 ? 5 : -5);
      osd(t("Volume {n}%", { n: Math.max(0, Math.min(130, volume + (e.deltaY < 0 ? 5 : -5))) }));
    }
  }
  function key(e: KeyboardEvent) {
    poke();
    handlePlayerKey(e, {
      toggleQueue: () => (panel = panel === "queue" ? null : "queue"),
      toggleInfo: () => (panel = panel === "info" ? null : "info"),
      toggleShortcuts: () => (help = !help),
      openSettings,
      osd,
    });
  }

  // Lift subtitles above the controls while they are visible.
  let lastMargin = -1;
  let winH = $state(window.innerHeight);
  $effect(() => {
    void winH;
    const show = visible && !!bottomEl;
    // clear only the seek bar and buttons, not the soft shade above them
    const bar = bottomEl?.querySelector(".timeline") as HTMLElement | null;
    const h = bar ? window.innerHeight - bar.getBoundingClientRect().top : 0;
    const margin = show && h > 0 ? Math.round((h / Math.max(200, window.innerHeight)) * 720) + 4 : 22;
    if (margin !== lastMargin) {
      lastMargin = margin;
      api.set("sub-margin-y", margin).catch(() => {});
    }
  });

  $effect(() => {
    // a new file: show the controls briefly and reset the up-next card
    void P.path;
    upNextDismissed = false;
    poke();
  });

  $effect(() => {
    if (P.paused) poke();
  });

  onMount(() => {
    poke();
    onPlayerEvent((ev) => {
      if (ev.type === "file-loaded") poke();
      if (ev.type === "resumed") {
        toast(t("Resumed from {time}", { time: clock(ev.pos) }), { icon: "history", ms: 6000, action: { label: t("Start over"), run: () => act.seekTo(0) } });
      }
      if (ev.type === "resume-available") {
        toast(t("Resume from {time}?", { time: clock(ev.pos) }), { icon: "history", ms: 10000, action: { label: t("Resume"), run: () => act.seekTo(ev.pos) } });
      }
    });
    const un = getCurrentWebview().onDragDropEvent(async (e) => {
      if (e.payload.type !== "drop") return;
      const paths = e.payload.paths;
      if (!paths.length) return;
      if (paths.every(isSubtitle)) {
        for (const p of paths) await api.addSubtitle(p);
        toast(t("Subtitle added"), { icon: "captions" });
      } else {
        await api.open(paths, { mode: "replace", sort: true });
      }
    });
    return () => {
      un.then((f) => f());
    };
  });

  const errorText: Record<string, string> = {
    "error.missing": "This file is no longer where it was. It may have been moved, renamed or deleted.",
    "error.share_offline": "The network folder with this file is not reachable right now.",
    "error.network": "MEDORE could not reach this address. Check the connection and the link.",
    "error.empty": "This file is empty.",
    "error.format": "This file is not a video or audio format MEDORE can play.",
    "error.corrupt": "This file could not be opened. It may be damaged or incomplete (still downloading?).",
    "error.audio_device": "The sound output could not be started. Check that a speaker or headphones are connected.",
    "error.video_output": "The video output could not be started. Updating the graphics driver usually helps.",
    "error.generic": "This file could not be played.",
  };
</script>

<svelte:window bind:innerHeight={winH} onkeydown={key} onmousemove={poke} onpointerdown={poke} />

<div
  class="stage"
  class:idle
  class:pip={player.pip}
  class:panel-open={!!panel}
  role="application"
  aria-label={t("Player")}
  onclick={stageClick}
  ondblclick={stageDbl}
  oncontextmenu={context}
  onwheel={wheel}
>
  {#if !P.hasVideo && !P.albumArt && !P.idle}
    <div class="audio-only" in:fade={{ duration: 200 }}>
      {#if item?.thumb}
        <div class="art" style:background-image={`url("${fileSrc(item.thumb)}")`}></div>
      {:else}
        <div class="art blank"><Icon name="music" size={64} stroke={1.25} /></div>
      {/if}
      <div class="atitle ellipsis">{titleText}</div>
      {#if subtitleText}<div class="asub muted ellipsis">{subtitleText}</div>{/if}
    </div>
  {/if}

  {#if busy && !player.error}
    <div class="spinner" in:fade={{ duration: 150, delay: 250 }}><Icon name="loader-circle" size={44} stroke={1.6} /></div>
  {/if}

  {#if player.error}
    <div class="error" role="alert" onclick={(e) => e.stopPropagation()} ondblclick={(e) => e.stopPropagation()}>
      <Icon name="triangle-alert" size={30} />
      <div class="etitle">{t("Can't play this file")}</div>
      <div class="etext muted">{t(errorText[player.error.key] ?? errorText["error.generic"])}</div>
      {#if player.error.path}<div class="epath" dir="ltr">{fileName(player.error.path)}</div>{/if}
      <div class="eact">
        {#if hasNext}<button class="btn primary" onclick={() => act.next()}>{t("Play next")}</button>{/if}
        <button class="btn secondary" onclick={goBack}>{t("Back to library")}</button>
      </div>
    </div>
  {/if}

  {#key flash?.k}
    {#if flash && !reducedMotion()}
      <div class="flash" aria-hidden="true"><Icon name={flash.icon} size={40} fill /></div>
    {/if}
  {/key}

  {#key osdKey}
    {#if osdText}
      <div class="osd num" out:fade={{ duration: 200 }}>{osdText}</div>
    {/if}
  {/key}

  <!-- top bar -->
  <div class="top" class:hidden={!visible} onpointerenter={() => (overControls = true)} onpointerleave={() => (overControls = false)} onclick={(e) => e.stopPropagation()} ondblclick={(e) => e.stopPropagation()}>
    <button class="icon-btn big" title={t("Back to library") + " (Backspace)"} onclick={goBack}><Icon name="arrow-left" size={22} /></button>
    <div class="titles">
      <div class="title ellipsis"><bdi>{titleText}</bdi></div>
      {#if subtitleText}<div class="subtitle ellipsis num"><bdi>{subtitleText}</bdi></div>{/if}
    </div>
    <div class="spacer"></div>
    {#if !player.pip}
      <button class="icon-btn big" title={t("Picture in picture") + " (P)"} onclick={() => act.pip()}><Icon name="picture-in-picture-2" size={20} /></button>
    {/if}
    <button class="icon-btn big" title={t("More")} aria-haspopup="menu" onclick={more}><Icon name="ellipsis" size={22} /></button>
  </div>

  <!-- bottom controls -->
  <div
    class="bottom"
    class:hidden={!visible}
    bind:this={bottomEl}
    onpointerenter={() => (overControls = true)}
    onpointerleave={() => (overControls = false)}
    onclick={(e) => e.stopPropagation()}
    ondblclick={(e) => e.stopPropagation()}
  >
    <Timeline compact={player.pip} onscrub={(s) => (scrubbing = s)} />
    <div class="row">
      <div class="group">
        <button class="icon-btn big" title={(P.paused ? t("Play") : t("Pause")) + " (Space)"} onclick={() => act.togglePause()}>
          <Icon name={P.paused ? "play" : "pause"} size={24} fill />
        </button>
        {#if !player.pip}
          <button class="icon-btn" title={t("Previous") + " (Shift+B)"} onclick={() => act.prev()}><Icon name="skip-back" size={19} fill /></button>
          <button class="icon-btn" title={t("Next") + " (Shift+N)"} disabled={!hasNext} onclick={() => act.next()}><Icon name="skip-forward" size={19} fill /></button>
          <button class="icon-btn" title={t("Back 10 seconds") + " (J)"} onclick={() => (act.seekBy(-10), osd("−10s"))}><Icon name="rotate-ccw" size={19} /></button>
          <button class="icon-btn" title={t("Forward 10 seconds") + " (L)"} onclick={() => (act.seekBy(10), osd("+10s"))}><Icon name="rotate-cw" size={19} /></button>
          <div class="volume">
            <button class="icon-btn" title={(muted ? t("Unmute") : t("Mute")) + " (M)"} onclick={() => act.toggleMute()}>
              <Icon name={muted || volume === 0 ? "volume-x" : volume < 50 ? "volume-1" : "volume-2"} size={20} />
            </button>
            <Slider value={muted ? 0 : volume} min={0} max={130} step={1} width={96} label={t("Volume")} oninput={(v) => act.setVolume(v)} onchange={(v) => (act.setVolume(v), muted && act.toggleMute())} />
          </div>
        {/if}
        <button class="time num" title={t("Show remaining time")} onclick={() => api.settingsSet("appearance.show_remaining", !showRemaining)}>
          {clock(P.pos)} <span class="muted">/ {showRemaining ? "−" + clock(remaining) : clock(P.duration)}</span>
        </button>
      </div>
      <div class="group">
        {#if !player.pip}
          <button class="icon-btn" class:on={subsOn} title={t("Subtitles")} aria-haspopup="menu" onclick={(e) => menuAt(e, subtitleItems)}><Icon name={subsOn ? "captions" : "captions-off"} size={20} /></button>
          <button class="icon-btn" title={t("Audio")} aria-haspopup="menu" onclick={(e) => menuAt(e, audioItems)}><Icon name="audio-lines" size={20} /></button>
          <button class="icon-btn speed" title={t("Speed")} aria-haspopup="menu" onclick={(e) => menuAt(e, speedItems)}>
            {#if Math.abs(speed - 1) < 0.001}<Icon name="gauge" size={20} />{:else}<span class="num">{num(speed)}×</span>{/if}
          </button>
          <button class="icon-btn" class:on={panel === "queue"} title={t("Queue") + " (Q)"} onclick={() => (panel = panel === "queue" ? null : "queue")}><Icon name="list-video" size={20} /></button>
        {:else}
          <button class="icon-btn" title={t("Back to full window")} onclick={() => act.pip(false)}><Icon name="maximize" size={18} /></button>
        {/if}
        {#if !player.pip}
          <button class="icon-btn" title={(player.fullscreen ? t("Exit full screen") : t("Full screen")) + " (F)"} onclick={() => act.fullscreen()}>
            <Icon name={player.fullscreen ? "minimize" : "maximize"} size={20} />
          </button>
        {/if}
      </div>
    </div>
  </div>

  {#if showUpNext && nextEntry}
    <div class="upnext" role="status" onclick={(e) => e.stopPropagation()} ondblclick={(e) => e.stopPropagation()} in:fade={{ duration: 200 }}>
      {#if nextItem?.thumb}<img class="nthumb" src={fileSrc(nextItem.thumb)} alt="" />{/if}
      <div class="ntext">
        <div class="overline">{t("Up next")}</div>
        <div class="nname ellipsis">
          {nextItem ? (nextItem.kind === "episode" ? [episodeTag(nextItem.season, nextItem.episode), nextItem.episode_title].filter(Boolean).join(" · ") || nextItem.title : nextItem.title) : fileName(nextEntry.filename)}
        </div>
        <div class="nbar"><div style:width={Math.max(0, Math.min(100, (1 - remaining / upNextSeconds) * 100)) + "%"}></div></div>
        <div class="nact">
          <button class="btn primary sm" onclick={() => act.next()}><Icon name="play" size={14} fill />{t("Play now")}</button>
          <button class="btn secondary sm" onclick={() => ((upNextDismissed = true), api.cancelAutoplay())}>{t("Cancel")}</button>
        </div>
      </div>
    </div>
  {/if}

  {#if panel === "queue"}
    <div class="panel-wrap" onclick={(e) => e.stopPropagation()} ondblclick={(e) => e.stopPropagation()} role="presentation"><QueuePanel onclose={() => (panel = null)} /></div>
  {:else if panel === "info"}
    <div class="panel-wrap" onclick={(e) => e.stopPropagation()} ondblclick={(e) => e.stopPropagation()} role="presentation"><InfoPanel onclose={() => (panel = null)} /></div>
  {/if}
</div>

{#if help}<ShortcutsHelp onclose={() => (help = false)} />{/if}
<Toasts bottom={visible ? 128 : 32} />
<MenuHost />

<style>
  .stage {
    position: fixed;
    inset: 0;
    overflow: hidden;
    /* almost-invisible fill keeps every pixel hit-testable */
    background: rgba(0, 0, 0, 0.004);
  }
  .stage.idle {
    cursor: none;
  }
  .top,
  .bottom {
    position: absolute;
    left: 0;
    right: 0;
    transition: opacity var(--t-slow) var(--ease), transform var(--t-slow) var(--ease);
  }
  .top {
    top: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 14px 16px 40px;
    background: linear-gradient(to bottom, rgba(0, 0, 0, 0.72), rgba(0, 0, 0, 0.32) 60%, transparent);
  }
  .bottom {
    bottom: 0;
    padding: 48px 20px 14px;
    background: linear-gradient(to top, rgba(0, 0, 0, 0.82), rgba(0, 0, 0, 0.45) 55%, transparent);
  }
  .hidden {
    opacity: 0;
    pointer-events: none;
  }
  .top.hidden {
    transform: translateY(-8px);
  }
  .bottom.hidden {
    transform: translateY(8px);
  }
  .titles {
    min-width: 0;
    padding-inline-start: 4px;
  }
  .title {
    font-size: 17px;
    font-weight: 600;
    text-shadow: 0 1px 6px rgba(0, 0, 0, 0.6);
  }
  .subtitle {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.75);
  }
  .spacer {
    flex: 1;
  }
  .panel-open .top .spacer ~ :global(*) {
    visibility: hidden;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 6px;
    gap: 12px;
  }
  .group {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
  }
  .icon-btn {
    color: rgba(255, 255, 255, 0.92);
  }
  .icon-btn:hover,
  .icon-btn.on {
    background: rgba(255, 255, 255, 0.14);
    color: #fff;
  }
  .icon-btn:disabled {
    opacity: 0.35;
    pointer-events: none;
  }
  .icon-btn.big {
    width: 42px;
    height: 42px;
  }
  .speed {
    width: auto;
    min-width: 36px;
    padding: 0 6px;
    font-weight: 700;
    font-size: 13px;
  }
  .volume {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-inline-end: 6px;
  }
  .time {
    height: 32px;
    padding: 0 10px;
    border-radius: 8px;
    font-size: 13.5px;
    font-weight: 600;
    white-space: nowrap;
  }
  .time:hover {
    background: rgba(255, 255, 255, 0.1);
  }
  .time .muted {
    color: rgba(255, 255, 255, 0.65);
    font-weight: 500;
  }
  .spinner {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    color: rgba(255, 255, 255, 0.9);
    pointer-events: none;
  }
  .spinner :global(svg) {
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .flash {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 84px;
    height: 84px;
    margin: -42px 0 0 -42px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.5);
    pointer-events: none;
    animation: flash 520ms var(--ease) forwards;
  }
  @keyframes flash {
    from {
      opacity: 1;
      transform: scale(0.85);
    }
    to {
      opacity: 0;
      transform: scale(1.25);
    }
  }
  .osd {
    position: absolute;
    top: 76px;
    left: 50%;
    transform: translateX(-50%);
    padding: 6px 14px;
    border-radius: 10px;
    background: rgba(0, 0, 0, 0.6);
    font-size: 15px;
    font-weight: 600;
    pointer-events: none;
  }
  .error {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    width: min(460px, 86vw);
    padding: 26px 26px 22px;
    border-radius: var(--r-xl);
    background: var(--glass-strong);
    border: 1px solid var(--border-subtle);
    box-shadow: var(--shadow-pop);
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }
  .etitle {
    font-size: 17px;
    font-weight: 600;
  }
  .etext {
    font-size: 13.5px;
  }
  .epath {
    font-size: 12px;
    color: var(--text-disabled);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .eact {
    display: flex;
    gap: 8px;
    margin-top: 10px;
  }
  .audio-only {
    position: absolute;
    inset: 0 0 140px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    pointer-events: none;
  }
  .art {
    width: min(42vh, 360px);
    aspect-ratio: 1;
    border-radius: var(--r-xl);
    background: var(--bg-surface) center/cover no-repeat;
    box-shadow: var(--shadow-card);
    display: grid;
    place-items: center;
    color: var(--text-disabled);
  }
  .atitle {
    margin-top: 10px;
    font-size: 22px;
    font-weight: 600;
    max-width: 80vw;
  }
  .asub {
    font-size: 14px;
    max-width: 80vw;
  }
  .upnext {
    position: absolute;
    inset-inline-end: 20px;
    bottom: 120px;
    display: flex;
    gap: 12px;
    width: 380px;
    max-width: calc(100vw - 40px);
    padding: 12px;
    border-radius: var(--r-l);
    background: var(--glass-strong);
    backdrop-filter: blur(24px);
    border: 1px solid var(--border-subtle);
    box-shadow: var(--shadow-pop);
  }
  .nthumb {
    width: 128px;
    height: 72px;
    object-fit: cover;
    border-radius: 8px;
    flex: none;
  }
  .ntext {
    min-width: 0;
    flex: 1;
  }
  .nname {
    font-weight: 600;
    margin: 2px 0 8px;
  }
  .nbar {
    height: 3px;
    border-radius: 3px;
    background: var(--progress-inactive);
    overflow: hidden;
    margin-bottom: 10px;
  }
  .nbar div {
    height: 100%;
    background: var(--progress-active);
    transition: width 0.25s linear;
  }
  .nact {
    display: flex;
    gap: 8px;
  }
  .panel-wrap {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .panel-wrap > :global(*) {
    pointer-events: auto;
  }
  /* picture in picture: just the essentials */
  .pip .top {
    padding: 8px 8px 24px;
  }
  .pip .title {
    font-size: 13px;
  }
  .pip .subtitle {
    display: none;
  }
  .pip .bottom {
    padding: 28px 10px 6px;
  }
  .pip .time {
    font-size: 12px;
    padding: 0 6px;
  }
</style>
