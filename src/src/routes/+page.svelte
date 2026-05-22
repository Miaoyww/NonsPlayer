<script lang="ts">
  import { SkipForward, ListMusic, ChevronRight } from "@lucide/svelte";
  import FavoritePlaylistCard from "$lib/components/cards/favorite-playlist-card.svelte";
  import SongCard from "$lib/components/cards/song-card.svelte";
  import PlaylistRowCard from "$lib/components/cards/playlist-row-card.svelte";
  import PlaylistCard from "$lib/components/cards/playlist-card.svelte";
  import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
  import { fly } from "svelte/transition";

  let greeting = "晚上好";
  let quote = "受尽苦难而不厌，此乃阿修罗之道。";

  // 临时数据 - 歌曲
  const dummySongs = [
    {
      songName: "JANE DOE",
      artist: "米津玄師 · 宇多田ヒカル",
      alias: "(剧场版《电锯人：蕾塞篇》片尾曲)",
      duration: "3:02",
      album: "JANE DOE",
    },
    {
      songName: "僕の戦争",
      artist: "神聖かまってちゃん",
      alias: "(TV动画《进击的巨人》片头曲)",
      duration: "3:30",
      album: "僕の戦争",
    },
    {
      songName: "The Rumbling",
      artist: "SiM",
      duration: "3:40",
      album: "The Rumbling",
    },
    {
      songName: "紅蓮華",
      artist: "LiSA",
      alias: "(TV动画《鬼灭之刃》片头曲)",
      duration: "3:56",
      album: "紅蓮華",
    },
    {
      songName: "廻廻奇譚",
      artist: "Eve",
      alias: "(TV动画《咒术回战》片头曲)",
      duration: "3:38",
      album: "廻廻奇譚",
    },
    {
      songName: "残酷な天使のテーゼ",
      artist: "高橋洋子",
      alias: "(TV动画《新世纪福音战士》片头曲)",
      duration: "4:06",
      album: "残酷な天使のテーゼ",
    },
    {
      songName: "Again",
      artist: "YUI",
      alias: "(TV动画《钢之炼金术师》片头曲)",
      duration: "4:15",
      album: "Fullmetal Alchemist",
    },
    {
      songName: "シルエット",
      artist: "KANA-BOON",
      duration: "4:01",
      album: "シルエット",
    },
    {
      songName: "R★O★C★K★S",
      artist: "HOUND DOG",
      duration: "3:39",
      album: "ROCKS",
    },
  ];

  // 临时数据 - 推荐歌单
  const dummyPlaylists = [
    {
      playlistName: "在路上-2026",
      creator: "Miaoyww",
      playCount: "161",
      trackCount: "104 Tracks",
    },
    {
      playlistName: "深夜安静学习",
      creator: "Miaoyww",
      playCount: "89",
      trackCount: "52 Tracks",
    },
    {
      playlistName: "动漫金曲精选",
      creator: "NonsPlayer",
      playCount: "342",
      trackCount: "128 Tracks",
    },
    {
      playlistName: "午后咖啡时光",
      creator: "Miaoyww",
      playCount: "56",
      trackCount: "36 Tracks",
    },
  ];

  // 临时数据 - 最爱歌单卡片 3x4
  const favoritePlaylists = [
    { playlistName: "R&B式情绪过肺｜深呼吸把烦恼吐出去" },
    { playlistName: "日语｜温柔治愈的日系旋律" },
    { playlistName: "电子｜深夜代码冲刺" },
    { playlistName: "说唱｜中文说唱精选集" },
    { playlistName: "古典｜专注阅读时光" },
    { playlistName: "民谣｜旅途中的故事" },
    { playlistName: "摇滚｜热血公路旅行" },
    { playlistName: "爵士｜深夜咖啡馆" },
    { playlistName: "轻音乐｜雨天阅读" },
    { playlistName: "欧美｜公告牌精选" },
    { playlistName: "韩语｜K-Pop热单" },
    { playlistName: "纯音乐｜专注工作" },
  ];
</script>

<div
  class="grid grid-rows-[auto_1fr] gap-4 p-8 h-screen overflow-hidden"
  transition:fly={{ y: -20, duration: 400 }}
>
  <!-- ===== 上半部分：问候语 + 最爱歌单 ===== -->
  <div class="flex flex-col gap-4 min-h-0 overflow-hidden">
    <!-- 问候语 -->
    <div class="flex flex-col gap-1 shrink-0">
      <p class="text-xl font-bold text-foreground">{greeting}</p>
      <p class="text-sm font-medium text-gray-400 whitespace-nowrap">{quote}</p>
    </div>

    <!-- 最爱歌单标签 左对齐 -->
    <div class="flex items-center gap-1 shrink-0">
      <ListMusic class="size-5 text-foreground" />
      <p class="text-base font-bold text-foreground">最爱歌单</p>
      <ChevronRight class="size-4 text-foreground" />
    </div>

    <!-- 我的喜欢 + 最爱歌单 3:7 布局 -->
    <div class="flex gap-9 min-h-0 overflow-hidden">
      <!-- 我的喜欢 (flex:3) -->
      <div class="min-h-0 overflow-hidden" style="flex: 3;">
        <FavoritePlaylistCard />
      </div>

      <!-- 最爱歌单 3列固定网格 (flex:7) -->
      <div class="min-h-0 overflow-hidden" style="flex: 7;">
        <div class="grid grid-cols-3 gap-3 auto-rows-auto">
          {#each favoritePlaylists as playlist}
            <PlaylistCard {...playlist} />
          {/each}
        </div>
      </div>
    </div>
  </div>

  <!-- ===== 下半部分 2/3：下一首播放 + 推荐歌单 ===== -->
  <div class="grid grid-cols-2 gap-4 min-h-0 overflow-hidden">
    <!-- 下一首播放 -->
    <div class="flex flex-col gap-3 min-h-0 overflow-hidden">
      <div class="flex items-center gap-1 shrink-0">
        <SkipForward class="size-6 text-foreground" />
        <p class="text-base font-bold text-foreground">下一首播放</p>
      </div>
      <ScrollArea class="flex gap-2 pb-1 flex-1 min-h-0">
        {#each dummySongs as song}
          <div class="shrink-0 mb-2">
            <SongCard {...song} />
          </div>
        {/each}
      </ScrollArea>
    </div>

    <!-- 推荐歌单 -->
    <div class="flex flex-col gap-3 min-h-0 overflow-hidden">
      <div class="flex items-center gap-1 shrink-0">
        <ListMusic class="size-6 text-foreground" />
        <p class="text-base font-bold text-foreground">推荐歌单</p>
        <ChevronRight class="size-5 text-foreground" />
      </div>
      <ScrollArea class="flex flex-col gap-2 flex-1 min-h-0">
        {#each dummyPlaylists as playlist}
          <div class="mb-2">
            <PlaylistRowCard {...playlist} />
          </div>
        {/each}
      </ScrollArea>
    </div>
  </div>
</div>
