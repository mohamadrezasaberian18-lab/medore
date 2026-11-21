<script lang="ts">
  // 16:9 card for continue watching, episodes and personal videos.
  import Icon from "../components/Icon.svelte";
  import { fileSrc } from "../lib/api";

  let {
    title,
    sub = "",
    thumb,
    progress = -1,
    watched = false,
    badge = "",
    corner = "",
    size = "m",
    onplay,
    onmenu,
  }: {
    title: string;
    sub?: string;
    thumb?: string | null;
    progress?: number;
    watched?: boolean;
    badge?: string;
    corner?: string;
    size?: "m" | "l";
    onplay?: () => void;
    onmenu?: (e: MouseEvent) => void;
  } = $props();
  let failed = $state(false);
</script>

<div class="card {size}" role="button" tabindex="0" onclick={() => onplay?.()} onkeydown={(e) => e.key === "Enter" && onplay?.()} oncontextmenu={(e) => (e.preventDefault(), onmenu?.(e))}>
  <div class="frame">
    {#if thumb && !failed}
      <img src={fileSrc(thumb)} alt="" loading="lazy" decoding="async" onerror={() => (failed = true)} />
    {:else}
      <div class="ph"><Icon name="film" size={30} stroke={1.25} /></div>
    {/if}
    {#if badge}<div class="badge">{badge}</div>{/if}
    {#if corner}<div class="corner num">{corner}</div>{/if}
    {#if watched}<div class="seen"><Icon name="check" size={13} stroke={2.6} /></div>{/if}
    <div class="play"><Icon name="play" size={20} fill /></div>
    {#if progress > 0 && progress < 1}
      <div class="bar"><div style:width={progress * 100 + "%"}></div></div>
    {/if}
  </div>
  <div class="meta">
    <div class="t ellipsis"><bdi>{title}</bdi></div>
    {#if sub}<div class="s ellipsis muted num"><bdi>{sub}</bdi></div>{/if}
  </div>
</div>

<style>
  .card {
    min-width: 0;
    border-radius: var(--r-l);
    outline-offset: 4px;
  }
  .frame {
    position: relative;
    aspect-ratio: 16 / 9;
    border-radius: var(--r-l);
    overflow: hidden;
    background: linear-gradient(160deg, #1d1d1d, #111);
    box-shadow: 0 0 0 1px var(--hairline) inset;
    transition: transform var(--t) var(--ease), box-shadow var(--t) var(--ease);
  }
  .card:hover .frame {
    transform: translateY(-3px) scale(1.015);
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
  @keyframes appear {
    from {
      opacity: 0;
    }
  }
  .ph {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: var(--text-disabled);
  }
  .badge {
    position: absolute;
    top: 8px;
    inset-inline-start: 8px;
    padding: 2px 7px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.72);
    font-size: 11px;
    font-weight: 700;
  }
  .corner {
    position: absolute;
    bottom: 8px;
    inset-inline-end: 8px;
    padding: 1px 6px;
    border-radius: 5px;
    background: rgba(0, 0, 0, 0.75);
    font-size: 11.5px;
    font-weight: 600;
  }
  .seen {
    position: absolute;
    top: 8px;
    inset-inline-end: 8px;
    width: 20px;
    height: 20px;
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
    width: 46px;
    height: 46px;
    margin: -23px 0 0 -23px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: rgba(255, 255, 255, 0.92);
    color: #000;
    opacity: 0;
    transform: scale(0.8);
    transition: opacity var(--t) var(--ease), transform var(--t) var(--ease);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.5);
  }
  .play :global(svg) {
    margin-inline-start: 2px;
  }
  .card:hover .play,
  .card:focus-visible .play {
    opacity: 1;
    transform: scale(1);
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
  .meta {
    padding: 9px 2px 0;
  }
  .t {
    font-weight: 600;
    font-size: 13.5px;
  }
  .l .t {
    font-size: 15px;
  }
  .s {
    font-size: 12.5px;
    margin-top: 1px;
  }
</style>
