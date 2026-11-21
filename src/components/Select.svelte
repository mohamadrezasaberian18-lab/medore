<script lang="ts">
  // Dropdown that opens a menu of choices.
  import Icon from "./Icon.svelte";
  import { openMenu } from "../lib/ui.svelte";
  let { options, value, onchange, width = 220 }: { options: { value: any; label: string }[]; value: any; onchange: (v: any) => void; width?: number } = $props();
  const current = $derived(options.find((o) => o.value === value)?.label ?? String(value ?? ""));
  let btn: HTMLButtonElement;
  function open() {
    const r = btn.getBoundingClientRect();
    openMenu(
      options.map((o) => ({ label: o.label, radio: true, checked: o.value === value, onSelect: () => onchange(o.value) })),
      { anchor: r, align: "start" },
      r.width,
    );
  }
</script>

<button class="select field" bind:this={btn} style:width={width + "px"} onclick={open}>
  <span class="ellipsis">{current}</span>
  <Icon name="chevron-down" size={16} />
</button>

<style>
  .select {
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    text-align: start;
    font-size: 13.5px;
  }
  .select :global(svg) {
    color: var(--text-secondary);
  }
</style>
