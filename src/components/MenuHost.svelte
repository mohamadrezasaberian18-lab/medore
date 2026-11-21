<script lang="ts">
  // One menu at a time (context menus, ⋯ menus, dropdowns), with submenus,
  // keyboard navigation, edge flipping and right-to-left mirroring.
  import { tick } from "svelte";
  import Icon from "./Icon.svelte";
  import { ui, closeMenu, type MenuItem, type MenuSource } from "../lib/ui.svelte";
  import { isRtl } from "../lib/i18n.svelte";
  import { player } from "../lib/player.svelte";
  import { settings } from "../lib/settings.svelte";

  type Level = { source: MenuSource; x: number; y: number; parentIndex: number };

  let levels: Level[] = $state([]);
  let active: number[] = $state([]);
  let panels: HTMLElement[] = $state([]);
  let placed: { left: number; top: number }[] = $state([]);
  let openTimer: any = 0;

  function resolve(src: MenuSource | undefined): MenuItem[] {
    if (!src) return [];
    // touch reactive state so live menus (delay, checks) update while open
    void player.seq;
    void player.p;
    void settings.v;
    return typeof src === "function" ? src() : src;
  }

  $effect(() => {
    const m = ui.menu;
    if (!m) {
      levels = [];
      active = [];
      return;
    }
    let x = m.x;
    let y = m.y;
    if (m.anchor) {
      const a = m.anchor;
      const rtl = isRtl();
      if (m.align === "end") x = rtl ? a.left : a.right;
      else if (m.align === "center") x = a.left + a.width / 2;
      else x = rtl ? a.right : a.left;
      y = m.above ? a.top - 6 : a.bottom + 6;
    }
    levels = [{ source: m.items, x, y, parentIndex: -1 }];
    active = [-1];
    placed = [];
    tick().then(() => place(0));
  });

  async function place(i: number) {
    await tick();
    const el = panels[i];
    const lv = levels[i];
    const m = ui.menu;
    if (!el || !lv || !m) return;
    const r = el.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const rtl = isRtl();
    let left: number;
    let top: number;
    if (i === 0) {
      if (m.anchor && m.align === "center") left = lv.x - r.width / 2;
      else if (m.anchor) left = (m.align === "end") !== rtl ? lv.x - r.width : lv.x;
      else left = rtl ? lv.x - r.width : lv.x;
      top = m.anchor && m.above ? lv.y - r.height : lv.y;
      if (!m.anchor && top + r.height > vh - 8) top = Math.max(8, lv.y - r.height);
      if (m.anchor && !m.above && top + r.height > vh - 8) top = Math.max(8, m.anchor.top - 6 - r.height);
    } else {
      const parent = panels[i - 1].getBoundingClientRect();
      const row = panels[i - 1].querySelectorAll<HTMLElement>("[data-row]")[lv.parentIndex]?.getBoundingClientRect();
      left = rtl ? parent.left - r.width + 4 : parent.right - 4;
      if (!rtl && left + r.width > vw - 8) left = parent.left - r.width + 4;
      if (rtl && left < 8) left = parent.right - 4;
      top = (row?.top ?? parent.top) - 6;
    }
    left = Math.max(8, Math.min(left, vw - r.width - 8));
    top = Math.max(8, Math.min(top, vh - r.height - 8));
    placed[i] = { left, top };
  }

  function openSub(levelIdx: number, rowIdx: number, item: MenuItem) {
    levels = levels.slice(0, levelIdx + 1);
    active = active.slice(0, levelIdx + 1);
    active[levelIdx] = rowIdx;
    if (item.submenu && !item.disabled) {
      levels.push({ source: item.submenu as MenuSource, x: 0, y: 0, parentIndex: rowIdx });
      active.push(-1);
      place(levels.length - 1);
    }
  }

  function hover(levelIdx: number, rowIdx: number, item: MenuItem) {
    clearTimeout(openTimer);
    active[levelIdx] = rowIdx;
    if (levels.length - 1 > levelIdx || item.submenu) {
      openTimer = setTimeout(() => openSub(levelIdx, rowIdx, item), item.submenu ? 120 : 200);
    }
  }

  function choose(levelIdx: number, rowIdx: number, item: MenuItem) {
    if (item.disabled || item.separator || item.header) return;
    if (item.submenu) {
      openSub(levelIdx, rowIdx, item);
      return;
    }
    item.onSelect?.();
    if (!item.keepOpen) closeMenu();
  }

  function selectable(items: MenuItem[], i: number) {
    const it = items[i];
    return it && !it.separator && !it.header && !it.disabled;
  }

  function onKey(e: KeyboardEvent) {
    if (!ui.menu) return;
    const li = levels.length - 1;
    const items = resolve(levels[li].source);
    const rtl = isRtl();
    const k = e.key;
    if (k === "Escape") {
      if (li > 0) {
        levels = levels.slice(0, li);
        active = active.slice(0, li);
      } else closeMenu();
    } else if (k === "ArrowDown" || k === "ArrowUp") {
      const d = k === "ArrowDown" ? 1 : -1;
      let i = active[li];
      for (let n = 0; n < items.length; n++) {
        i = (i + d + items.length) % items.length;
        if (selectable(items, i)) break;
      }
      active[li] = i;
    } else if (k === "Enter" || k === " ") {
      const i = active[li];
      if (i >= 0) choose(li, i, items[i]);
    } else if ((k === "ArrowRight" && !rtl) || (k === "ArrowLeft" && rtl)) {
      const i = active[li];
      if (i >= 0 && items[i]?.submenu) {
        openSub(li, i, items[i]);
        active[li + 1] = resolve(items[i].submenu as MenuSource).findIndex((_, j, arr) => selectable(arr, j));
      }
    } else if ((k === "ArrowLeft" && !rtl) || (k === "ArrowRight" && rtl)) {
      if (li > 0) {
        levels = levels.slice(0, li);
        active = active.slice(0, li);
      }
    } else return;
    e.preventDefault();
    e.stopPropagation();
  }

  function onDown(e: PointerEvent) {
    if (!ui.menu) return;
    if (panels.some((p) => p?.contains(e.target as Node))) return;
    closeMenu();
  }
