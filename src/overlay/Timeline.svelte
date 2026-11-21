<script lang="ts">
  // Seek bar: smooth playhead between updates, buffered range, chapter marks,
  // hover preview (frame + time + chapter) and drag to seek. Mirrored in RTL.
  import { player, P, act } from "../lib/player.svelte";
  import { api } from "../lib/api";
  import { clock } from "../lib/format";
  import { isRtl } from "../lib/i18n.svelte";

  let { compact = false, onscrub }: { compact?: boolean; onscrub?: (active: boolean) => void } = $props();

  let bar: HTMLDivElement;
  let shownPos = $state(0);
  let hoverX = $state<number | null>(null);
  let hoverT = $state(0);
  let dragT = $state<number | null>(null);
  let preview = $state<string | null>(null);
  let previewFor = "";
  let raf = 0;
  let lastSeek = 0;

  const duration = $derived(P.duration);
  const chapters = $derived((player.p["chapter-list"] ?? []) as { time: number; title?: string }[]);
  const cache = $derived(player.p["demuxer-cache-time"] ?? 0);
  const playedPct = $derived(duration > 0 ? Math.min(100, ((dragT ?? shownPos) / duration) * 100) : 0);
  const cachePct = $derived(duration > 0 ? Math.min(100, (cache / duration) * 100) : 0);
  const abA = $derived(typeof player.p["ab-loop-a"] === "number" ? player.p["ab-loop-a"] : null);
  const abB = $derived(typeof player.p["ab-loop-b"] === "number" ? player.p["ab-loop-b"] : null);

  // Interpolate the playhead between time-pos updates (they arrive ~5x/s).
  function frame() {
    const base = player.p["time-pos"] ?? 0;
    if (!P.paused && !player.p["seeking"] && !player.p["paused-for-cache"] && player.posAt) {
      const dt = (performance.now() - player.posAt) / 1000;
      shownPos = base + Math.min(dt, 0.6) * (player.p.speed ?? 1);
    } else shownPos = base;
    raf = requestAnimationFrame(frame);
  }
  $effect(() => {
    raf = requestAnimationFrame(frame);
    return () => cancelAnimationFrame(raf);
  });

  function timeAt(clientX: number): number {
    const r = bar.getBoundingClientRect();
    let f = (clientX - r.left) / r.width;
    if (isRtl()) f = 1 - f;
    return Math.max(0, Math.min(1, f)) * duration;
  }

  function chapterAt(t: number) {
    let c: { time: number; title?: string } | undefined;
    for (const ch of chapters) if (ch.time <= t) c = ch;
    return c?.title;
  }

  let previewTimer: any = 0;
  function requestPreview(t: number) {
    clearTimeout(previewTimer);
    previewTimer = setTimeout(async () => {
      if (!P.hasVideo || !P.path) return;
      const key = P.path + "@" + Math.floor(t / Math.max(1, duration / 300));
      if (key === previewFor) return;
      previewFor = key;
      const url = await api.preview(P.path, t, duration);
      if (previewFor === key && url) preview = url;
    }, 40);
  }

  function move(e: PointerEvent) {
    const r = bar.getBoundingClientRect();
    hoverX = Math.max(0, Math.min(r.width, e.clientX - r.left));
    hoverT = timeAt(e.clientX);
    if (dragT != null) {
      dragT = hoverT;
      const now = performance.now();
      if (now - lastSeek > 90) {
        lastSeek = now;
        api.cmd("seek", dragT, "absolute+keyframes");
      }
    }
    requestPreview(hoverT);
  }
  function leave() {
    if (dragT == null) {
      hoverX = null;
      preview = null;
      previewFor = "";
    }
  }
  function down(e: PointerEvent) {
    if (duration <= 0) return;
    bar.setPointerCapture(e.pointerId);
    dragT = timeAt(e.clientX);
    onscrub?.(true);
  }
  function up(e: PointerEvent) {
    if (dragT == null) return;
    const t = dragT;
    dragT = null;
    act.seekTo(t);
    shownPos = t;
    onscrub?.(false);
    const r = bar.getBoundingClientRect();
    if (e.clientY < r.top - 30 || e.clientY > r.bottom + 30) leave();
  }

  // hoverX is a physical offset from the bar's left edge, so no mirroring here
  const bubbleLeft = $derived.by(() => {
    if (hoverX == null || !bar) return 0;
    const w = bar.getBoundingClientRect().width;
    const half = P.hasVideo ? 90 : 40;
    return 4 + Math.max(half, Math.min(w - half, hoverX));
  });
