<script lang="ts">
  // 2:3 card for movies and shows. Uses the online poster when there is one,
  // otherwise a frame from the video with the title on it.
  import Icon from "../components/Icon.svelte";
  import { fileSrc } from "../lib/api";

  let {
    title,
    sub = "",
    poster,
    thumb,
    progress = -1,
    watched = false,
    badge = "",
    onplay,
    onopen,
    onmenu,
  }: {
    title: string;
    sub?: string;
    poster?: string | null;
    thumb?: string | null;
    progress?: number;
    watched?: boolean;
    badge?: string;
    onplay?: () => void;
    onopen?: () => void;
    onmenu?: (e: MouseEvent) => void;
  } = $props();

  let failed = $state(false);
  const src = $derived(!failed ? fileSrc(poster || thumb) : undefined);
  const isPoster = $derived(!!poster && !failed);
</script>

<div class="card" role="button" tabindex="0" onclick={() => (onopen ?? onplay)?.()} onkeydown={(e) => e.key === "Enter" && (onopen ?? onplay)?.()} oncontextmenu={(e) => (e.preventDefault(), onmenu?.(e))}>
  <div class="frame">
    {#if src}
      <img class:frameimg={!isPoster} {src} alt="" loading="lazy" decoding="async" onerror={() => (failed = true)} />
    {/if}
    {#if !isPoster}
      <div class="shade"></div>
      <div class="label">{title}</div>
    {/if}
    {#if badge}<div class="badge">{badge}</div>{/if}
    {#if watched}<div class="seen" title="✓"><Icon name="check" size={14} stroke={2.6} /></div>{/if}
    {#if onplay}
      <button class="play" aria-label="Play" onclick={(e) => (e.stopPropagation(), onplay?.())}><Icon name="play" size={22} fill /></button>
    {/if}
    {#if progress > 0 && progress < 1}
      <div class="bar"><div style:width={progress * 100 + "%"}></div></div>
    {/if}
  </div>
  <div class="meta">
    <div class="t ellipsis"><bdi>{title}</bdi></div>
    {#if sub}<div class="s ellipsis muted num">{sub}</div>{/if}
  </div>
</div>

<style>
  .card {
    min-width: 0;
    outline-offset: 4px;
    border-radius: var(--r-l);
  }
  .frame {
    position: relative;
    aspect-ratio: 2 / 3;
    border-radius: var(--r-l);
    overflow: hidden;
    background: linear-gradient(160deg, #1d1d1d, #111);
    box-shadow: 0 0 0 1px var(--hairline) inset;
    transition: transform var(--t) var(--ease), box-shadow var(--t) var(--ease);
  }
  .card:hover .frame {
    transform: translateY(-4px) scale(1.02);
    box-shadow: var(--shadow-card), 0 0 0 1px rgba(255, 255, 255, 0.12) inset;
  }
  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    animation: appear 400ms var(--ease);
  }
  img.frameimg {
    filter: saturate(0.9);
  }
  @keyframes appear {
    from {
      opacity: 0;
    }
  }
  .shade {
    position: absolute;
    inset: 0;
    background: linear-gradient(to top, rgba(0, 0, 0, 0.85) 10%, rgba(0, 0, 0, 0.1) 60%);
  }
  .label {
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 12px;
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: 600;
    line-height: 1.25;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .badge {
    position: absolute;
    top: 8px;
    inset-inline-start: 8px;
    padding: 2px 7px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.7);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.02em;
  }
  .seen {
    position: absolute;
    top: 8px;
    inset-inline-end: 8px;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--accent-primary);
    color: var(--on-accent);
  }
  .play {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 52px;
    height: 52px;
    margin: -26px 0 0 -26px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: rgba(255, 255, 255, 0.92);
    color: #000;
    opacity: 0;
    transform: scale(0.8);
    transition: opacity var(--t) var(--ease), transform var(--t) var(--ease), background var(--t-fast);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.5);
  }
  .play :global(svg) {
    margin-inline-start: 3px;
  }
  .card:hover .play,
  .card:focus-visible .play {
    opacity: 1;
    transform: scale(1);
  }
  .play:hover {
    background: #fff;
    transform: scale(1.08) !important;
  }
  .bar {
    position: absolute;
    left: 10px;
    right: 10px;
    bottom: 10px;
    height: 4px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.25);
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    background: var(--progress-active);
  }
  .meta {
    padding: 9px 2px 0;
  }
  .t {
    font-weight: 600;
    font-size: 13.5px;
  }
  .s {
    font-size: 12.5px;
    margin-top: 1px;
  }
</style>
