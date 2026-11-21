<script lang="ts">
  import Icon from "./Icon.svelte";
  import { SHORTCUTS } from "../lib/keys";
  import { t } from "../lib/i18n.svelte";
  let { onclose }: { onclose: () => void } = $props();
</script>

<div class="scrim" role="presentation" onclick={onclose}>
  <div class="sheet" role="dialog" aria-label={t("Keyboard shortcuts")} tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.key === "Escape" && onclose()}>
    <header>
      <div class="title">{t("Keyboard shortcuts")}</div>
      <button class="icon-btn" title={t("Close")} onclick={onclose}><Icon name="x" size={18} /></button>
    </header>
    <div class="grid">
      {#each SHORTCUTS as [keys, what]}
        <div class="k" dir="ltr">{keys}</div>
        <div class="w">{t(what)}</div>
      {/each}
    </div>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 800;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.5);
    animation: fade var(--t) var(--ease);
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  .sheet {
    width: min(720px, 92vw);
    max-height: 86vh;
    overflow-y: auto;
    padding: 18px 22px 22px;
    border-radius: var(--r-xl);
    background: var(--glass-strong);
    backdrop-filter: blur(28px);
    border: 1px solid var(--border-subtle);
    box-shadow: var(--shadow-pop);
    animation: pop var(--t) var(--ease);
  }
  @keyframes pop {
    from {
      transform: scale(0.97);
      opacity: 0;
    }
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }
  .title {
    font-size: 18px;
    font-weight: 600;
  }
  .grid {
    display: grid;
    grid-template-columns: auto 1fr auto 1fr;
    gap: 9px 16px;
    font-size: 13px;
  }
  .k {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--text-primary);
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    padding: 2px 8px;
    justify-self: start;
    white-space: nowrap;
  }
  :global([dir="rtl"]) .k {
    justify-self: end;
  }
  .w {
    color: var(--text-secondary);
    align-self: center;
  }
  @media (max-width: 700px) {
    .grid {
      grid-template-columns: auto 1fr;
    }
  }
</style>
