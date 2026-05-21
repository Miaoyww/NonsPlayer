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

<div class="w-full rounded-lg shadow bg-muted">
  <div class="flex items-end justify-between p-4">
    <!-- 左侧：歌词 + 标题信息 -->
    <div class="flex flex-col gap-5">
      <!-- 歌词 -->
      <div class="text-sm font-medium max-w-80">
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
      <!-- 歌单名称 + 曲目数 -->
      <div class="flex flex-col gap-2">
        <p class="text-sm font-bold text-foreground">{title}</p>
        <p class="text-sm font-bold text-foreground">{trackCount}</p>
      </div>
    </div>

    <!-- 右侧：播放按钮 -->
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
