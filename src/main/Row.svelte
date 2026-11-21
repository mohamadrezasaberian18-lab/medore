<script lang="ts">
  // Horizontal shelf with scroll buttons.
  import type { Snippet } from "svelte";
  import Icon from "../components/Icon.svelte";
  import { isRtl } from "../lib/i18n.svelte";
  let { title, min = 180, action, children }: { title: string; min?: number; action?: Snippet; children: Snippet } = $props();
  let scroller: HTMLDivElement;
  let canBack = $state(false);
  let canFwd = $state(false);

  function update() {
    if (!scroller) return;
    const max = scroller.scrollWidth - scroller.clientWidth;
    const x = Math.abs(scroller.scrollLeft);
    canBack = x > 4;
    canFwd = x < max - 4;
  }
  function page(dir: 1 | -1) {
    const d = scroller.clientWidth * 0.85 * dir * (isRtl() ? -1 : 1);
    scroller.scrollBy({ left: d, behavior: "smooth" });
  }
  $effect(() => {
    update();
    const ro = new ResizeObserver(update);
    ro.observe(scroller);
    return () => ro.disconnect();
  });
</script>

<section class="row">
  <header>
    <h2>{title}</h2>
    <div class="right">
      {#if action}{@render action()}{/if}
      <button class="icon-btn" disabled={!canBack} aria-label="Back" onclick={() => page(-1)}><Icon name="chevron-left" size={18} /></button>
      <button class="icon-btn" disabled={!canFwd} aria-label="Forward" onclick={() => page(1)}><Icon name="chevron-right" size={18} /></button>
    </div>
  </header>
  <div class="scroller" bind:this={scroller} onscroll={update} style:--min={min + "px"}>
    {@render children()}
  </div>
</section>

<style>
  .row {
    margin-top: 30px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 40px;
    margin-bottom: 12px;
  }
  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 19px;
    font-weight: 600;
  }
  .right {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .icon-btn:disabled {
    opacity: 0.3;
    pointer-events: none;
  }
  .scroller {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: var(--min);
    gap: 18px;
    overflow-x: auto;
    overflow-y: hidden;
    padding: 6px 40px 14px;
    scroll-padding: 0 40px;
    scrollbar-width: none;
  }
  .scroller::-webkit-scrollbar {
    display: none;
  }
</style>
