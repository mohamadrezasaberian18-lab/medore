// Page navigation inside the main window.
export type Page = "home" | "movies" | "shows" | "show" | "videos" | "music" | "search" | "settings";

export const nav = $state({
  page: "home" as Page,
  param: "" as string,
  history: [] as { page: Page; param: string; scroll: number }[],
  query: "",
  scrollEl: null as HTMLElement | null,
});

export function go(page: Page, param = "") {
  if (nav.page === page && nav.param === param) return;
  nav.history = [...nav.history.slice(-30), { page: nav.page, param: nav.param, scroll: nav.scrollEl?.scrollTop ?? 0 }];
  nav.page = page;
  nav.param = param;
  queueMicrotask(() => nav.scrollEl?.scrollTo({ top: 0 }));
}

export function back() {
  const prev = nav.history.at(-1);
  if (!prev) return;
  nav.history = nav.history.slice(0, -1);
  nav.page = prev.page;
  nav.param = prev.param;
  queueMicrotask(() => nav.scrollEl?.scrollTo({ top: prev.scroll }));
}

export function root(page: Page) {
  nav.history = [];
  nav.page = page;
  nav.param = "";
  queueMicrotask(() => nav.scrollEl?.scrollTo({ top: 0 }));
}
