<script lang="ts">
  import { Heart, Ellipsis } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";

  interface Props {
    liked?: boolean;
    variant?: "table" | "card";
    visible?: boolean;
    onlike?: () => void;
    class?: string;
  }

  let {
    liked = $bindable(false),
    variant = "table",
    visible = true,
    onlike,
    class: className = "",
  }: Props = $props();

  function handleLike(e: MouseEvent) {
    e.stopPropagation();
    liked = !liked;
    onlike?.();
  }
</script>

{#if visible}
  <div class="flex items-center gap-5 shrink-0 m-2 ml-auto w-30 {className}">
    <Button
      class="shrink-0 cursor-pointer {liked ? 'opacity-100' : ''}"
      variant="ghost"
      size="icon"
      onclick={handleLike}
      aria-label={liked ? "取消收藏" : "收藏"}
    >
      <Heart size={16} class={liked ? "fill-blue-500 text-blue-500" : ""} />
    </Button>
    <Button
      class="shrink-0 cursor-pointer"
      variant="ghost"
      size="icon"
      aria-label="更多"
    >
      <Ellipsis size={5} />
    </Button>
  </div>
{/if}
