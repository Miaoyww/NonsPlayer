<script lang="ts">
  import { Star, SkipForward, ListMusic, ChevronRight } from "@lucide/svelte";
  import FavoritePlaylistCard from "$lib/components/cards/favorite-playlist-card.svelte";
  import RecommendedSongCard from "$lib/components/cards/recommended-song-card.svelte";
  import SongCard from "$lib/components/cards/song-card.svelte";
  import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";

  let greeting = "晚上好";
  let quote = "受尽苦难而不厌，此乃阿修罗之道。";

  // 临时数据
  const dummySongs = [
    {
      songName: "僕の戦争",
      artist: "神聖かまってちゃん",
      duration: "3:30",
      album: "僕の戦争",
    },
    {
      songName: "The Rumbling",
      artist: "SiM",
      duration: "3:40",
      album: "The Rumbling",
    },
    { songName: "紅蓮華", artist: "LiSA", duration: "3:56", album: "紅蓮華" },
    {
      songName: "廻廻奇譚",
      artist: "Eve",
      duration: "3:38",
      album: "廻廻奇譚",
    },
    {
      songName: "残酷な天使のテーゼ",
      artist: "高橋洋子",
      duration: "4:06",
      album: "残酷な天使のテーゼ",
    },
    {
      songName: "Again",
      artist: "YUI",
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
</script>

<div class="flex flex-col gap-8 p-8 h-full overflow-auto">
  <!-- 头部区域 -->
  <div class="flex justify-between items-start">
    <!-- 问候语 -->
    <div class="flex flex-col gap-2">
      <p class="text-xl font-bold text-foreground">{greeting}</p>
      <p class="text-sm font-medium text-gray-400 whitespace-nowrap">
        {quote}
      </p>
      <!-- 最爱歌单卡片 -->
      <FavoritePlaylistCard />
    </div>
  </div>

  <!-- 最爱歌单 -->
  <div class="flex flex-col gap-4">
    <div class="flex items-center gap-1">
      <ListMusic class="size-6 text-foreground" />
      <p class="text-xl font-bold text-foreground">最爱歌单</p>
      <ChevronRight class="size-5 text-foreground" />
    </div>
  </div>

  <!-- 底部双栏 -->
  <div class="flex justify-between items-end gap-8 mt-auto">
    <!-- 下一首播放 -->
    <div class="flex flex-col gap-3 min-w-0 flex-1">
      <div class="flex items-center gap-1">
        <SkipForward class="size-6 text-foreground" />
        <p class="text-xl font-bold text-foreground">下一首播放</p>
      </div>

      <!-- 歌曲列表：单行横向滚动 -->
      <ScrollArea class="flex gap-2 pb-1">
        {#each dummySongs as song}
          <div class="shrink-0">
            <SongCard {...song} />
          </div>
        {/each}
      </ScrollArea>
    </div>

    <!-- 推荐歌单 -->
    <div class="flex items-center gap-1 shrink-0">
      <ListMusic class="size-6 text-foreground" />
      <p class="text-xl font-bold text-foreground">推荐歌单</p>
      <ChevronRight class="size-5 text-foreground" />
    </div>
  </div>
</div>
