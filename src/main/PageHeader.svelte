<script lang="ts">
  // Page title with the page's sort order and filters. The sort choice is
  // remembered per page.
  import type { Snippet } from "svelte";
  import Icon from "../components/Icon.svelte";
  import { openMenu } from "../lib/ui.svelte";
  import { t } from "../lib/i18n.svelte";
  import { settings, set } from "../lib/settings.svelte";
  import { sortState } from "../lib/library.svelte";

  let {
    title,
    count = "",
    page = "",
    sorts = [],
    defaultSort = "title",
    children,
  }: { title: string; count?: string; page?: string; sorts?: { key: string; label: string; desc?: boolean }[]; defaultSort?: string; children?: Snippet } = $props();

  const state = $derived(sortState(page, defaultSort, sorts.find((s) => s.key === defaultSort)?.desc ?? false));
  const current = $derived(sorts.find((s) => s.key === state.by));

  function choose(key: string) {
    const all = { ...((settings.v["library.sort"] ?? {}) as Record<string, any>) };
    const def = sorts.find((s) => s.key === key);
    all[page] = { by: key, desc: state.by === key ? !state.desc : (def?.desc ?? false) };
    set("library.sort", all);
  }

  function open(e: MouseEvent) {
    openMenu(
      () => [
        { header: t("Sort by") },
        ...sorts.map((s) => ({ label: s.label, radio: true, checked: state.by === s.key, hint: state.by === s.key ? (state.desc ? "↓" : "↑") : "", keepOpen: false, onSelect: () => choose(s.key) })),
        { separator: true },
        { label: t("Ascending"), radio: true, checked: !state.desc, onSelect: () => setDir(false) },
        { label: t("Descending"), radio: true, checked: state.desc, onSelect: () => setDir(true) },
      ],
      { anchor: (e.currentTarget as HTMLElement).getBoundingClientRect(), align: "end" },
    );
  }
  function setDir(desc: boolean) {
    const all = { ...((settings.v["library.sort"] ?? {}) as Record<string, any>) };
    all[page] = { by: state.by, desc };
    set("library.sort", all);
  }
</script>

<header class="ph">
  <div class="left">
    <h1>{title}</h1>
    {#if count}<span class="count muted num">{count}</span>{/if}
  </div>
  <div class="right">
    {#if children}{@render children()}{/if}
    {#if sorts.length}
      <button class="btn ghost sm sortbtn" onclick={open} aria-haspopup="menu">
        <Icon name={state.desc ? "arrow-down-wide-narrow" : "arrow-up-narrow-wide"} size={16} />
        {current?.label ?? t("Sort")}
      </button>
    {/if}
  </div>
</header>

<style>
  .ph {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    padding: 26px 40px 8px;
    flex-wrap: wrap;
  }
  .left {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 30px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .count {
    font-size: 14px;
  }
  .right {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .sortbtn {
    color: var(--text-primary);
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
  }
</style>
