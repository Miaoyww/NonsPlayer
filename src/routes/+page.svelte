<script lang="ts">
  import FavoritePlaylistCard from "$lib/components/cards/playlist/favorite-playlist-card.svelte";
  import NextupSongList from "$lib/components/nextup-song-list.svelte";
  import RecommendedPlaylists from "$lib/components/recommended-playlists.svelte";
  import { fly } from "svelte/transition";
  import GreetingQuote from "$lib/components/greeting-quote.svelte";
</script>

<div
  class="grid grid-rows-[auto_1fr] gap-4 pt-8 pl-8 pr-8 h-full overflow-hidden"
  in:fly={{ y: 16, duration: 300, opacity: 0 }}
>
  <!-- ===== 上半部分：问候语 + 最爱歌单 ===== -->
  <div class="flex flex-col gap-4 min-h-0 overflow-hidden">
    <GreetingQuote />

    <FavoritePlaylistCard />
  </div>

  <!-- ===== 下半部分：下一首播放 + 推荐歌单 ===== -->
  <div class="grid grid-cols-2 gap-4 min-h-0 overflow-hidden">
    <NextupSongList />
    <RecommendedPlaylists />
  </div>
</div>
