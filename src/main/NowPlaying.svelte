<script lang="ts">
  // Bar at the bottom of the library while something is loaded.
  import Icon from "../components/Icon.svelte";
  import Slider from "../components/Slider.svelte";
  import { player, P, act } from "../lib/player.svelte";
  import { lib } from "../lib/library.svelte";
  import { api, fileName, fileSrc } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import { clock, episodeTag } from "../lib/format";

  const item = $derived(lib.items.find((i) => i.path.toLowerCase() === P.path.toLowerCase()));
  const title = $derived(item ? (item.kind === "episode" ? item.show || item.title : item.title) : player.p["medore-title"]?.title || P.title || fileName(P.path));
  const sub = $derived(
    item?.kind === "episode" ? [episodeTag(item.season, item.episode), item.episode_title].filter(Boolean).join(" · ") : item?.kind === "music" ? [item.artist, item.album].filter(Boolean).join(" · ") : "",
  );
  const pct = $derived(P.duration > 0 ? (P.pos / P.duration) * 100 : 0);
  const volume = $derived(Math.round(player.p.volume ?? 100));
</script>

<div class="np" role="region" aria-label={t("Now playing")}>
  <div class="prog"><div style:width={pct + "%"}></div></div>
  <button class="who" onclick={() => api.uiPlayer(true)} title={t("Open player")}>
    <div class="art">
      {#if item?.thumb}<img src={fileSrc(item.thumb)} alt="" />{:else}<Icon name={P.hasVideo ? "film" : "music"} size={20} />{/if}
      <div class="expand"><Icon name="maximize" size={16} /></div>
    </div>
    <div class="text">
      <div class="title ellipsis"><bdi>{title}</bdi></div>
      <div class="sub ellipsis muted num"><bdi>{sub || clock(P.pos) + " / " + clock(P.duration)}</bdi></div>
    </div>
  </button>
  <div class="controls">
    <button class="icon-btn" title={t("Previous")} onclick={() => act.prev()}><Icon name="skip-back" size={18} fill /></button>
    <button class="icon-btn main" title={P.paused ? t("Play") : t("Pause")} onclick={() => act.togglePause()}><Icon name={P.paused ? "play" : "pause"} size={20} fill /></button>
    <button class="icon-btn" title={t("Next")} disabled={P.playlistPos >= P.playlist.length - 1} onclick={() => act.next()}><Icon name="skip-forward" size={18} fill /></button>
  </div>
  <div class="end">
    <span class="time num muted">{clock(P.pos)} / {clock(P.duration)}</span>
    <Icon name={volume === 0 || player.p.mute ? "volume-x" : "volume-2"} size={18} />
    <Slider value={player.p.mute ? 0 : volume} max={130} width={90} label={t("Volume")} onchange={(v) => act.setVolume(v)} oninput={(v) => act.setVolume(v)} />
    <button class="icon-btn" title={t("Open player")} onclick={() => api.uiPlayer(true)}><Icon name="maximize" size={18} /></button>
    <button class="icon-btn" title={t("Stop")} onclick={() => act.stop()}><Icon name="x" size={18} /></button>
  </div>
</div>

<style>
  .np {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: center;
    gap: 16px;
    height: 72px;
    padding: 0 16px;
    background: var(--bg-surface);
    border-top: 1px solid var(--hairline);
    animation: up var(--t-slow) var(--ease);
  }
  @keyframes up {
    from {
      transform: translateY(100%);
    }
  }
  .prog {
    position: absolute;
    top: -1px;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--progress-inactive);
  }
  .prog div {
    height: 100%;
    background: var(--progress-active);
  }
  :global([dir="rtl"]) .prog div {
    margin-left: auto;
  }
  .who {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    text-align: start;
    border-radius: var(--r-m);
  }
  .art {
    position: relative;
    width: 76px;
    height: 44px;
    border-radius: 8px;
    overflow: hidden;
    background: var(--bg-elevated);
    display: grid;
    place-items: center;
    color: var(--text-disabled);
    flex: none;
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .expand {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.55);
    color: #fff;
    opacity: 0;
    transition: opacity var(--t-fast);
  }
  .who:hover .expand {
    opacity: 1;
  }
  .text {
    min-width: 0;
  }
  .title {
    font-weight: 600;
  }
  .sub {
    font-size: 12.5px;
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .main {
    width: 42px;
    height: 42px;
    border-radius: 50%;
    background: var(--accent-primary);
    color: var(--on-accent);
  }
  .main:hover {
    background: var(--accent-hover);
    color: var(--on-accent);
  }
  .icon-btn:disabled {
    opacity: 0.35;
    pointer-events: none;
  }
  .end {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    color: var(--text-secondary);
  }
  .time {
    font-size: 12.5px;
    margin-inline-end: 8px;
  }
  @media (max-width: 900px) {
    .time {
      display: none;
    }
  }
</style>
