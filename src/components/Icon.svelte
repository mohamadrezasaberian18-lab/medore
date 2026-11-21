<script lang="ts" module>
  const raw = import.meta.glob("../icons/*.svg", { query: "?raw", import: "default", eager: true }) as Record<string, string>;
  const icons: Record<string, string> = {};
  for (const [path, svg] of Object.entries(raw)) {
    const name = path.split("/").pop()!.replace(".svg", "");
    // keep only the drawing; size and stroke come from the component
    const inner = svg.replace(/<!--[\s\S]*?-->/g, "").replace(/^[\s\S]*?<svg[^>]*>/, "").replace(/<\/svg>\s*$/, "");
    icons[name] = inner;
  }
  export const MIRRORED = new Set(["arrow-left", "chevron-left", "chevron-right", "skip-forward", "skip-back", "list-start", "undo-2", "rotate-ccw", "rotate-cw", "external-link"]);
</script>

<script lang="ts">
  let { name, size = 20, stroke = 1.75, mirror = true, fill = false, class: cls = "" }: { name: string; size?: number; stroke?: number; mirror?: boolean; fill?: boolean; class?: string } = $props();
  const body = $derived(icons[name] ?? icons["square"] ?? "");
</script>

<svg
  class="icon {cls}"
  class:mirror={mirror && MIRRORED.has(name)}
  class:filled={fill}
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true">{@html body}</svg
>

<style>
  .icon {
    flex: none;
    display: block;
  }
  .filled :global(path),
  .filled :global(polygon) {
    fill: currentColor;
  }
  :global([dir="rtl"]) .mirror {
    transform: scaleX(-1);
  }
</style>
