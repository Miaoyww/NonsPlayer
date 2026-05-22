<script lang="ts">
  import { Play } from "@lucide/svelte";
  import Button from "../ui/button/button.svelte";

  interface Props {
    title?: string;
    trackCount?: string;
    lyrics?: string[];
  }

  let {
    title = "我的喜欢",
    trackCount = "2862 Tracks",
    lyrics = [],
  }: Props = $props();
</script>

<div class="w-full h-full rounded-lg shadow bg-muted flex flex-col">
  <!-- 歌词 置顶 -->
  <div class="text-sm font-medium max-w-80 p-4 pb-0">
    {#if lyrics.length > 0}
      {#each lyrics as line}
        <p>{line}</p>
      {/each}
    {:else}
      <p>and I'm sorry</p>
      <p>But I can't control just what you do</p>
      <p>And I hope to god that you leave me soon</p>
      <p>That you walk away from this</p>
    {/if}
  </div>

  <!-- 底部：标题 + 播放按钮 -->
  <div class="flex items-end justify-between p-4 mt-auto">
    <div class="flex flex-col gap-1">
      <p class="text-2xl font-bold text-blue-500">{title}</p>
      <p class="text-sm text-blue-500">{trackCount}</p>
    </div>

    <Button
      class="relative shrink-0 rounded-full flex items-center justify-center hover:bg-blue-600 cursor-pointer bg-blue-500"
      size="icon"
    >
      <Play
        class="size-4 text-foreground ml-0.5 fill-white stroke-white"
      />
    </Button>
  </div>
</div>
