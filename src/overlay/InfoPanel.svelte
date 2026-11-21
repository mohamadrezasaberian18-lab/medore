<script lang="ts">
  import Icon from "../components/Icon.svelte";
  import { player, P } from "../lib/player.svelte";
  import { api, fileName, dirName } from "../lib/api";
  import { t, num } from "../lib/i18n.svelte";
  import { bytes, bitrate, resolutionLabel, clock } from "../lib/format";
  import { trackLabel } from "../lib/menus";

  let { onclose }: { onclose: () => void } = $props();

  const vp = $derived((player.p["video-params"] ?? {}) as Record<string, any>);
  const ap = $derived((player.p["audio-params"] ?? {}) as Record<string, any>);
  const vtrack = $derived(P.tracks.find((x) => x.type === "video" && x.selected));
  const atrack = $derived(P.tracks.find((x) => x.type === "audio" && x.selected));
  const stracks = $derived(P.tracks.filter((x) => x.type === "sub"));
  const strack = $derived(stracks.find((x) => x.selected));

  function hdr(): string {
    const g = vp.gamma;
    if (g === "pq") return vp.light === "dolbyvision" ? "Dolby Vision" : "HDR10";
    if (g === "hlg") return "HLG";
    return "SDR";
  }
  function channels(n?: number): string {
    if (!n) return "";
    return n === 6 ? "5.1" : n === 8 ? "7.1" : n === 2 ? t("Stereo") : n === 1 ? t("Mono") : num(n) + " ch";
  }

  const groups = $derived.by(() => {
    const out: { title: string; rows: [string, string][] }[] = [];
    const file: [string, string][] = [
      [t("Name"), fileName(P.path)],
      [t("Folder"), dirName(P.path)],
      [t("Size"), bytes(player.p["file-size"])],
      [t("Container"), String(player.p["file-format"] ?? "").toUpperCase()],
      [t("Length"), clock(P.duration)],
    ];
    out.push({ title: t("File"), rows: file.filter((r) => r[1]) });
    if (vtrack && !vtrack.albumart && !vtrack.image) {
      const w = vp.dw || vtrack["demux-w"] || 0;
      const h = vp.dh || vtrack["demux-h"] || 0;
      const fps = player.p["estimated-vf-fps"] || player.p["container-fps"];
      const hw = player.p["hwdec-current"];
      out.push({
        title: t("Video"),
        rows: (
          [
            [t("Codec"), String(player.p["video-format"] ?? vtrack.codec ?? "").toUpperCase()],
            [t("Resolution"), w && h ? `${num(w)} × ${num(h)} (${resolutionLabel(w, h)})` : ""],
            [t("Frame rate"), fps ? num(Number(fps).toFixed(3).replace(/\.?0+$/, "")) + " fps" : ""],
            [t("Dynamic range"), hdr()],
            [t("Colour"), [vp.primaries, vp.colormatrix, vp.pixelformat].filter(Boolean).join(" · ")],
            [t("Decoding"), hw && hw !== "no" ? t("Hardware ({name})", { name: hw }) : t("Software")],
            [t("Bitrate"), bitrate(player.p["video-bitrate"])],
            [t("Dropped frames"), num((player.p["frame-drop-count"] ?? 0) + (player.p["decoder-frame-drop-count"] ?? 0))],
          ] as [string, string][]
        ).filter((r) => r[1]),
      });
    }
    if (atrack) {
      out.push({
        title: t("Audio"),
        rows: (
          [
            [t("Track"), trackLabel(atrack, 0)],
            [t("Codec"), String(player.p["audio-codec-name"] ?? atrack.codec ?? "").toUpperCase()],
            [t("Channels"), channels(ap["channel-count"] ?? atrack["demux-channel-count"])],
            [t("Sample rate"), ap.samplerate ? num((ap.samplerate / 1000).toFixed(1).replace(/\.0$/, "")) + " kHz" : ""],
            [t("Bitrate"), bitrate(player.p["audio-bitrate"])],
          ] as [string, string][]
        ).filter((r) => r[1]),
      });
    }
    out.push({
      title: t("Subtitles"),
      rows: [
        [t("Showing"), strack ? trackLabel(strack, stracks.indexOf(strack)) : t("Off")],
        [t("Available"), num(stracks.length)],
      ],
    });
    return out;
  });
</script>

<aside class="panel" role="dialog" aria-label={t("Media info")}>
  <header>
    <div class="title">{t("Media info")}</div>
    <div class="tools">
      <button class="icon-btn" title={t("Show in folder")} onclick={() => api.reveal(P.path)}><Icon name="folder-open" size={18} /></button>
      <button class="icon-btn" title={t("Close")} onclick={onclose}><Icon name="x" size={18} /></button>
    </div>
  </header>
  <div class="body">
    {#each groups as g}
      <section>
        <div class="overline">{g.title}</div>
        <dl>
          {#each g.rows as [k, v]}
            <dt>{k}</dt>
            <dd class="num" dir="auto">{v}</dd>
          {/each}
        </dl>
      </section>
    {/each}
  </div>
</aside>

<style>
  .panel {
    position: absolute;
    top: 0;
    bottom: 0;
    inset-inline-end: 0;
    width: 360px;
    max-width: 92vw;
    display: flex;
    flex-direction: column;
    background: var(--glass);
    backdrop-filter: blur(28px) saturate(1.2);
    border-inline-start: 1px solid var(--hairline);
    animation: slide-in var(--t-slow) var(--ease);
  }
  @keyframes slide-in {
    from {
      transform: translateX(24px);
      opacity: 0;
    }
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 18px 12px 6px 18px;
  }
  .title {
    font-size: 17px;
    font-weight: 600;
  }
  .tools {
    display: flex;
    gap: 2px;
  }
  .body {
    flex: 1;
    overflow-y: auto;
    padding: 8px 18px 20px;
    user-select: text;
  }
  section {
    margin-top: 16px;
  }
  dl {
    display: grid;
    grid-template-columns: 112px 1fr;
    gap: 6px 12px;
    margin: 8px 0 0;
    font-size: 13px;
  }
  dt {
    color: var(--text-secondary);
  }
  dd {
    margin: 0;
    word-break: break-word;
  }
</style>
