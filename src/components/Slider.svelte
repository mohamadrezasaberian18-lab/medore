<script lang="ts">
  // Horizontal slider: drag, click, wheel and arrow keys; mirrored in RTL.
  import { isRtl } from "../lib/i18n.svelte";
  let {
    value,
    min = 0,
    max = 100,
    step = 1,
    width = 120,
    label = "",
    onchange,
    oninput,
  }: { value: number; min?: number; max?: number; step?: number; width?: number | string; label?: string; onchange?: (v: number) => void; oninput?: (v: number) => void } = $props();

  let track: HTMLDivElement;
  let dragging = $state(false);
  let local = $state<number | null>(null);
  const shown = $derived(local ?? value);
  const pct = $derived(Math.max(0, Math.min(1, (shown - min) / (max - min || 1))) * 100);

  function at(clientX: number) {
    const r = track.getBoundingClientRect();
    let f = (clientX - r.left) / r.width;
    if (isRtl()) f = 1 - f;
    f = Math.max(0, Math.min(1, f));
    const raw = min + f * (max - min);
    return Math.round(raw / step) * step;
  }
  function down(e: PointerEvent) {
    dragging = true;
    track.setPointerCapture(e.pointerId);
    local = at(e.clientX);
    oninput?.(local);
  }
  function move(e: PointerEvent) {
    if (!dragging) return;
    local = at(e.clientX);
    oninput?.(local);
  }
  function up() {
    if (!dragging) return;
    dragging = false;
    if (local != null) onchange?.(local);
    local = null;
  }
  function key(e: KeyboardEvent) {
    const rtl = isRtl();
    let d = 0;
    if (e.key === "ArrowUp" || e.key === (rtl ? "ArrowLeft" : "ArrowRight")) d = step;
    else if (e.key === "ArrowDown" || e.key === (rtl ? "ArrowRight" : "ArrowLeft")) d = -step;
    else if (e.key === "Home") d = min - value;
    else if (e.key === "End") d = max - value;
    else return;
    e.preventDefault();
    e.stopPropagation();
    onchange?.(Math.max(min, Math.min(max, value + d)));
  }
  function wheel(e: WheelEvent) {
    e.preventDefault();
    e.stopPropagation();
    onchange?.(Math.max(min, Math.min(max, value + (e.deltaY < 0 ? step : -step) * (step < 1 ? 1 : 5 / step))));
  }
</script>

<div
  class="slider"
  class:dragging
  style:width={typeof width === "number" ? width + "px" : width}
  role="slider"
  tabindex="0"
  aria-label={label}
  aria-valuemin={min}
  aria-valuemax={max}
  aria-valuenow={shown}
  bind:this={track}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  onkeydown={key}
  onwheel={wheel}
>
  <div class="rail"><div class="fill" style:width={pct + "%"}></div></div>
  <div class="knob" style:inset-inline-start={pct + "%"}></div>
</div>

<style>
  .slider {
    position: relative;
    height: 28px;
    display: flex;
    align-items: center;
    flex: none;
    touch-action: none;
  }
  .rail {
    position: relative;
    width: 100%;
    height: 4px;
    border-radius: 4px;
    background: var(--progress-inactive);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--progress-active);
  }
  :global([dir="rtl"]) .fill {
    margin-inline-start: auto;
    margin-left: auto;
  }
  .knob {
    position: absolute;
    top: 50%;
    width: 14px;
    height: 14px;
    margin-inline-start: -7px;
    border-radius: 50%;
    background: var(--text-primary);
    transform: translateY(-50%) scale(0.75);
    opacity: 0;
    transition: transform var(--t-fast) var(--ease), opacity var(--t-fast) var(--ease);
    box-shadow: 0 1px 6px rgba(0, 0, 0, 0.5);
  }
  .slider:hover .knob,
  .slider.dragging .knob,
  .slider:focus-visible .knob {
    opacity: 1;
    transform: translateY(-50%) scale(1);
  }
  .slider:hover .rail {
    height: 5px;
  }
</style>
