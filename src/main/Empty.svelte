<script lang="ts">
  import Icon from "../components/Icon.svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { api } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  let { icon, title, text, folderAction = true }: { icon: string; title: string; text: string; folderAction?: boolean } = $props();
  async function addFolder() {
    const d = await openDialog({ directory: true, multiple: false });
    if (typeof d === "string") await api.libraryAddFolder(d);
  }
</script>

<div class="empty">
  <div class="ic"><Icon name={icon} size={34} stroke={1.4} /></div>
  <h2>{title}</h2>
  <p class="muted">{text}</p>
  {#if folderAction}<button class="btn secondary" onclick={addFolder}><Icon name="folder-plus" size={16} />{t("Add a folder")}</button>{/if}
</div>

<style>
  .empty {
    margin: 64px auto 0;
    max-width: 440px;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 0 24px;
  }
  .ic {
    width: 72px;
    height: 72px;
    border-radius: 22px;
    display: grid;
    place-items: center;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    color: var(--text-secondary);
    margin-bottom: 10px;
  }
  h2 {
    margin: 0;
    font-size: 19px;
  }
  p {
    margin: 2px 0 16px;
  }
</style>
