<script lang="ts">
  import SongInfoCard from "./song-info-card.svelte";
  import SongActionCard from "./song-action-card.svelte";

  interface Props {
    songName: string;
    artist: string;
    alias?: string;
    coverUrl?: string;
    duration?: string;
    album?: string;
    liked?: boolean;
    onplay?: () => void;
    onlike?: () => void;
  }

  let {
    songName,
    artist,
    alias = "",
    coverUrl = "",
    duration = "",
    album = "",
    liked = $bindable(false),
    onplay,
    onlike,
  }: Props = $props();
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
    name={songName}
    {artist}
    {alias}
    {coverUrl}
    class="flex-1"
  />

  <SongActionCard {duration} bind:liked variant="card" />
</div>
