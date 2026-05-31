<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { cn } from "$lib/utils";
  import Button from "../ui/button/button.svelte";

  let {
    href,
    label,
    class: className = "",
  }: {
    href: string;
    label: string;
    class?: string;
  } = $props();

  let isActive = $derived($page.url.pathname === href);

  function onClick() {
    goto(href);
  }
</script>

<Button
  class="relative flex flex-col items-center gap-1 p-0 font-medium text-muted-foreground transition-colors hover:text-foreground {className} cursor-pointer"
  variant="ghost"
  onclick={onClick}
>
  <span class="text-base leading-none">
    {label}
  </span>
  <span
    class={cn(
      "h-0.5 w-5 rounded-full transition-opacity cursor-pointer bg-blue-600",
    )}
    class:opacity-100={isActive}
    class:opacity-0={!isActive}
  ></span>
</Button>
