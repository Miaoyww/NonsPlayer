<script lang="ts">
  import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '$lib/components/ui/card';
  import { globalSettings } from '$lib/stores/global-settings-store';
  import { Folder, Plus, Trash2 } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import { fly } from 'svelte/transition';
  import { open } from '@tauri-apps/plugin-dialog';

  let localFolders = $state<string[]>([]);

  onMount(() => {
    const unsub = globalSettings.subscribe((s) => {
      localFolders = [...s.localMusicFolders];
    });
    return unsub;
  });

  function persist() {
    globalSettings.patch({
      localMusicFolders: localFolders.filter((f) => f.trim() !== '')
    });
  }

  async function addFolder() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: '选择音乐文件夹',
    });
    if (selected && typeof selected === 'string') {
      localFolders = [...localFolders, selected];
      persist();
    }
  }

  function removeFolder(index: number) {
    localFolders = localFolders.filter((_, i) => i !== index);
    persist();
  }
</script>

<div class="step-card-container">
  <Card class="step-card">
    <CardHeader>
      <CardTitle class="step-title">本地音乐</CardTitle>
      <CardDescription class="step-desc">
        添加本地音乐文件夹，NonsPlayer 会自动扫描其中的音频文件。你可以在设置中随时添加更多文件夹。
      </CardDescription>
    </CardHeader>
    <CardContent>
      <div class="folder-section">
        {#if localFolders.length === 0}
          <div class="folder-empty">
            <Folder class="size-12 text-muted-foreground/40" />
            <p class="text-muted-foreground text-sm">暂未添加文件夹</p>
          </div>
        {/if}
        <div class="folder-list">
          {#each localFolders as folder, i}
            <div class="folder-item" transition:fly={{ y: -10, duration: 200 }}>
              <Folder class="size-4 text-amber-500 shrink-0" />
              <span class="folder-path">{folder}</span>
              <button class="folder-remove" onclick={() => removeFolder(i)}>
                <Trash2 class="size-4" />
              </button>
            </div>
          {/each}
        </div>
        <button class="folder-add-btn" onclick={addFolder}>
          <Plus class="size-4" />
          添加文件夹
        </button>
      </div>
    </CardContent>
  </Card>
</div>

<style>
  .step-card-container { width: 100%; }

  .folder-section { display: flex; flex-direction: column; gap: 0.75rem; }
  .folder-empty {
    display: flex; flex-direction: column; align-items: center; gap: 0.5rem;
    padding: 1.5rem; border: 2px dashed var(--border); border-radius: 0.75rem;
  }
  .folder-list { display: flex; flex-direction: column; gap: 0.5rem; }
  .folder-item {
    display: flex; align-items: center; gap: 0.625rem;
    padding: 0.625rem 0.75rem; border: 1px solid var(--border);
    border-radius: 0.5rem; background: var(--background);
  }
  .folder-path {
    flex: 1; font-size: 0.8125rem; color: var(--foreground);
    min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .folder-remove {
    display: flex; align-items: center; justify-content: center; padding: 0.25rem;
    border: none; background: none; color: var(--muted-foreground);
    cursor: pointer; border-radius: 0.25rem; transition: all 0.2s; flex-shrink: 0;
  }
  .folder-remove:hover { color: var(--destructive); background: var(--destructive) / 0.1; }
  .folder-add-btn {
    display: inline-flex; align-items: center; gap: 0.375rem;
    padding: 0.5rem 0.875rem; border: 1.5px dashed var(--border);
    border-radius: 0.5rem; background: var(--background);
    color: var(--muted-foreground); font-size: 0.8125rem;
    cursor: pointer; transition: all 0.2s;
  }
  .folder-add-btn:hover { border-color: var(--ring); color: var(--foreground); background: var(--accent); }
</style>
