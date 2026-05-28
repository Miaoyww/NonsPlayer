<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { adapterStore } from "$lib/stores/adapter-store.svelte";
  import { getRecommendedPlaylists } from "$lib/services/adapter-service";
  import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
  import PlaylistCard from "$lib/components/cards/playlist-card.svelte";
  import { Compass, Sparkles } from "@lucide/svelte";
  import type { Playlist } from "$lib/types";
  import { fly } from "svelte/transition";

  let playlists = $state<Playlist[]>([]);
  let loading = $state(false);

  onMount(async () => {
    await adapterStore.refresh();
    loading = true;

    // Try each adapter for recommendations
    for (const a of adapterStore.adapters) {
      try {
        const recs = await getRecommendedPlaylists(a.slug, 20);
        if (recs.length > 0) {
          playlists = recs;
          break;
        }
      } catch {
        // adapter doesn't support recommend
      }
    }

    // Fallback: use dummy data if no recommendations available
    if (playlists.length === 0) {
      playlists = getDummyPlaylists();
    }

    loading = false;
  });

  function goPlaylist(name: string) {
    goto(`/adapter/playlist/${encodeURIComponent(name)}`);
  }

  function getDummyPlaylists(): Playlist[] {
    const names = [
      "在路上-2026", "深夜安静学习", "动漫金曲精选", "午后咖啡时光",
      "R&B式情绪过肺", "日语｜温柔治愈", "电子｜深夜代码冲刺",
      "说唱｜中文说唱精选", "古典｜专注阅读时光", "民谣｜旅途中的故事",
      "摇滚｜热血公路旅行", "爵士｜深夜咖啡馆", "轻音乐｜雨天阅读",
    ];
    return names.map((name) => ({
      id: name,
      md5: "",
      name,
      shareUrl: "",
      avatarUrl: "",
      smallAvatarUrl: "",
      middleAvatarUrl: "",
      title: name,
      creator: "NonsPlayer",
      createTime: "",
      description: "",
      musicTrackIds: [],
      tags: [],
      musics: [],
      isInitialized: true,
      playCount: Math.floor(Math.random() * 500),
      musicsCount: Math.floor(Math.random() * 150),
      adapterSlug: "local",
    }));
  }
</script>

<div
  class="flex flex-col gap-4 px-8 py-6 h-full overflow-hidden"
  transition:fly={{ y: -20, duration: 200 }}
>
  <div class="flex items-center gap-2 shrink-0">
    <Compass class="size-5 text-foreground" />
    <h1 class="text-xl font-bold text-foreground">发现</h1>
  </div>

  <ScrollArea class="flex-1 min-h-0">
    {#if loading}
      <div class="text-sm text-muted-foreground text-center py-16">加载推荐中...</div>
    {:else}
      <div class="mb-4">
        <div class="flex items-center gap-2 mb-3">
          <Sparkles class="size-4 text-foreground" />
          <h2 class="text-base font-bold text-foreground">推荐歌单</h2>
        </div>
        <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3">
          {#each playlists as p}
            <PlaylistCard playlist={p} onclick={() => goPlaylist(p.name)} />
          {/each}
        </div>
      </div>
    {/if}
  </ScrollArea>
</div>
