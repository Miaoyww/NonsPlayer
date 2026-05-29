<script lang="ts">
  import { goto } from "$app/navigation";
  import Button from "$lib/components/ui/button/button.svelte";

  interface Props {
    name: string;
    artist: string;
    artistId?: string;
    alias?: string;
    coverUrl?: string;
    size?: "sm" | "default";
    class?: string;
  }

  let {
    name,
    artist,
    artistId = "",
    alias = "",
    coverUrl = "",
    size = "default",
    class: className = "",
  }: Props = $props();

  function goArtist(e: MouseEvent) {
    e.stopPropagation();
    goto(`/adapter/artist/${encodeURIComponent(artistId || artist)}`);
  }

  // 占位Cover
  const hash = $derived(
    name.split("").reduce((acc, c) => acc + c.charCodeAt(0), 0),
  );
  const hue1 = $derived(hash % 360);
  const hue2 = $derived((hash + 50) % 360);
</script>

<div class="flex items-center gap-4 {className}">
  <!-- Cover -->
  <div class="shrink-0 rounded-md bg-muted overflow-hidden size-12">
    {#if coverUrl}
      <img src={coverUrl} alt="" class="size-full object-cover" />
    {:else}
      <div
        class="size-full"
        style="background: linear-gradient(135deg, hsl({hue1}, 50%, 45%), hsl({hue2}, 60%, 38%))"
      ></div>
    {/if}
  </div>

  <!-- Song name + alias + artist -->
  <div class="flex flex-col justify-center min-w-0 flex-1">
    <div class="flex items-center gap-1 min-w-0">
      <p class="truncate {size === 'sm' ? 'text-sm' : 'text-base'} font-medium">
        {name}
      </p>
      {#if alias}
        <p class="text-xs text-gray-400 truncate shrink-0">{alias}</p>
      {/if}
    </div>
    <Button
      class="truncate justify-start text-left cursor-pointer font-medium text-gray-500 w-fit h-auto py-0 hover:underline-offset-2 p-0 rounded-none"
      onclick={goArtist}
      variant="link"
    >
      {artist}
    </Button>
  </div>
</div>
