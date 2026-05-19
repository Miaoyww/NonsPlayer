<script lang="ts">
  import { goto } from "$app/navigation";
  import { ChevronLeft, ChevronRight } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";

  let {
    direction = "back" as "back" | "forward",
    href = undefined as string | undefined,
    onclick = undefined as (() => void) | undefined,
    title = "",
  }: {
    direction?: "back" | "forward";
    href?: string;
    onclick?: () => void;
    title?: string;
  } = $props();

  function handleClick() {
    if (onclick) {
      onclick();
    } else if (href) {
      goto(href);
    } else if (direction === "back") {
      history.back();
    } else {
      history.forward();
    }
  }
</script>

<Button
  variant="ghost"
  size="icon"
  class="size-8 bg-transparent text-muted-foreground transition-colors hover:bg-accent hover:text-foreground cursor-pointer"
  {title}
  onclick={handleClick}
>
  {#if direction === "back"}
    <ChevronLeft />
  {:else}
    <ChevronRight />
  {/if}
</Button>
