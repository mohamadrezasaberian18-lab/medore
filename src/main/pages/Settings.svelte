<script lang="ts">
  import { onMount } from "svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import Icon from "../../components/Icon.svelte";
  import Toggle from "../../components/Toggle.svelte";
  import Slider from "../../components/Slider.svelte";
  import Select from "../../components/Select.svelte";
  import Segmented from "../../components/Segmented.svelte";
  import { settings, set } from "../../lib/settings.svelte";
  import { api } from "../../lib/api";
  import { t, num, i18n } from "../../lib/i18n.svelte";
  import { languageName, LANGUAGE_CODES } from "../../lib/languages";
  import { openMenu, toast } from "../../lib/ui.svelte";
  import { SHORTCUTS } from "../../lib/keys";
  import { player } from "../../lib/player.svelte";
  import { lib } from "../../lib/library.svelte";

  type Section = "general" | "playback" | "subtitles" | "audio" | "video" | "library" | "appearance" | "keyboard" | "about";
  let section = $state<Section>("general");
  let fonts = $state<string[]>([]);
  let info = $state<any>({});
  let checking = $state(false);
  let updateMsg = $state("");
  const v = $derived(settings.v);

  const sections: { id: Section; icon: string; label: string }[] = $derived([
    { id: "general", icon: "sliders-horizontal", label: t("General") },
    { id: "playback", icon: "circle-play", label: t("Playback") },
    { id: "subtitles", icon: "captions", label: t("Subtitles") },
    { id: "audio", icon: "audio-lines", label: t("Audio") },
    { id: "video", icon: "monitor", label: t("Video") },
    { id: "library", icon: "folder", label: t("Library") },
    { id: "appearance", icon: "sun-moon", label: t("Appearance") },
    { id: "keyboard", icon: "keyboard", label: t("Keyboard") },
    { id: "about", icon: "info", label: t("About MEDORE") },
  ]);

  onMount(async () => {
    info = await api.appInfo();
    api.fonts().then((f) => (fonts = f));
  });

  async function addFolder() {
    const d = await openDialog({ directory: true, multiple: false });
    if (typeof d === "string") await api.libraryAddFolder(d);
  }
  async function pickShotDir() {
    const d = await openDialog({ directory: true, multiple: false });
    if (typeof d === "string") set("playback.screenshot_dir", d);
  }
  async function checkUpdate() {
    checking = true;
    updateMsg = "";
    try {
      const r = await api.updateCheck();
      updateMsg = r.available ? t("Version {v} is ready. It installs when you restart MEDORE.", { v: r.version }) : t("MEDORE is up to date.");
    } catch (e) {
      updateMsg = t("Could not check for updates. Check the internet connection.");
    }
    checking = false;
  }

  function langList(key: string): string[] {
    return (v[key] ?? []) as string[];
  }
  function addLang(key: string, e: MouseEvent) {
    const cur = langList(key);
    openMenu(
      LANGUAGE_CODES.filter((c) => !cur.includes(c))
        .map((c) => ({ code: c, name: languageName(c) }))
        .sort((a, b) => a.name.localeCompare(b.name, i18n.lang))
        .map((l) => ({ label: l.name, onSelect: () => set(key, [...cur, l.code]) })),
      { anchor: (e.currentTarget as HTMLElement).getBoundingClientRect(), align: "start" },
    );
  }
  function moveLang(key: string, i: number, d: number) {
    const l = [...langList(key)];
    const j = i + d;
    if (j < 0 || j >= l.length) return;
    [l[i], l[j]] = [l[j], l[i]];
    set(key, l);
  }
  function removeLang(key: string, i: number) {
    const l = [...langList(key)];
    l.splice(i, 1);
    set(key, l);
  }

  const fontOptions = $derived([...new Set(["Calibri", "Segoe UI", "Arial", "Tahoma", "Vazirmatn", ...fonts])].map((f) => ({ value: f, label: f + (f === "Calibri" ? " — " + t("default") : "") })));
  const colors = ["#FFFFFF", "#FFF4C2", "#E0E0E0", "#FFE066", "#9BE2B0", "#A0D8FF"];
  const devices = $derived(((player.p["audio-device-list"] ?? []) as { name: string; description: string }[]).map((d) => ({ value: d.name, label: d.name === "auto" ? t("System default") : d.description.replace(/\s*\(WASAPI\)/i, "") })));
  const subShadow = $derived.by(() => {
    const b = Number(v["subs.border_size"] ?? 2.4);
    const c = v["subs.border_color"] ?? "#000";
    const px = Math.max(0, b * 0.9);
    const shadows: string[] = [];
    for (let a = 0; a < 16; a++) shadows.push(`${(Math.cos((a / 16) * Math.PI * 2) * px).toFixed(1)}px ${(Math.sin((a / 16) * Math.PI * 2) * px).toFixed(1)}px 0 ${c}`);
    const s = Number(v["subs.shadow"] ?? 0.8);
    if (s > 0) shadows.push(`${s * 1.5}px ${s * 1.5}px ${s * 2}px rgba(0,0,0,.8)`);
    return shadows.join(",");
  });
