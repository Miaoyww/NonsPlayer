<script lang="ts">
  import { SkipForward, ListMusic, ChevronRight } from "@lucide/svelte";
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import FavoritePlaylistCard from "$lib/components/cards/favorite-playlist-card.svelte";
  import SongCard from "$lib/components/cards/song-nextup-card.svelte";
  import PlaylistRowCard from "$lib/components/cards/playlist-row-card.svelte";
  import PlaylistCard from "$lib/components/cards/playlist-card.svelte";
  import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
  import { fly } from "svelte/transition";
  import Button from "$lib/components/ui/button/button.svelte";
  import GreetingQuote from "$lib/components/greeting-quote.svelte";
  import { adapterStore } from "$lib/stores/adapter-store.svelte";
  import { playerService } from "$lib/services/player-service.svelte";
  import { search, getRecommendedPlaylists } from "$lib/services/adapter-service";
  import type { Song, Playlist } from "$lib/types";

  let songs = $state<Song[]>([]);
  let recommendedPlaylists = $state<Playlist[]>([]);
  let loading = $state(true);

  onMount(async () => {
    await adapterStore.refresh();

    // Try to load real data from adapters
    let realSongs: Song[] = [];
    let realPlaylists: Playlist[] = [];

    for (const a of adapterStore.adapters) {
      // Load songs from adapters that support Search
      try {
        const result = await search(a.slug, "");
        if (result.songs.length > 0) {
          realSongs.push(...result.songs);
        }
      } catch { /* skip */ }

      // Load recommended playlists
      try {
        const recs = await getRecommendedPlaylists(a.slug, 4);
        if (recs.length > 0) {
          realPlaylists.push(...recs);
        }
      } catch { /* skip */ }
    }

    // Use real data or fall back to dummies
    songs = realSongs.length > 0 ? realSongs.slice(0, 9) : getDummySongs();
    recommendedPlaylists = realPlaylists.length > 0 ? realPlaylists.slice(0, 4) : getDummyPlaylists();
    loading = false;
  });

  function getDummySongs(): Song[] {
    return [
      mkSong("JANE DOE", "artist-kenshi", "米津玄師 · 宇多田ヒカル", "JANE DOE", "3:02", "(剧场版《电锯人：蕾塞篇》片尾曲)"),
      mkSong("僕の戦争", "artist-shinsei", "神聖かまってちゃん", "僕の戦争", "3:30", "(TV动画《进击的巨人》片头曲)"),
      mkSong("The Rumbling", "artist-sim", "SiM", "The Rumbling", "3:40"),
      mkSong("紅蓮華", "artist-lisa", "LiSA", "紅蓮華", "3:56", "(TV动画《鬼灭之刃》片头曲)"),
      mkSong("廻廻奇譚", "artist-eve", "Eve", "廻廻奇譚", "3:38", "(TV动画《咒术回战》片头曲)"),
      mkSong("残酷な天使のテーゼ", "artist-yoko", "高橋洋子", "残酷な天使のテーゼ", "4:06", "(TV动画《新世纪福音战士》片头曲)"),
      mkSong("Again", "artist-yui", "YUI", "Fullmetal Alchemist", "4:15", "(TV动画《钢之炼金术师》片头曲)"),
      mkSong("シルエット", "artist-kana", "KANA-BOON", "シルエット", "4:01"),
      mkSong("R★O★C★K★S", "artist-hound", "HOUND DOG", "ROCKS", "3:39"),
    ];
  }

  function getDummyPlaylists(): Playlist[] {
    return [
      mkPlaylist("在路上-2026", "Miaoyww", 161, 104),
      mkPlaylist("深夜安静学习", "Miaoyww", 89, 52),
      mkPlaylist("动漫金曲精选", "NonsPlayer", 342, 128),
      mkPlaylist("午后咖啡时光", "Miaoyww", 56, 36),
    ];
  }

  // Dummy factory functions (used as fallback)
  const empty = "";
  const a = (id: string, name: string) =>
    ({ id, md5: empty, name, shareUrl: empty, avatarUrl: empty, smallAvatarUrl: empty, middleAvatarUrl: empty, createDate: empty, description: empty, songs: [], artists: [], artistsName: name, collectionCount: 0, trackCount: 0, adapterSlug: "local" }) as Song["album"];
  const r = (id: string, name: string) =>
    ({ id, md5: empty, name, shareUrl: empty, avatarUrl: empty, smallAvatarUrl: empty, middleAvatarUrl: empty, description: empty, songs: [], musicCount: 0, trans: empty, adapterSlug: "local" }) as Song["artists"][number];

  function mkSong(name: string, artistId: string, artistName: string, albumName: string, durationText: string, trans?: string): Song {
    return {
      id: empty, md5: empty, name, shareUrl: empty, avatarUrl: empty, smallAvatarUrl: empty, middleAvatarUrl: empty,
      album: a(empty, albumName),
      artists: [r(artistId, artistName)],
      isEmpty: false, duration: 0, url: empty, lyric: null, available: true,
      isLiked: false, trans: trans ?? null, albumName, artistsName: artistName, durationText,
      adapterSlug: "local",
    };
  }

  function mkPlaylist(name: string, creator: string, playCount: number, musicsCount: number): Playlist {
    return {
      id: empty, md5: empty, name, shareUrl: empty, avatarUrl: empty, smallAvatarUrl: empty, middleAvatarUrl: empty,
      title: name, creator, createTime: empty, description: empty, musicTrackIds: [], tags: [], musics: [],
      isInitialized: true, playCount, musicsCount,
      adapterSlug: "local",
    };
  }

  function goPlaylist(playlist: { id: string; adapterSlug: string }) {
    goto(`/adapter/${playlist.adapterSlug}/playlist/${encodeURIComponent(playlist.id)}`);
  }

  function playSong(song: Song) {
    playerService.play(songs, songs.indexOf(song));
  }

  // Favorite playlists grid (static curated list)
  const favoritePlaylists: Playlist[] = [
    mkPlaylist("R&B式情绪过肺｜深呼吸把烦恼吐出去", "", 0, 0),
    mkPlaylist("日语｜温柔治愈的日系旋律", "", 0, 0),
    mkPlaylist("电子｜深夜代码冲刺", "", 0, 0),
    mkPlaylist("说唱｜中文说唱精选集", "", 0, 0),
    mkPlaylist("古典｜专注阅读时光", "", 0, 0),
    mkPlaylist("民谣｜旅途中的故事", "", 0, 0),
    mkPlaylist("摇滚｜热血公路旅行", "", 0, 0),
    mkPlaylist("爵士｜深夜咖啡馆", "", 0, 0),
    mkPlaylist("轻音乐｜雨天阅读", "", 0, 0),
    mkPlaylist("欧美｜公告牌精选", "", 0, 0),
    mkPlaylist("韩语｜K-Pop热单", "", 0, 0),
    mkPlaylist("纯音乐｜专注工作", "", 0, 0),
  ];
