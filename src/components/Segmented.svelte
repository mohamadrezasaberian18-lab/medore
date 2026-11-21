<script lang="ts">
  // Segmented control with a sliding highlight.
  let { options, value, onchange }: { options: { value: string; label: string }[]; value: string; onchange: (v: string) => void } = $props();
  let box: HTMLDivElement;
  let pill = $state({ left: 0, width: 0, ready: false });

  $effect(() => {
    void value;
    void options;
    queueMicrotask(measure);
  });

  function measure() {
    if (!box) return;
    const el = box.querySelector<HTMLElement>(`[data-v="${CSS.escape(value)}"]`);
    if (!el) return;
    pill = { left: el.offsetLeft, width: el.offsetWidth, ready: true };
  }
</script>

<svelte:window onresize={measure} />

<div class="seg" bind:this={box} role="radiogroup">
  <div class="pill" class:ready={pill.ready} style:left={pill.left + "px"} style:width={pill.width + "px"}></div>
  {#each options as o (o.value)}
    <button data-v={o.value} role="radio" aria-checked={o.value === value} class:on={o.value === value} onclick={() => onchange(o.value)}>{o.label}</button>
  {/each}
</div>

<style>
  .seg {
    position: relative;
    display: inline-flex;
    padding: 3px;
    border-radius: var(--r-m);
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
  }
  .pill {
    position: absolute;
    top: 3px;
    bottom: 3px;
    border-radius: 7px;
    background: var(--bg-hover);
  }
  .pill.ready {
    transition: left var(--t) var(--ease), width var(--t) var(--ease);
  }
  button {
    position: relative;
    height: 28px;
    padding: 0 12px;
    border-radius: 7px;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    transition: color var(--t-fast);
  }
  button.on {
    color: var(--text-primary);
  }
  button:hover {
    color: var(--text-primary);
  }
</style>
