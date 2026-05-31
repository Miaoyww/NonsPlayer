<script lang="ts">
  import { Play } from "@lucide/svelte";
  import { goto } from "$app/navigation";
  import Button from "../../ui/button/button.svelte";
  import { adapterStore } from "$lib/stores/adapter-store.svelte";
  import { getFavoritePlaylist } from "$lib/services/adapter-service";
  import type { Playlist } from "$lib/types";

  let favorite: Playlist | null = $state(null);
  let loading = $state(true);
  let loaded = false;

  $effect(() => {
    const online = adapterStore.streaming;
    // Wait until adapters are actually available (frontend adapters
    // may be merged in asynchronously after the first paint).
    if (online.length === 0) return;
    if (loaded) return;
    loaded = true;

    (async () => {
      for (const a of online) {
        try {
          const fav = await getFavoritePlaylist(a.slug);
          if (fav) { favorite = fav; break; }
        } catch { /* adapter may not support this */ }
      }
      loading = false;
    })();
  });

  const hasNoAdapters = $derived(!loading && !favorite && adapterStore.streaming.length === 0);
  const needLogin = $derived(!loading && !favorite && !hasNoAdapters);
  const fav = $derived(favorite!);
</script>

<div class="flex gap-8 min-h-0 overflow-hidden">
  <div class="min-h-0 overflow-hidden" style="flex: 4;">
    <div class="w-full h-full rounded-lg shadow bg-muted flex flex-col">
      {#if loading}
        <div class="flex items-center justify-center h-full text-sm text-muted-foreground">
          加载中...
        </div>
      {:else if favorite}
        <!-- 歌词区域 -->
        <div class="text-sm font-medium max-w-80 p-4 pb-0">
          <p>{fav.name}</p>
          <p class="text-xs text-muted-foreground">{fav.musicsCount ?? fav.musicTrackIds?.length ?? 0} 首</p>
        </div>
        <!-- 底部 -->
        <div class="flex items-end justify-between p-4 mt-auto">
          <div class="flex flex-col gap-1">
            <p class="text-2xl font-bold text-blue-500">{fav.name}</p>
            <p class="text-sm text-blue-500">{fav.musicsCount ?? fav.musicTrackIds?.length ?? 0} Tracks</p>
          </div>
          <Button
            class="relative shrink-0 rounded-full flex items-center justify-center hover:bg-blue-600 cursor-pointer bg-blue-500"
            size="icon"
            onclick={() => goto(`/adapter/${fav.adapterSlug}/playlist/${encodeURIComponent(fav.id)}`)}
          >
            <Play class="size-4 text-foreground ml-0.5 fill-white stroke-white" />
          </Button>
        </div>
      {:else if hasNoAdapters}
        <div class="flex flex-col items-center justify-center h-full text-sm text-muted-foreground gap-2 p-4">
          <p>尚未连接音乐服务</p>
          <Button variant="outline" size="sm" class="cursor-pointer text-xs" onclick={() => goto("/settings")}>
            去设置中连接
          </Button>
        </div>
      {:else if needLogin}
        <div class="flex flex-col items-center justify-center h-full text-sm text-muted-foreground gap-2 p-4">
          <p>登录后即可同步喜欢的歌单</p>
        </div>
      {:else}
        <div class="flex items-center justify-center h-full text-sm text-muted-foreground">
          暂无收藏歌单
        </div>
      {/if}
    </div>
  </div>
  <div class="min-h-0 overflow-hidden" style="flex: 7;">
    <div class="h-full flex items-center justify-center text-sm text-muted-foreground">
      暂无收藏歌单
    </div>
  </div>
</div>