</script>

<svelte:window onkeydown={onKey} onpointerdowncapture={onDown} onblur={() => closeMenu()} onresize={() => closeMenu()} />

{#if ui.menu}
  {#each levels as lv, li (li + ":" + lv.parentIndex)}
    {@const items = resolve(lv.source)}
    <div
      class="menu"
      role="menu"
      tabindex="-1"
      bind:this={panels[li]}
      style:left={(placed[li]?.left ?? -9999) + "px"}
      style:top={(placed[li]?.top ?? -9999) + "px"}
      style:min-width={(li === 0 && ui.menu.minWidth ? ui.menu.minWidth : 220) + "px"}
      oncontextmenu={(e) => e.preventDefault()}
    >
      {#each items as it, ri}
        {#if it.separator}
          <div class="sep"></div>
        {:else if it.header}
          <div class="header num">{it.header}</div>
        {:else}
          <button
            data-row
            class="row"
            class:active={active[li] === ri}
            class:disabled={it.disabled}
            class:danger={it.danger}
            role={it.radio ? "menuitemradio" : it.checked !== undefined ? "menuitemcheckbox" : "menuitem"}
            aria-checked={it.checked}
            onpointerenter={() => hover(li, ri, it)}
            onclick={() => choose(li, ri, it)}
          >
            <span class="lead">
              {#if it.checked}
                <Icon name="check" size={16} stroke={2.2} />
              {:else if it.icon}
                <Icon name={it.icon} size={17} />
              {/if}
            </span>
            <span class="label ellipsis">{it.label}</span>
            {#if it.hint}<span class="hint">{it.hint}</span>{/if}
            {#if it.submenu}<span class="chev"><Icon name="chevron-right" size={15} /></span>{/if}
          </button>
        {/if}
      {/each}
    </div>
  {/each}
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 1000;
    max-width: 380px;
    max-height: calc(100vh - 16px);
    overflow-y: auto;
    padding: 6px;
    border-radius: var(--r-l);
    background: var(--glass-strong);
    backdrop-filter: blur(24px) saturate(1.2);
    border: 1px solid var(--border-subtle);
    box-shadow: var(--shadow-pop);
    animation: menu-in var(--t) var(--ease);
  }
  @keyframes menu-in {
    from {
      opacity: 0;
      transform: translateY(4px) scale(0.98);
    }
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 34px;
    padding: 0 10px 0 8px;
    border-radius: 8px;
    text-align: start;
    color: var(--text-primary);
    font-size: 13.5px;
  }
  .row.active {
    background: var(--bg-hover);
  }
  .row.disabled {
    color: var(--text-disabled);
  }
  .row.danger {
    color: var(--danger);
  }
  .lead {
    width: 18px;
    display: grid;
    place-items: center;
    color: var(--text-secondary);
  }
  .row[aria-checked="true"] .lead {
    color: var(--text-primary);
  }
  .label {
    flex: 1;
  }
  .hint {
    color: var(--text-disabled);
    font-size: 12px;
    margin-inline-start: 12px;
    white-space: nowrap;
  }
  .chev {
    color: var(--text-secondary);
    margin-inline-end: -4px;
  }
  .sep {
    height: 1px;
    margin: 6px 4px;
    background: var(--border-subtle);
  }
  .header {
    padding: 6px 10px 4px 36px;
    font-size: 11.5px;
    color: var(--text-secondary);
  }
</style>
