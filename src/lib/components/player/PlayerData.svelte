<script lang="ts">
  interface Props {
    songName?: string;
    artists?: string;
    album?: string;
    center?: boolean;
    light?: boolean;
    hideDetails?: boolean;
  }

  let {
    songName = "",
    artists = "",
    album = "",
    center = false,
    light = false,
    hideDetails = false,
  }: Props = $props();
</script>

<div class="flex flex-col gap-1" class:items-center={center}>
  <h2
    class="text-[26px] font-bold truncate max-w-full"
    class:text-white={light}
    class:text-foreground={!light}
  >
    {songName || "未知曲目"}
  </h2>

  {#if !hideDetails}
    <p
      class="text-base truncate max-w-full"
      class:text-white={light}
      class:text-muted-foreground={!light}
    >
      {artists || "未知艺术家"}
      {#if album}
        <span class="opacity-70"> · {album}</span>
      {/if}
    </p>
  {/if}
</div>
