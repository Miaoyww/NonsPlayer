<script lang="ts">
  import { Play } from "@lucide/svelte";
  import { goto } from "$app/navigation";
  import { Button } from "$lib/components/ui/button";
  import SongInfoCard from "$lib/components/cards/song/song-info-card.svelte";
  import SongActionCard from "$lib/components/cards/song/song-action-card.svelte";
  import type { Song } from "$lib/types";

  interface Props {
    songs: Song[];
    showActions?: boolean;
    onlike?: (index: number) => void;
    onplay?: (index: number) => void;
    class?: string;
  }

  let {
    songs,
    showActions = true,
    onlike,
    onplay,
    class: className = "",
  }: Props = $props();
</script>

<div class="pb-24 {className}">
  <!-- Table Header -->
  <div
    class="flex items-center gap-4 px-3 py-2 text-xs font-semibold text-muted-foreground/50 uppercase tracking-wider border-b border-border"
  >
    <span class="w-8 text-center">#</span>
    <span class="w-12 shrink-0"></span>
    <span class="flex-1">歌曲</span>
    <span class="w-72">专辑</span>
    <span class="w-20 text-right mr-16">时长</span>
  </div>

  <!-- Song Rows -->
  {#each songs as song, i}
    {@const idx = i + 1}
    <div
      class="flex items-center gap-4 px-3 py-2 rounded-lg hover:bg-muted/50 transition-colors cursor-pointer group"
      role="button"
      tabindex="0"
      onclick={() => onplay?.(i)}
      onkeydown={(e) => e.key === "Enter" && onplay?.(i)}
    >
      <!-- Index -->
      <span
        class="w-8 text-center text-sm text-muted-foreground group-hover:hidden"
      >
        {idx}
      </span>
      <span
        class="w-8 text-center hidden group-hover:flex items-center justify-center"
      >
        <Play size={14} />
      </span>

      <SongInfoCard
        name={song.name}
        artist={song.artistsName}
        coverUrl={song.avatarUrl}
        size="sm"
        class="flex-1"
      />

      <!-- Album -->
      <Button
        class="w-72 truncate justify-start text-left text-sm text-muted-foreground h-auto py-0 hover:underline-offset-2 p-0 rounded-none cursor-pointer"
        variant="link"
        onclick={(e) => {
          e.stopPropagation();
          goto(`/adapter/album/${encodeURIComponent(song.album.id)}`);
        }}
      >
        {song.albumName}
      </Button>

      <!-- Duration -->
      <span class="w-20 text-right text-sm text-muted-foreground tabular-nums">
        {song.durationText}
      </span>

      <SongActionCard
        bind:liked={song.isLiked}
        visible={showActions}
        onlike={() => onlike?.(i)}
      />
    </div>
  {/each}
</div>
