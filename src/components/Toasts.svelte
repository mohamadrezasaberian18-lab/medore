<script lang="ts">
  import { fly } from "svelte/transition";
  import Icon from "./Icon.svelte";
  import { ui, dismissToast } from "../lib/ui.svelte";
  import { reducedMotion } from "../lib/settings.svelte";
  let { bottom = 24 }: { bottom?: number } = $props();
</script>

<div class="toasts" style:bottom={bottom + "px"}>
  {#each ui.toasts as tst (tst.id)}
    <div class="toast" role="status" transition:fly={{ y: 12, duration: reducedMotion() ? 0 : 200 }}>
      {#if tst.icon}<Icon name={tst.icon} size={18} />{/if}
      <span class="text">{tst.text}</span>
      {#if tst.action}
        <button
          class="act"
          onclick={() => {
            tst.action!.run();
            dismissToast(tst.id);
          }}>{tst.action.label}</button
        >
      {/if}
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    left: 50%;
    transform: translateX(-50%);
    z-index: 900;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 42px;
    max-width: min(560px, 90vw);
    padding: 8px 10px 8px 14px;
    border-radius: var(--r-l);
    background: var(--glass-strong);
    backdrop-filter: blur(20px);
    border: 1px solid var(--border-subtle);
    box-shadow: var(--shadow-pop);
    font-size: 13.5px;
  }
  .text {
    padding-inline-end: 6px;
  }
  .act {
    height: 28px;
    padding: 0 12px;
    border-radius: 8px;
    background: var(--bg-elevated);
    font-weight: 600;
    font-size: 12.5px;
  }
  .act:hover {
    background: var(--bg-hover);
  }
</style>
