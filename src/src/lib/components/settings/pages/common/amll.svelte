<script lang="ts">
  import SettingCard from "$lib/components/cards/settings-card.svelte";
  import { Switch } from "$lib/components/ui/switch";
  import { globalSettings } from "$lib/stores/global-settings-store";
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";

  let showLyricTran = $state(true);
  let showLyricRoma = $state(true);
  let showWordLyrics = $state(true);
  let enableAmllDb = $state(true);
  let initialized = false;

  $effect(() => {
    showLyricTran;
    showLyricRoma;
    showWordLyrics;
    enableAmllDb;
    if (initialized) {
      globalSettings.patch({ showLyricTran, showLyricRoma, showWordLyrics, enableAmllDb });
    }
  });

  onMount(() => {
    const unsub = globalSettings.subscribe((s) => {
      showLyricTran = s.showLyricTran;
      showLyricRoma = s.showLyricRoma;
      showWordLyrics = s.showWordLyrics;
      enableAmllDb = s.enableAmllDb;
    });
    setTimeout(() => { initialized = true; }, 0);
    return unsub;
  });
</script>

<div in:fly={{ y: 16, duration: 300, opacity: 0 }}>
  <div class="mb-1 text-xl font-bold text-stone-800 dark:text-stone-100">
    AMLL 歌词
  </div>
  <p class="mb-4 text-sm text-muted-foreground">
    Apple Music Like Lyrics 歌词引擎相关设置。
  </p>

  <div class="space-y-3">
    <SettingCard
      title="歌词来源"
      description="设置歌词获取的优先级和来源。"
    >
      <div class="flex flex-col gap-3">
        <label class="flex items-center gap-3">
          <Switch bind:checked={enableAmllDb} />
          <div class="flex flex-col gap-0.5">
            <span class="text-sm font-medium">AMLL 歌词库</span>
            <span class="text-xs text-muted-foreground">
              从 amll-ttml-db 获取逐字 TTML 歌词（优先）
            </span>
          </div>
        </label>
        <label class="flex items-center gap-3">
          <Switch bind:checked={showWordLyrics} />
          <div class="flex flex-col gap-0.5">
            <span class="text-sm font-medium">逐字歌词</span>
            <span class="text-xs text-muted-foreground">
              启用逐字级别的歌词显示（需要 YRC 或 TTML 歌词源）
            </span>
          </div>
        </label>
      </div>
    </SettingCard>

    <SettingCard
      title="歌词显示"
      description="控制歌词下方的附加行显示。"
    >
      <div class="flex flex-col gap-3">
        <label class="flex items-center gap-3">
          <Switch bind:checked={showLyricTran} />
          <div class="flex flex-col gap-0.5">
            <span class="text-sm font-medium">显示翻译</span>
            <span class="text-xs text-muted-foreground">
              在歌词下方显示翻译行（如果可用）
            </span>
          </div>
        </label>
        <label class="flex items-center gap-3">
          <Switch bind:checked={showLyricRoma} />
          <div class="flex flex-col gap-0.5">
            <span class="text-sm font-medium">显示罗马音</span>
            <span class="text-xs text-muted-foreground">
              在歌词下方显示罗马音行（如果可用）
            </span>
          </div>
        </label>
      </div>
    </SettingCard>
  </div>
</div>
