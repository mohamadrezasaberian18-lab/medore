<script lang="ts">
  let { checked, label = "", onchange }: { checked: boolean; label?: string; onchange: (v: boolean) => void } = $props();
</script>

<button class="toggle" class:on={checked} role="switch" aria-checked={checked} aria-label={label} onclick={() => onchange(!checked)}>
  <span class="dot"></span>
</button>

<style>
  .toggle {
    position: relative;
    width: 40px;
    height: 22px;
    border-radius: 11px;
    background: var(--bg-hover);
    border: 1px solid var(--border-subtle);
    flex: none;
    transition: background var(--t) var(--ease), border-color var(--t) var(--ease);
  }
  .toggle.on {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
  }
  .dot {
    position: absolute;
    top: 2px;
    inset-inline-start: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text-secondary);
    transition: transform var(--t) var(--ease), background var(--t) var(--ease);
  }
  .on .dot {
    background: var(--on-accent);
    transform: translateX(18px);
  }
  :global([dir="rtl"]) .on .dot {
    transform: translateX(-18px);
  }
</style>
