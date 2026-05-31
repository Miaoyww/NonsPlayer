<script lang="ts">
  import { X } from "@lucide/svelte";
  import Button from "$lib/components/ui/button/button.svelte";

  interface Props {
    show?: boolean;
    onClose?: () => void;
  }

  let {
    show = true,
    onClose = () => {},
  }: Props = $props();
</script>

<div
  class="absolute top-0 left-0 right-0 h-20 z-[100] flex items-center justify-between px-5 transition-opacity duration-300"
  class:opacity-0={!show}
  class:opacity-100={show}
  style="background: linear-gradient(to bottom, rgba(0,0,0,0.5) 0%, transparent 100%);"
>
  <!-- Left: spacer -->
  <div class="w-32"></div>

  <!-- Center: drag region -->
  <div class="flex-1 h-full" style="-webkit-app-region: drag;"></div>

  <!-- Right: close button -->
  <div class="flex items-center justify-end w-32 gap-1">
    <Button
      variant="ghost"
      size="icon"
      class="text-white/60 hover:text-white hover:bg-white/10 rounded-lg transition"
      onclick={onClose}
    >
      <X class="size-[28px]" />
    </Button>
  </div>
</div>