</script>

<div
  class="grid grid-rows-[auto_1fr] gap-4 p-8 h-full overflow-hidden"
  transition:fly={{ y: -20, duration: 200 }}
>
  <!-- ===== 上半部分：问候语 + 最爱歌单 ===== -->
  <div class="flex flex-col gap-4 min-h-0 overflow-hidden">
    <GreetingQuote />

    <!-- 我的喜欢 + 最爱歌单 3:7 布局 -->
    <div class="flex gap-8 min-h-0 overflow-hidden">
      <div class="min-h-0 overflow-hidden" style="flex: 4;">
        <FavoritePlaylistCard />
      </div>

      <div class="min-h-0 overflow-hidden" style="flex: 7;">
        <div class="grid grid-cols-4 gap-3 auto-rows-auto">
          {#each favoritePlaylists as p}
            <PlaylistCard playlist={p} onclick={() => goPlaylist(p)} />
          {/each}
        </div>
      </div>
    </div>
  </div>

  <!-- ===== 下半部分：下一首播放 + 推荐歌单 ===== -->
  <div class="grid grid-cols-2 gap-4 min-h-0 overflow-hidden">
    <!-- 下一首播放 -->
    <div class="flex flex-col gap-3 min-h-0 overflow-hidden">
      <div class="flex items-center gap-1 shrink-0">
        <SkipForward class="size-4 text-foreground" />
        <p class="text-base font-bold text-foreground">下一首播放</p>
        <Button
          variant="ghost"
          class="cursor-pointer"
          size="icon"
          href="/playqueue"
        >
          <ChevronRight class="size-4 text-foreground" />
        </Button>
      </div>
      <ScrollArea class="flex gap-2 pb-1 flex-1 min-h-0">
        {#if loading}
          <div class="text-sm text-muted-foreground p-4">加载中...</div>
        {:else}
          {#each songs as song}
            <div class="shrink-0 mb-2">
              <SongCard {song} onplay={() => playSong(song)} />
            </div>
          {/each}
        {/if}
      </ScrollArea>
    </div>

    <!-- 推荐歌单 -->
    <div class="flex flex-col gap-3 min-h-0 overflow-hidden">
      <div class="flex items-center gap-1 shrink-0">
        <ListMusic class="size-6 text-foreground" />
        <p class="text-base font-bold text-foreground">推荐歌单</p>
        <Button
          variant="ghost"
          class="cursor-pointer"
          size="icon"
          href="/explore"
        >
          <ChevronRight class="size-4 text-foreground" />
        </Button>
      </div>
      <ScrollArea class="flex flex-col gap-2 flex-1 min-h-0">
        {#if loading}
          <div class="text-sm text-muted-foreground p-4">加载中...</div>
        {:else}
          {#each recommendedPlaylists as p}
            <div class="mb-2">
              <PlaylistRowCard playlist={p} onplay={() => goPlaylist(p)} />
            </div>
          {/each}
        {/if}
      </ScrollArea>
    </div>
  </div>
</div>
