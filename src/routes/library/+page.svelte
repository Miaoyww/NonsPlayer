<script lang="ts">
  import { goto } from "$app/navigation";
  import { adapterStore } from "$lib/stores/adapter-store.svelte";
  import { playerService } from "$lib/services/player-service.svelte";
  import { search } from "$lib/services/adapter-service";
  import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
  import Button from "$lib/components/ui/button/button.svelte";
  import SongList from "$lib/components/song-list.svelte";
  import { PlugZap, Music, ChevronRight, Disc3, SearchX } from "@lucide/svelte";
  import type { Song } from "$lib/types";
  import { fly } from "svelte/transition";

  let localSongs = $state<Song[]>([]);
  let loadingLocal = $state(false);
  let localError = $state("");
  let localLoaded = false;

  // Adapters are initialized by +layout.svelte; reactively load local music
  // when the local adapter becomes available.
  $effect(() => {
    if (localLoaded) return;
    const localAdapter = adapterStore.get("local");
    if (!localAdapter) return;

    localLoaded = true;
    loadingLocal = true;
    search("local", "")
      .then((result) => {
        localSongs = result.songs;
      })
      .catch((e) => {
        localError = String(e);
      })
      .finally(() => {
        loadingLocal = false;
      });
  });

  const onlineAdapters = $derived(
    adapterStore.streaming,
  );

  function playAllLocal(index = 0) {
    if (localSongs.length > 0) {
      playerService.play(localSongs, index);
    }
  }

  function playAll() { playAllLocal(0); }
</script>

<div
  class="grid grid-rows-[auto_1fr] gap-6 px-8 py-6 h-full overflow-hidden"
  in:fly={{ y: 16, duration: 300, opacity: 0 }}
>
  <!-- ===== 上半：在线适配器卡片 ===== -->
  <div class="shrink-0">
    <div class="flex items-center gap-2 mb-3">
      <PlugZap class="size-5 text-foreground" />
      <h2 class="text-lg font-bold text-foreground">音乐服务</h2>
    </div>

    <div
      class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3"
    >
      {#each onlineAdapters as adapter}
        <button
          class="flex items-center gap-3 rounded-xl p-3 bg-card border border-border/50 hover:bg-accent/40 hover:border-border hover:shadow-md transition-all duration-200 cursor-pointer text-left group"
          onclick={() => goto(`/adapter/${adapter.slug}`)}
        >
          <div
            class="size-10 shrink-0 rounded-lg bg-linear-to-br from-primary/20 to-primary/5 flex items-center justify-center"
          >
            <Disc3 class="size-5 text-primary/60" />
          </div>
          <div class="min-w-0 flex-1">
            <p class="text-sm font-medium text-foreground truncate">
              {adapter.displayPlatform}
            </p>
            <p class="text-xs text-muted-foreground truncate">
              {adapter.description}
            </p>
          </div>
          <ChevronRight
            class="size-4 text-muted-foreground/40 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity"
          />
        </button>
      {/each}

      {#if onlineAdapters.length === 0}
        <div
          class="col-span-full text-center py-8 text-sm text-muted-foreground"
        >
          暂无在线音乐服务，请在设置中连接在线服务
        </div>
      {/if}
    </div>
  </div>

  <!-- ===== 下半：本地音乐 ===== -->
  <div class="flex flex-col min-h-0 overflow-hidden">
    <div class="flex items-center justify-between mb-3 shrink-0">
      <div class="flex items-center gap-2">
        <Music class="size-5 text-foreground" />
        <h2 class="text-lg font-bold text-foreground">本地音乐</h2>
        {#if localSongs.length > 0}
          <span class="text-xs text-muted-foreground"
            >{localSongs.length} 首</span
          >
        {/if}
        <Button
          variant="ghost"
          size="sm"
          class="cursor-pointer text-xs"
          href="/library/local"
        >
          前往本地音乐库
          <ChevronRight class="size-4 text-muted-foreground/40" />
        </Button>
      </div>
      {#if localSongs.length > 0}
        <Button
          variant="ghost"
          size="sm"
          class="cursor-pointer text-xs"
          onclick={playAll}
        >
          播放全部
        </Button>
      {/if}
    </div>

    <ScrollArea class="flex-1 min-h-0">
      {#if loadingLocal}
        <div class="text-sm text-muted-foreground text-center py-16">
          扫描本地音乐中...
        </div>
      {:else if localError}
        <div class="text-sm text-muted-foreground text-center py-16">
          <SearchX class="size-8 mx-auto mb-2 opacity-40" />
          无法加载本地音乐：{localError}
        </div>
      {:else if localSongs.length === 0}
        <div class="text-sm text-muted-foreground text-center py-16">
          <Music class="size-8 mx-auto mb-2 opacity-40" />
          {#if adapterStore.get("local")}
            本地音乐文件夹为空，请在设置中添加包含音频文件的文件夹
          {:else}
            尚未配置本地音乐文件夹，请在设置中添加
          {/if}
        </div>
      {:else}
        <SongList songs={localSongs} showActions={false} onplay={playAllLocal} />
      {/if}

    </ScrollArea>
  </div>
</div>
