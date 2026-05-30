<script lang="ts">
  import SettingCard from "$lib/components/cards/settings-card.svelte";
  import { Button } from "$lib/components/ui/button";
  import { Switch } from "$lib/components/ui/switch";
  import { globalSettings } from "$lib/stores/global-settings-store";
  import { Folder, Plus, Trash2, FolderOpen, Settings } from "@lucide/svelte";
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { open } from "@tauri-apps/plugin-dialog";
  import * as Dialog from "$lib/components/ui/dialog";

  let localFolders = $state<string[]>([]);
  let localLyricFirst = $state(true);
  let initialized = false;

  // Persist lyric settings whenever toggles change (skips initial mount)
  $effect(() => {
    localLyricFirst;
    if (initialized) {
      globalSettings.patch({ localLyricFirst });
    }
  });

  onMount(() => {
    const unsub = globalSettings.subscribe((s) => {
      localFolders = [...s.localMusicFolders];
      localLyricFirst = s.localLyricFirst;
    });
    // Mark initialized after first store sync, so $effect won't trigger on mount
    setTimeout(() => { initialized = true; }, 0);
    return unsub;
  });

  function persist() {
    globalSettings.patch({
      localMusicFolders: localFolders.filter((f) => f.trim() !== ""),
    });
  }

  async function addFolder() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择音乐文件夹",
    });
    if (selected && typeof selected === "string") {
      localFolders = [...localFolders, selected];
      persist();
    }
  }

  function removeFolder(index: number) {
    localFolders = localFolders.filter((_, i) => i !== index);
    persist();
  }

</script>

<div in:fly={{ y: 16, duration: 300, opacity: 0 }}>
  <div class="mb-1 text-xl font-bold text-stone-800 dark:text-stone-100">
    本地设置
  </div>
  <p class="mb-4 text-sm text-muted-foreground">
    管理本地音乐文件夹。添加文件夹后，NonsPlayer 会自动扫描其中的音频文件。
  </p>

  <div class="space-y-3">
    <SettingCard
      title="音乐文件夹"
      description="添加或移除本地音乐文件夹。已添加的文件夹会被自动扫描。"
    >
      <Dialog.Root>
        <Dialog.Trigger>
          <Button variant="outline" size="sm">
            <Settings class="size-4" />
            管理文件夹
          </Button>
        </Dialog.Trigger>
        <Dialog.Content class="sm:max-w-120">
          <Dialog.Header>
            <Dialog.Title>管理音乐文件夹</Dialog.Title>
            <Dialog.Description>
              添加或移除本地音乐文件夹。已添加的文件夹会被自动扫描。
            </Dialog.Description>
          </Dialog.Header>
          <div class="flex flex-col gap-3 py-4">
            {#if localFolders.length === 0}
              <div
                class="flex flex-col items-center gap-2 rounded-lg border-2 border-dashed px-4 py-8"
              >
                <Folder class="size-10 text-muted-foreground/40" />
                <p class="text-sm text-muted-foreground">暂未添加文件夹</p>
              </div>
            {:else}
              <div class="flex max-h-80 flex-col gap-2 overflow-y-auto">
                {#each localFolders as folder, i}
                  <div
                    class="flex items-center gap-2.5 rounded-lg border bg-background px-3 py-2"
                    in:fly={{ y: 8, duration: 200, opacity: 0 }}
                  >
                    <FolderOpen class="size-4 shrink-0 text-amber-500" />
                    <span
                      class="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-sm"
                    >
                      {folder}
                    </span>
                    <button
                      class="flex shrink-0 items-center justify-center rounded p-1 text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
                      onclick={() => removeFolder(i)}
                      aria-label="移除文件夹"
                    >
                      <Trash2 class="size-4" />
                    </button>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
          <Dialog.Footer>
            <Button variant="outline" size="sm" class="w-full" onclick={addFolder}>
              <Plus class="size-4" />
              添加文件夹
            </Button>
          </Dialog.Footer>
        </Dialog.Content>
      </Dialog.Root>
    </SettingCard>

    <!-- ── 歌词设置 ── -->
    <SettingCard
      title="歌词来源"
      description="设置歌词获取的优先级和来源。"
    >
      <div class="flex flex-col gap-3">
        <label class="flex items-center gap-3">
          <Switch bind:checked={localLyricFirst} />
          <div class="flex flex-col gap-0.5">
            <span class="text-sm font-medium">本地歌词优先</span>
            <span class="text-xs text-muted-foreground">
              优先使用本地内嵌或外挂 .lrc 文件
            </span>
          </div>
        </label>
      </div>
    </SettingCard>
  </div>
</div>