</script>

<div class="timeline" class:compact class:active={hoverX != null || dragT != null}>
  <div
    class="hit"
    bind:this={bar}
    role="slider"
    tabindex="-1"
    aria-label="Seek"
    aria-valuemin={0}
    aria-valuemax={duration}
    aria-valuenow={shownPos}
    onpointermove={move}
    onpointerleave={leave}
    onpointerdown={down}
    onpointerup={up}
    onpointercancel={up}
  >
    <div class="rail">
      <div class="cache" style:width={cachePct + "%"}></div>
      {#if abA != null}
        <div class="ab" style:inset-inline-start={(abA / duration) * 100 + "%"} style:width={((abB ?? shownPos) - abA) / duration * 100 + "%"}></div>
      {/if}
      <div class="played" style:width={playedPct + "%"}></div>
      {#if hoverX != null && dragT == null}
        <div class="hover" style:width={(hoverT / duration) * 100 + "%"}></div>
      {/if}
      {#each chapters as ch}
        {#if ch.time > 0.5 && duration > 0}
          <div class="tick" style:inset-inline-start={(ch.time / duration) * 100 + "%"}></div>
        {/if}
      {/each}
    </div>
    <div class="head" style:inset-inline-start={playedPct + "%"}></div>
  </div>

  {#if hoverX != null && duration > 0}
    <div class="bubble" style:left={bubbleLeft + "px"}>
      {#if P.hasVideo}
        <div class="frame">
          {#if preview}<img src={preview} alt="" />{/if}
        </div>
      {/if}
      {#if chapterAt(dragT ?? hoverT)}<div class="chap ellipsis">{chapterAt(dragT ?? hoverT)}</div>{/if}
      <div class="t num">{clock(dragT ?? hoverT)}</div>
    </div>
  {/if}
</div>

<style>
  .timeline {
    position: relative;
    height: 22px;
    margin: 0 -4px;
  }
  .hit {
    position: absolute;
    inset: 0 4px;
    display: flex;
    align-items: center;
    touch-action: none;
  }
  .rail {
    position: relative;
    width: 100%;
    height: 4px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.22);
    overflow: hidden;
    transition: height var(--t-fast) var(--ease);
  }
  .active .rail {
    height: 7px;
  }
  .cache,
  .played,
  .hover,
  .ab {
    position: absolute;
    top: 0;
    bottom: 0;
    inset-inline-start: 0;
  }
  .cache {
    background: rgba(255, 255, 255, 0.22);
  }
  .hover {
    background: rgba(255, 255, 255, 0.28);
  }
  .played {
    background: var(--progress-active);
  }
  .ab {
    background: rgba(255, 255, 255, 0.35);
  }
  .tick {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 3px;
    margin-inline-start: -1.5px;
    background: rgba(0, 0, 0, 0.7);
  }
  .head {
    position: absolute;
    top: 50%;
    width: 15px;
    height: 15px;
    margin-inline-start: -7.5px;
    border-radius: 50%;
    background: var(--accent-hover);
    box-shadow: 0 1px 8px rgba(0, 0, 0, 0.6);
    transform: translateY(-50%) scale(0);
    transition: transform var(--t-fast) var(--ease);
    pointer-events: none;
  }
  .active .head {
    transform: translateY(-50%) scale(1);
  }
  .bubble {
    position: absolute;
    bottom: 26px;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    pointer-events: none;
    animation: rise var(--t) var(--ease);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translate(-50%, 4px);
    }
  }
  .frame {
    width: 176px;
    height: 99px;
    border-radius: 10px;
    overflow: hidden;
    background: #000;
    border: 1px solid rgba(255, 255, 255, 0.18);
    box-shadow: var(--shadow-pop);
  }
  .frame img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .chap {
    max-width: 200px;
    font-size: 12px;
    font-weight: 600;
    text-shadow: 0 1px 4px rgba(0, 0, 0, 0.9);
  }
  .t {
    padding: 2px 8px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.75);
    font-size: 12.5px;
    font-weight: 600;
  }
  .compact .rail {
    height: 3px;
  }
</style>
