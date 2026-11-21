// Interface state shared by components: toasts, menus, navigation.

export interface MenuItem {
  label?: string;
  icon?: string;
  hint?: string; // shortcut or value shown on the trailing side
  checked?: boolean;
  radio?: boolean;
  disabled?: boolean;
  danger?: boolean;
  separator?: boolean;
  header?: string;
  submenu?: MenuItem[] | (() => MenuItem[]);
  onSelect?: () => void;
  keepOpen?: boolean;
}

export type MenuSource = MenuItem[] | (() => MenuItem[]);

export interface MenuRequest {
  items: MenuSource;
  x: number;
  y: number;
  anchor?: DOMRect;
  align?: "start" | "end" | "center";
  above?: boolean;
  minWidth?: number;
  id: number;
}

export interface Toast {
  id: number;
  text: string;
  icon?: string;
  action?: { label: string; run: () => void };
  ms: number;
}

export const ui = $state({
  menu: null as MenuRequest | null,
  toasts: [] as Toast[],
});

let seq = 1;

export function openMenu(items: MenuSource, at: { x: number; y: number } | { anchor: DOMRect; align?: "start" | "end" | "center"; above?: boolean }, minWidth?: number) {
  if ("anchor" in at) {
    ui.menu = { items, x: 0, y: 0, anchor: at.anchor, align: at.align ?? "start", above: at.above, minWidth, id: seq++ };
  } else {
    ui.menu = { items, x: at.x, y: at.y, minWidth, id: seq++ };
  }
}

export function closeMenu() {
  ui.menu = null;
}

export function toast(text: string, opts: { icon?: string; action?: Toast["action"]; ms?: number } = {}) {
  const t: Toast = { id: seq++, text, icon: opts.icon, action: opts.action, ms: opts.ms ?? 3200 };
  ui.toasts = [...ui.toasts.slice(-2), t];
  setTimeout(() => dismissToast(t.id), t.ms);
}

export function dismissToast(id: number) {
  ui.toasts = ui.toasts.filter((t) => t.id !== id);
}