</script>

{#snippet row(label: string, help: string)}
  <div class="label">
    <div class="lt">{label}</div>
    {#if help}<div class="lh muted">{help}</div>{/if}
  </div>
{/snippet}

<div class="settings">
  <nav class="secnav">
    <h1>{t("Settings")}</h1>
    {#each sections as s (s.id)}
      <button class="sec" class:active={section === s.id} onclick={() => (section = s.id)}>
        <Icon name={s.icon} size={18} />
        <span>{s.label}</span>
      </button>
    {/each}
  </nav>

  <div class="body">
    {#if section === "general"}
      <h2>{t("General")}</h2>
      <div class="card">
        <div class="item">{@render row(t("Language"), t("The language of menus and buttons."))}
          <Select value={v["general.language"]} onchange={(x) => set("general.language", x)} options={[{ value: "auto", label: t("Same as Windows") }, { value: "en", label: "English" }, { value: "fa", label: "فارسی" }]} />
        </div>
        <div class="item">{@render row(t("Start page"), t("What MEDORE shows when it opens."))}
          <Select value={v["general.start_page"]} onchange={(x) => set("general.start_page", x)} options={[{ value: "home", label: t("Home") }, { value: "movies", label: t("Movies") }, { value: "shows", label: t("TV Shows") }, { value: "videos", label: t("Videos") }, { value: "music", label: t("Music") }]} />
        </div>
      </div>
      <h3>{t("Updates")}</h3>
      <div class="card">
        <div class="item">{@render row(t("Check for updates automatically"), t("New versions download in the background and install when MEDORE restarts."))}
          <Toggle checked={!!v["updates.auto_check"]} onchange={(x) => set("updates.auto_check", x)} />
        </div>
        <div class="item">{@render row(t("Install updates when closing"), t("If you keep watching, the update installs the next time you close MEDORE."))}
          <Toggle checked={!!v["updates.auto_install"]} onchange={(x) => set("updates.auto_install", x)} />
        </div>
        <div class="item">{@render row(t("Version {v}", { v: info.version ?? "" }), updateMsg)}
          <button class="btn secondary sm" disabled={checking} onclick={checkUpdate}><Icon name="refresh-cw" size={15} class={checking ? "spin" : ""} />{t("Check now")}</button>
        </div>
      </div>
    {:else if section === "playback"}
      <h2>{t("Playback")}</h2>
      <div class="card">
        <div class="item">{@render row(t("Resume where I stopped"), t("When you open something you watched before."))}
          <Segmented value={v["playback.resume"]} onchange={(x) => set("playback.resume", x)} options={[{ value: "auto", label: t("Always") }, { value: "ask", label: t("Ask") }, { value: "never", label: t("Never") }]} />
        </div>
        <div class="item">{@render row(t("Play the next episode automatically"), t("With a countdown you can cancel."))}
          <Toggle checked={!!v["playback.autoplay_next"]} onchange={(x) => set("playback.autoplay_next", x)} />
        </div>
        <div class="item">{@render row(t("Countdown before the next episode"), num(v["playback.up_next_seconds"] ?? 10) + " " + t("seconds"))}
          <Slider value={v["playback.up_next_seconds"] ?? 10} min={0} max={30} width={180} onchange={(x) => set("playback.up_next_seconds", x)} />
        </div>
        <div class="item">{@render row(t("Queue the rest of the folder"), t("Opening one episode, video or song plays the others in the folder after it, in order."))}
          <Toggle checked={!!v["playback.auto_queue_folder"]} onchange={(x) => set("playback.auto_queue_folder", x)} />
        </div>
      </div>
      <h3>{t("Mouse and keyboard")}</h3>
      <div class="card">
        <div class="item">{@render row(t("Click the video to pause"), "")}
          <Toggle checked={!!v["playback.click_to_pause"]} onchange={(x) => set("playback.click_to_pause", x)} />
        </div>
        <div class="item">{@render row(t("Double-click for full screen"), "")}
          <Toggle checked={!!v["playback.double_click_fullscreen"]} onchange={(x) => set("playback.double_click_fullscreen", x)} />
        </div>
        <div class="item">{@render row(t("Mouse wheel"), "")}
          <Segmented value={v["playback.wheel"]} onchange={(x) => set("playback.wheel", x)} options={[{ value: "volume", label: t("Volume") }, { value: "seek", label: t("Seek") }]} />
        </div>
        <div class="item">{@render row(t("Arrow keys jump"), t("{n} seconds (with Shift: {m} seconds)", { n: v["playback.seek_step"] ?? 5, m: v["playback.seek_step_large"] ?? 30 }))}
          <Slider value={v["playback.seek_step"] ?? 5} min={1} max={60} width={180} onchange={(x) => set("playback.seek_step", x)} />
        </div>
        <div class="item">{@render row(t("Pause when minimized"), "")}
          <Toggle checked={!!v["playback.pause_on_minimize"]} onchange={(x) => set("playback.pause_on_minimize", x)} />
        </div>
        <div class="item">{@render row(t("Remember volume"), "")}
          <Toggle checked={!!v["playback.remember_volume"]} onchange={(x) => set("playback.remember_volume", x)} />
        </div>
      </div>
      <h3>{t("Screenshots")}</h3>
      <div class="card">
        <div class="item">{@render row(t("Format"), "")}
          <Segmented value={v["playback.screenshot_format"]} onchange={(x) => set("playback.screenshot_format", x)} options={[{ value: "png", label: "PNG" }, { value: "jpg", label: "JPG" }, { value: "webp", label: "WebP" }]} />
        </div>
        <div class="item">{@render row(t("Folder"), v["playback.screenshot_dir"] || t("Pictures › MEDORE"))}
          <button class="btn secondary sm" onclick={pickShotDir}>{t("Choose…")}</button>
        </div>
      </div>
    {:else if section === "subtitles"}
      <h2>{t("Subtitles")}</h2>
      <div class="preview">
        <div
          class="sample"
          style:font-family={`"${v["subs.font"] || "Calibri"}", Calibri, "Vazirmatn", sans-serif`}
          style:font-size={Math.round(Number(v["subs.size"] ?? 46) * 0.62) + "px"}
          style:font-weight={v["subs.bold"] ? 700 : 400}
          style:color={v["subs.color"]}
          style:text-shadow={subShadow}
          style:background={Number(v["subs.back_opacity"]) > 0 ? `rgba(11,11,11,${Number(v["subs.back_opacity"]) / 100})` : "transparent"}
        >
          {t("This is how your subtitles look.")}<br /><span dir="rtl">زیرنویس‌ها این‌طور دیده می‌شوند.</span>
        </div>
      </div>
      <div class="card">
        <div class="item">{@render row(t("Load subtitles automatically"), t("Subtitle files next to the video are found by themselves, even with different names, in Subs folders and inside .zip files."))}
          <Toggle checked={!!v["subs.auto_load"]} onchange={(x) => set("subs.auto_load", x)} />
        </div>
        <div class="item">{@render row(t("Choose the subtitle track for me"), t("Uses your languages below. Films in your first language get no subtitles."))}
          <Toggle checked={!!v["subs.auto_select"]} onchange={(x) => set("subs.auto_select", x)} />
        </div>
        <div class="item col">{@render row(t("My subtitle languages"), t("In order of preference."))}
          <div class="langs">
            {#each langList("subs.languages") as l, i (l)}
              <div class="lang">
                <span class="ln num">{num(i + 1)}</span>{languageName(l)}
                <button class="mini" aria-label={t("Move up")} disabled={i === 0} onclick={() => moveLang("subs.languages", i, -1)}><Icon name="chevron-up" size={14} /></button>
                <button class="mini" aria-label={t("Remove")} onclick={() => removeLang("subs.languages", i)}><Icon name="x" size={14} /></button>
              </div>
            {/each}
            <button class="btn ghost sm" onclick={(e) => addLang("subs.languages", e)}><Icon name="plus" size={15} />{t("Add language")}</button>
          </div>
        </div>
      </div>
      <h3>{t("Style")}</h3>
      <div class="card">
        <div class="item">{@render row(t("Font"), t("Calibri, the Windows font, is the default. Persian text uses a matching Persian font automatically."))}
          <Select value={v["subs.font"]} onchange={(x) => set("subs.font", x)} options={fontOptions} width={240} />
        </div>
        <div class="item">{@render row(t("Size"), num(v["subs.size"] ?? 46))}
          <Slider value={v["subs.size"] ?? 46} min={20} max={90} width={200} onchange={(x) => set("subs.size", x)} />
        </div>
        <div class="item">{@render row(t("Bold"), "")}
          <Toggle checked={!!v["subs.bold"]} onchange={(x) => set("subs.bold", x)} />
        </div>
        <div class="item">{@render row(t("Text colour"), "")}
          <div class="swatches">
            {#each colors as c}
              <button class="sw" class:on={(v["subs.color"] ?? "").toUpperCase() === c} style:background={c} aria-label={c} onclick={() => set("subs.color", c)}></button>
            {/each}
          </div>
        </div>
        <div class="item">{@render row(t("Outline"), num(Number(v["subs.border_size"] ?? 2.4).toFixed(1)))}
          <Slider value={v["subs.border_size"] ?? 2.4} min={0} max={6} step={0.2} width={200} onchange={(x) => set("subs.border_size", x)} />
        </div>
        <div class="item">{@render row(t("Shadow"), num(Number(v["subs.shadow"] ?? 0.8).toFixed(1)))}
          <Slider value={v["subs.shadow"] ?? 0.8} min={0} max={4} step={0.2} width={200} onchange={(x) => set("subs.shadow", x)} />
        </div>
        <div class="item">{@render row(t("Background box"), num(v["subs.back_opacity"] ?? 0) + "%")}
          <Slider value={v["subs.back_opacity"] ?? 0} min={0} max={100} step={5} width={200} onchange={(x) => set("subs.back_opacity", x)} />
        </div>
        <div class="item">{@render row(t("Use my style for styled (ASS) subtitles too"), t("Off keeps the colours and fonts the subtitle maker chose."))}
          <Toggle checked={!!v["subs.override_ass"]} onchange={(x) => set("subs.override_ass", x)} />
        </div>
      </div>
      <h3>{t("Persian and Arabic")}</h3>
      <div class="card">
        <div class="item">{@render row(t("Fix Persian letters"), t("Old Persian subtitles use Arabic ي and ك; MEDORE shows ی and ک instead. Your files are never changed."))}
          <Toggle checked={!!v["subs.persian_fix"]} onchange={(x) => set("subs.persian_fix", x)} />
        </div>
        <div class="item">{@render row(t("Right-to-left layout for styled subtitles"), t("Turn on if punctuation in styled (ASS) Persian subtitles appears on the wrong side."))}
          <Toggle checked={!!v["subs.rtl_ass_auto"]} onchange={(x) => set("subs.rtl_ass_auto", x)} />
        </div>
        <div class="item">{@render row(t("Text encoding"), t("Automatic detection reads old Windows-1256 Persian files correctly."))}
          <Select
            value={v["subs.encoding"]}
            onchange={(x) => set("subs.encoding", x)}
            options={[
              { value: "auto", label: t("Automatic") },
              { value: "utf-8", label: "UTF-8" },
              { value: "cp1256", label: t("Persian / Arabic (Windows-1256)") },
              { value: "cp1252", label: t("Western (Windows-1252)") },
              { value: "cp1251", label: t("Cyrillic (Windows-1251)") },
            ]}
          />
        </div>
      </div>
    {:else if section === "audio"}
      <h2>{t("Audio")}</h2>
      <div class="card">
        <div class="item">{@render row(t("Output device"), "")}
          <Select value={v["audio.device"]} onchange={(x) => (set("audio.device", x), api.set("audio-device", x))} options={devices.length ? devices : [{ value: "auto", label: t("System default") }]} width={260} />
        </div>
        <div class="item col">{@render row(t("My audio languages"), t("When a file has several audio tracks. Empty means the original language."))}
          <div class="langs">
            {#each langList("audio.languages") as l, i (l)}
              <div class="lang">
                <span class="ln num">{num(i + 1)}</span>{languageName(l)}
                <button class="mini" aria-label={t("Move up")} disabled={i === 0} onclick={() => moveLang("audio.languages", i, -1)}><Icon name="chevron-up" size={14} /></button>
                <button class="mini" aria-label={t("Remove")} onclick={() => removeLang("audio.languages", i)}><Icon name="x" size={14} /></button>
              </div>
            {/each}
            <button class="btn ghost sm" onclick={(e) => addLang("audio.languages", e)}><Icon name="plus" size={15} />{t("Add language")}</button>
          </div>
        </div>
        <div class="item">{@render row(t("Normalize volume"), t("Evens out quiet dialogue and loud scenes."))}
          <Toggle checked={!!v["audio.normalize"]} onchange={(x) => set("audio.normalize", x)} />
        </div>
        <div class="item">{@render row(t("Speakers"), "")}
          <Segmented value={v["audio.channels"]} onchange={(x) => set("audio.channels", x)} options={[{ value: "auto-safe", label: t("Automatic") }, { value: "stereo", label: t("Stereo") }]} />
        </div>
        <div class="item">{@render row(t("Pass-through to receiver"), t("Sends Dolby and DTS untouched to an AV receiver over HDMI or optical."))}
          <Toggle checked={!!v["audio.passthrough"]} onchange={(x) => set("audio.passthrough", x)} />
        </div>
        <div class="item">{@render row(t("Exclusive mode"), t("Lets MEDORE use the sound device alone, for bit-perfect output."))}
          <Toggle checked={!!v["audio.exclusive"]} onchange={(x) => set("audio.exclusive", x)} />
        </div>
      </div>
    {:else if section === "video"}
      <h2>{t("Video")}</h2>
      <div class="card">
        <div class="item">{@render row(t("Picture quality"), t("Higher quality uses more of the graphics card."))}
          <Segmented value={v["video.quality"]} onchange={(x) => set("video.quality", x)} options={[{ value: "fast", label: t("Power saving") }, { value: "balanced", label: t("Balanced") }, { value: "high", label: t("Best") }]} />
        </div>
        <div class="item">{@render row(t("Hardware decoding"), t("Uses the graphics card to decode video. Smoother and cooler."))}
          <Select value={v["video.hwdec"]} onchange={(x) => set("video.hwdec", x)} options={[{ value: "auto-safe", label: t("Automatic") }, { value: "d3d11va", label: "D3D11" }, { value: "nvdec", label: "NVIDIA" }, { value: "auto-copy", label: t("Automatic (compatible)") }, { value: "no", label: t("Off") }]} />
        </div>
        <div class="item">{@render row(t("HDR"), t("Automatic sends HDR to HDR screens and tone-maps it for others."))}
          <Segmented value={v["video.hdr"]} onchange={(x) => set("video.hdr", x)} options={[{ value: "auto", label: t("Automatic") }, { value: "tonemap", label: t("Always tone-map") }, { value: "passthrough", label: t("Pass through") }]} />
        </div>
        <div class="item">{@render row(t("Smooth motion"), t("Removes judder on screens faster than the video. Uses more power."))}
          <Toggle checked={!!v["video.smooth_motion"]} onchange={(x) => set("video.smooth_motion", x)} />
        </div>
        <div class="item">{@render row(t("Reduce banding"), t("Smooths colour steps in skies and dark scenes."))}
          <Toggle checked={!!v["video.deband"]} onchange={(x) => set("video.deband", x)} />
        </div>
        <div class="item">{@render row(t("Deinterlace"), "")}
          <Segmented value={v["video.deinterlace"]} onchange={(x) => set("video.deinterlace", x)} options={[{ value: "auto", label: t("Automatic") }, { value: "yes", label: t("On") }, { value: "no", label: t("Off") }]} />
        </div>
      </div>
      <h3>{t("Advanced")}</h3>
      <div class="card">
        <div class="item col">{@render row(t("Extra engine options"), t("For experts: one mpv option per line, e.g. deband-iterations=2."))}
          <textarea class="field area" spellcheck="false" dir="ltr" value={v["advanced.mpv_options"] ?? ""} onchange={(e) => set("advanced.mpv_options", (e.target as HTMLTextAreaElement).value)}></textarea>
        </div>
      </div>
    {:else if section === "library"}
      <h2>{t("Library")}</h2>
      <div class="card">
        <div class="item col">{@render row(t("Folders"), t("MEDORE watches these folders for movies, shows, videos and music."))}
          <div class="folders">
            {#each (v["library.folders"] ?? []) as f (f)}
              <div class="folder">
                <Icon name="folder" size={18} />
                <span class="ellipsis" dir="ltr">{f}</span>
                <button class="mini" title={t("Open")} onclick={() => api.reveal(f)}><Icon name="folder-open" size={15} /></button>
                <button class="mini" title={t("Remove")} onclick={() => api.libraryRemoveFolder(f)}><Icon name="x" size={15} /></button>
              </div>
            {/each}
            <div class="facts">
              <button class="btn secondary sm" onclick={addFolder}><Icon name="folder-plus" size={15} />{t("Add folder")}</button>
              <button class="btn ghost sm" onclick={() => (api.libraryRescan(), toast(t("Looking for new files…"), { icon: "refresh-cw" }))}><Icon name="refresh-cw" size={15} />{t("Scan now")}</button>
              <span class="muted small num">{t("{n} files", { n: lib.items.length })}</span>
            </div>
          </div>
        </div>
        <div class="item">{@render row(t("Download posters"), t("Fetches movie and show posters from the internet. Only the title is sent."))}
          <Toggle checked={!!v["library.posters_online"]} onchange={(x) => set("library.posters_online", x)} />
        </div>
        <div class="item">{@render row(t("Make thumbnails"), t("Picks a frame from each video (skipping black frames)."))}
          <Toggle checked={!!v["library.thumbnails"]} onchange={(x) => set("library.thumbnails", x)} />
        </div>
        <div class="item">{@render row(t("Look for new files at start"), "")}
          <Toggle checked={!!v["library.scan_on_start"]} onchange={(x) => set("library.scan_on_start", x)} />
        </div>
        <div class="item">{@render row(t("Watch history"), t("Clears resume points and watched marks."))}
          <button class="btn secondary sm danger" onclick={() => (api.libraryClearHistory(), toast(t("Watch history cleared"), { icon: "history" }))}>{t("Clear history")}</button>
        </div>
      </div>
    {:else if section === "appearance"}
      <h2>{t("Appearance")}</h2>
      <div class="card">
        <div class="item">{@render row(t("Animations"), t("“Like Windows” follows Settings › Accessibility › Visual effects."))}
          <Segmented value={v["appearance.reduce_motion"]} onchange={(x) => set("appearance.reduce_motion", x)} options={[{ value: "system", label: t("Like Windows") }, { value: "off", label: t("On") }, { value: "on", label: t("Reduced") }]} />
        </div>
        <div class="item">{@render row(t("Hide player controls after"), num(Number(v["appearance.controls_timeout"] ?? 2.5).toFixed(1)) + " " + t("seconds"))}
          <Slider value={v["appearance.controls_timeout"] ?? 2.5} min={1} max={8} step={0.5} width={200} onchange={(x) => set("appearance.controls_timeout", x)} />
        </div>
        <div class="item">{@render row(t("Show remaining time"), t("Instead of the total length."))}
          <Toggle checked={!!v["appearance.show_remaining"]} onchange={(x) => set("appearance.show_remaining", x)} />
        </div>
        <div class="item">{@render row(t("App icon"), t("The MEDORE icon follows Windows' light or dark mode by itself."))}
          <div class="icons">
            <img src="/brand/medore-icon-dark.svg" alt="" width="40" height="40" />
            <img src="/brand/medore-icon-light.svg" alt="" width="40" height="40" />
          </div>
        </div>
      </div>
    {:else if section === "keyboard"}
      <h2>{t("Keyboard")}</h2>
      <div class="card keys">
        {#each SHORTCUTS as [k, w]}
          <div class="kr"><span class="kk" dir="ltr">{k}</span><span>{t(w)}</span></div>
        {/each}
      </div>
    {:else if section === "about"}
      <div class="about">
        <img src="/brand/medore-icon-dark.svg" alt="" width="96" height="96" />
        <div class="aname">MEDORE</div>
        <div class="muted num">{t("Version {v}", { v: info.version ?? "" })}{info.portable ? " · " + t("portable") : ""}</div>
        <p class="muted">{t("A media player that plays everything, finds subtitles by itself and keeps your library tidy.")}</p>
        <div class="acts">
          <button class="btn secondary sm" onclick={() => api.openUrl("https://github.com/mohamadrezasaberian18-lab/medore")}><Icon name="external-link" size={15} />{t("Project page")}</button>
          <button class="btn secondary sm" onclick={() => api.reveal(info.dataDir)}><Icon name="folder-open" size={15} />{t("Settings folder")}</button>
        </div>
        <div class="credits muted">
          {t("Free software under the GNU GPL v3. Playback by libmpv and FFmpeg. Icons by Lucide. Persian font Vazirmatn.")}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .settings {
    display: grid;
    grid-template-columns: 240px minmax(0, 1fr);
    gap: 24px;
    padding: 18px 32px 0;
    max-width: 1200px;
  }
  .secnav {
    position: sticky;
    top: 0;
    align-self: start;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .secnav h1 {
    margin: 6px 12px 14px;
    font-family: var(--font-display);
    font-size: 28px;
  }
  .sec {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 40px;
    padding: 0 12px;
    border-radius: var(--r-m);
    color: var(--text-secondary);
    text-align: start;
  }
  .sec:hover {
    background: var(--bg-surface);
    color: var(--text-primary);
  }
  .sec.active {
    background: var(--bg-elevated);
    color: var(--text-primary);
    font-weight: 600;
  }
  .body {
    min-width: 0;
    padding-bottom: 40px;
  }
  h2 {
    margin: 10px 0 14px;
    font-size: 22px;
  }
  h3 {
    margin: 26px 0 10px;
    font-size: 15px;
    color: var(--text-secondary);
  }
  .card {
    border-radius: var(--r-l);
    background: var(--bg-surface);
    border: 1px solid var(--hairline);
  }
  .item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    min-height: 64px;
    padding: 12px 18px;
    border-top: 1px solid var(--hairline);
  }
  .item:first-child {
    border-top: 0;
  }
  .item.col {
    flex-direction: column;
    align-items: stretch;
    gap: 12px;
  }
  .label {
    min-width: 0;
  }
  .lt {
    font-weight: 500;
  }
  .lh {
    font-size: 12.5px;
    margin-top: 2px;
    max-width: 560px;
  }
  .preview {
    display: grid;
    place-items: end center;
    height: 150px;
    margin-bottom: 14px;
    padding-bottom: 18px;
    border-radius: var(--r-l);
    background: linear-gradient(135deg, #2a2a2a, #121212 55%, #1e1e1e), #000;
    border: 1px solid var(--hairline);
    overflow: hidden;
  }
  .sample {
    text-align: center;
    line-height: 1.3;
    padding: 2px 10px;
    border-radius: 4px;
  }
  .langs,
  .folders {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
  .folders {
    flex-direction: column;
    align-items: stretch;
  }
  .lang {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 34px;
    padding: 0 6px 0 10px;
    border-radius: 17px;
    background: var(--bg-elevated);
    font-size: 13px;
  }
  .ln {
    color: var(--text-disabled);
    font-size: 12px;
  }
  .mini {
    width: 24px;
    height: 24px;
    border-radius: 6px;
    display: grid;
    place-items: center;
    color: var(--text-secondary);
  }
  .mini:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .mini:disabled {
    opacity: 0.3;
  }
  .folder {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 40px;
    padding: 0 8px 0 12px;
    border-radius: var(--r-m);
    background: var(--bg-elevated);
  }
  .folder span {
    flex: 1;
    min-width: 0;
  }
  .facts {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
  }
  .small {
    font-size: 12.5px;
  }
  .swatches {
    display: flex;
    gap: 8px;
  }
  .sw {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 2px solid transparent;
    box-shadow: 0 0 0 1px var(--border-subtle);
  }
  .sw.on {
    border-color: var(--bg-surface);
    box-shadow: 0 0 0 2px var(--accent-hover);
  }
  .area {
    height: 110px;
    padding: 10px 12px;
    resize: vertical;
    font-family: var(--mono);
    font-size: 12.5px;
  }
  .danger {
    color: var(--danger);
  }
  .icons {
    display: flex;
    gap: 10px;
  }
  .icons img {
    border-radius: 10px;
  }
  .keys {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2px 24px;
    padding: 12px 18px;
  }
  .kr {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 38px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .kk {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--text-primary);
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    padding: 2px 8px;
    white-space: nowrap;
  }
  .about {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 4px;
    padding: 40px 20px;
  }
  .about img {
    border-radius: 22px;
    box-shadow: var(--shadow-card);
    margin-bottom: 14px;
  }
  .aname {
    font-family: "Segoe UI Variable Display", "Segoe UI", sans-serif;
    font-size: 26px;
    font-weight: 700;
    letter-spacing: 0.24em;
  }
  .about p {
    max-width: 460px;
  }
  .acts {
    display: flex;
    gap: 8px;
    margin: 8px 0 20px;
  }
  .credits {
    font-size: 12.5px;
    max-width: 520px;
  }
  :global(.spin) {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
