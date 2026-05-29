<script lang="ts">
  import SongInfoCard from "$lib/components/cards/song/song-info-card.svelte";
  import SongActionCard from "$lib/components/cards/song/song-action-card.svelte";
  import type { Song } from "$lib/types";

  interface Props {
    song: Song;
    onplay?: () => void;
    onlike?: () => void;
  }

  let { song, onplay, onlike }: Props = $props();
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="group flex items-center gap-4 w-full h-full rounded-lg hover:bg-muted/50 transition-colors cursor-pointer text-left"
  onclick={() => onplay?.()}
  onkeydown={(e) => e.key === "Enter" && onplay?.()}
  role="button"
  tabindex="0"
>
  <SongInfoCard
    name={song.name}
    artist={song.artistsName}
    alias={song.trans ?? ""}
    coverUrl={song.avatarUrl}
    class="flex-1"
  />
  <!-- Duration -->
  <span class="w-20 text-right text-sm text-muted-foreground tabular-nums">
    {song.durationText}
  </span>

  <SongActionCard bind:liked={song.isLiked} variant="card" />
</div>
