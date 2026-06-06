<script lang="ts">
  import SettingCard from "$lib/components/cards/settings-card.svelte";
  import { Switch } from "$lib/components/ui/switch";
  import { Button } from "$lib/components/ui/button";
  import { globalSettings } from "$lib/stores/global-settings-store";
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { AlignStartVertical, AlignCenterVertical, AlignEndVertical } from "@lucide/svelte";

  // ── Local state mirrored from store ──
  let showLyricTran = $state(true);
  let showLyricRoma = $state(true);
  let lyricWordFadeWidth = $state(0.5);
  let lyricEnableBlur = $state(true);
  let lyricEnableSpring = $state(true);
  let lyricEnableScale = $state(true);
  let lyricHidePassedLines = $state(false);
  let lyricAlignAnchor = $state<"top" | "bottom" | "center">("center");
  let lyricAlignPosition = $state(0.35);
  let lyricFontFamily = $state("follow");
  let englishLyricFont = $state("follow");
  let japaneseLyricFont = $state("follow");
  let koreanLyricFont = $state("follow");
  let playerBackgroundFps = $state(30);
  let playerBackgroundFlowSpeed = $state(4);
  let playerBackgroundRenderScale = $state(0.5);
  let playerBackgroundStaticMode = $state(false);

  let initialized = false;

  $effect(() => {
    showLyricTran; showLyricRoma;
    lyricWordFadeWidth; lyricEnableBlur; lyricEnableSpring; lyricEnableScale;
    lyricHidePassedLines; lyricAlignAnchor; lyricAlignPosition;
    lyricFontFamily; englishLyricFont; japaneseLyricFont; koreanLyricFont;
    playerBackgroundFps; playerBackgroundFlowSpeed; playerBackgroundRenderScale;
    playerBackgroundStaticMode;
    if (initialized) {
      globalSettings.patch({
        showLyricTran, showLyricRoma,
        lyricWordFadeWidth, lyricEnableBlur, lyricEnableSpring, lyricEnableScale,
        lyricHidePassedLines, lyricAlignAnchor, lyricAlignPosition,
        lyricFontFamily, englishLyricFont, japaneseLyricFont, koreanLyricFont,
        playerBackgroundFps, playerBackgroundFlowSpeed, playerBackgroundRenderScale,
        playerBackgroundStaticMode,
      });
    }
  });

  onMount(() => {
    const unsub = globalSettings.subscribe((s) => {
      showLyricTran = s.showLyricTran;
      showLyricRoma = s.showLyricRoma;
      lyricWordFadeWidth = s.lyricWordFadeWidth;
      lyricEnableBlur = s.lyricEnableBlur;
      lyricEnableSpring = s.lyricEnableSpring;
      lyricEnableScale = s.lyricEnableScale;
      lyricHidePassedLines = s.lyricHidePassedLines;
      lyricAlignAnchor = s.lyricAlignAnchor;
      lyricAlignPosition = s.lyricAlignPosition;
      lyricFontFamily = s.lyricFontFamily ?? "follow";
      englishLyricFont = s.englishLyricFont ?? "follow";
      japaneseLyricFont = s.japaneseLyricFont ?? "follow";
      koreanLyricFont = s.koreanLyricFont ?? "follow";
      playerBackgroundFps = s.playerBackgroundFps;
      playerBackgroundFlowSpeed = s.playerBackgroundFlowSpeed;
      playerBackgroundRenderScale = s.playerBackgroundRenderScale;
      playerBackgroundStaticMode = s.playerBackgroundStaticMode;
    });
    setTimeout(() => { initialized = true; }, 0);
    return unsub;
  });

  function round1(n: number) { return Math.round(n * 10) / 10; }
  function round2(n: number) { return Math.round(n * 100) / 100; }
</script>

<div in:fly={{ y: 16, duration: 300, opacity: 0 }}>
  <div class="mb-1 text-xl font-bold text-stone-800 dark:text-stone-100">
    歌词设置
  </div>
  <p class="mb-4 text-sm text-muted-foreground">
    控制歌词显示效果和背景动画行为。AMLL 歌词库会始终优先获取。
  </p>

  <div class="space-y-3">
    <!-- ── 歌词显示 ── -->
    <SettingCard title="显示翻译" description="在歌词下方显示翻译行（如果可用）">
      <Switch bind:checked={showLyricTran} />
    </SettingCard>

    <SettingCard title="显示罗马音" description="在歌词下方显示罗马音行（如果可用）">
      <Switch bind:checked={showLyricRoma} />
    </SettingCard>

    <!-- ── 歌词字体 ── -->
    <SettingCard
      title="歌词区域字体"
      description="主歌词区域的基础字体。跟随全局则使用全局字体设置。"
    >
      <div class="flex items-center gap-2 min-w-0 max-w-[320px]">
        <input
          type="text"
          class="h-9 flex-1 min-w-0 rounded-md border border-border bg-background px-3 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
          placeholder="跟随全局"
          value={lyricFontFamily === "follow" ? "" : lyricFontFamily}
          oninput={(e) => {
            const val = (e.target as HTMLInputElement).value.trim();
            lyricFontFamily = val || "follow";
          }}
        />
        <Button
          variant="ghost"
          size="sm"
          disabled={lyricFontFamily === "follow"}
          onclick={() => (lyricFontFamily = "follow")}
        >
          跟随全局
        </Button>
      </div>
    </SettingCard>

    <SettingCard
      title="英文歌词字体"
      description="歌词包含英文时使用的特定字体。跟随歌词则使用歌词区域字体。"
    >
      <div class="flex items-center gap-2 min-w-0 max-w-[320px]">
        <input
          type="text"
          class="h-9 flex-1 min-w-0 rounded-md border border-border bg-background px-3 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
          placeholder="跟随歌词"
          value={englishLyricFont === "follow" ? "" : englishLyricFont}
          oninput={(e) => {
            const val = (e.target as HTMLInputElement).value.trim();
            englishLyricFont = val || "follow";
          }}
        />
        <Button
          variant="ghost"
          size="sm"
          disabled={englishLyricFont === "follow"}
          onclick={() => (englishLyricFont = "follow")}
        >
          跟随歌词
        </Button>
      </div>
    </SettingCard>

    <SettingCard
      title="日语歌词字体"
      description="歌词包含日语（假名）时使用的特定字体。"
    >
      <div class="flex items-center gap-2 min-w-0 max-w-[320px]">
        <input
          type="text"
          class="h-9 flex-1 min-w-0 rounded-md border border-border bg-background px-3 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
          placeholder="跟随歌词"
          value={japaneseLyricFont === "follow" ? "" : japaneseLyricFont}
          oninput={(e) => {
            const val = (e.target as HTMLInputElement).value.trim();
            japaneseLyricFont = val || "follow";
          }}
        />
        <Button
          variant="ghost"
          size="sm"
          disabled={japaneseLyricFont === "follow"}
          onclick={() => (japaneseLyricFont = "follow")}
        >
          跟随歌词
        </Button>
      </div>
    </SettingCard>

    <SettingCard
      title="韩语歌词字体"
      description="歌词包含韩语（谚文）时使用的特定字体。"
    >
      <div class="flex items-center gap-2 min-w-0 max-w-[320px]">
        <input
          type="text"
          class="h-9 flex-1 min-w-0 rounded-md border border-border bg-background px-3 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
          placeholder="跟随歌词"
          value={koreanLyricFont === "follow" ? "" : koreanLyricFont}
          oninput={(e) => {
            const val = (e.target as HTMLInputElement).value.trim();
            koreanLyricFont = val || "follow";
          }}
        />
        <Button
          variant="ghost"
          size="sm"
          disabled={koreanLyricFont === "follow"}
          onclick={() => (koreanLyricFont = "follow")}
        >
          跟随歌词
        </Button>
      </div>
    </SettingCard>

    <!-- ── 歌词效果 ── -->
    <SettingCard
      title="歌词渐变宽度"
      description={`逐字淡入过渡宽度（${lyricWordFadeWidth.toFixed(2)}）`}
    >
      <div class="flex items-center gap-3 min-w-40">
        <span class="text-xs text-muted-foreground w-6 text-right">0</span>
        <input
          type="range" min="0" max="1" step="0.05"
          bind:value={lyricWordFadeWidth}
          class="h-1.5 flex-1 appearance-none rounded-full bg-muted cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:size-3 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-primary"
        />
        <span class="text-xs text-muted-foreground w-6">1</span>
      </div>
    </SettingCard>

    <SettingCard title="歌词模糊" description="为非焦点行启用模糊效果">
      <Switch bind:checked={lyricEnableBlur} />
    </SettingCard>

    <SettingCard title="使用弹簧动画" description="使用物理弹簧替代 CSS transition 驱动歌词位移">
      <Switch bind:checked={lyricEnableSpring} />
    </SettingCard>

    <SettingCard title="已过行缩放" description="已播放的歌词行略微缩小">
      <Switch bind:checked={lyricEnableScale} />
    </SettingCard>

    <SettingCard title="隐藏已过行" description="已播放完毕的歌词行完全隐藏">
      <Switch bind:checked={lyricHidePassedLines} />
    </SettingCard>

    <!-- ── 歌词对齐 ── -->
    <SettingCard
      title="歌词对齐"
      description={`歌词锚点位置（${lyricAlignAnchor === "center" ? "居中" : lyricAlignAnchor === "top" ? "顶部" : "底部"}，${(lyricAlignPosition * 100).toFixed(0)}%）`}
    >
      <div class="flex flex-col gap-2.5">
        <div class="inline-flex rounded-lg border border-border bg-muted p-0.5">
          <Button
            variant={lyricAlignAnchor === "top" ? "secondary" : "ghost"}
            size="sm"
            class="flex items-center gap-1.5 rounded-md {lyricAlignAnchor !== 'top' ? 'text-muted-foreground' : ''}"
            onclick={() => (lyricAlignAnchor = "top")}
          >
            <AlignStartVertical class="size-4" />
            顶部
          </Button>
          <Button
            variant={lyricAlignAnchor === "center" ? "secondary" : "ghost"}
            size="sm"
            class="flex items-center gap-1.5 rounded-md {lyricAlignAnchor !== 'center' ? 'text-muted-foreground' : ''}"
            onclick={() => (lyricAlignAnchor = "center")}
          >
            <AlignCenterVertical class="size-4" />
            居中
          </Button>
          <Button
            variant={lyricAlignAnchor === "bottom" ? "secondary" : "ghost"}
            size="sm"
            class="flex items-center gap-1.5 rounded-md {lyricAlignAnchor !== 'bottom' ? 'text-muted-foreground' : ''}"
            onclick={() => (lyricAlignAnchor = "bottom")}
          >
            <AlignEndVertical class="size-4" />
            底部
          </Button>
        </div>
        <div class="flex items-center gap-3 mt-1">
          <span class="text-xs text-muted-foreground w-6 text-right">0%</span>
          <input
            type="range" min="0" max="1" step="0.05"
            bind:value={lyricAlignPosition}
            class="h-1.5 flex-1 appearance-none rounded-full bg-muted cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:size-3 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-primary"
          />
          <span class="text-xs text-muted-foreground w-12">{(lyricAlignPosition * 100).toFixed(0)}%</span>
        </div>
      </div>
    </SettingCard>

    <!-- ── 背景动画 ── -->
    <SettingCard
      title="背景渲染帧率"
      description={`流体背景动画帧率（${playerBackgroundFps} FPS）`}
    >
      <div class="flex items-center gap-3">
        <span class="text-xs text-muted-foreground w-6 text-right">10</span>
        <input
          type="range" min="10" max="120" step="5"
          bind:value={playerBackgroundFps}
          class="h-1.5 flex-1 appearance-none rounded-full bg-muted cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:size-3 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-primary"
        />
        <span class="text-xs tabular-nums text-muted-foreground w-8">{playerBackgroundFps}</span>
      </div>
    </SettingCard>

    <SettingCard
      title="背景流动速度"
      description={`渐变流动速率（${playerBackgroundFlowSpeed.toFixed(1)}x）`}
    >
      <div class="flex items-center gap-3">
        <span class="text-xs text-muted-foreground w-6 text-right">0.1</span>
        <input
          type="range" min="0.1" max="10" step="0.1"
          bind:value={playerBackgroundFlowSpeed}
          class="h-1.5 flex-1 appearance-none rounded-full bg-muted cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:size-3 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-primary"
        />
        <span class="text-xs tabular-nums text-muted-foreground w-8">{playerBackgroundFlowSpeed.toFixed(1)}</span>
      </div>
    </SettingCard>

    <SettingCard
      title="背景渲染精度"
      description={`Canvas 内部渲染缩放（${playerBackgroundRenderScale.toFixed(2)}）— 降低可减少 GPU 压力`}
    >
      <div class="flex items-center gap-3">
        <span class="text-xs text-muted-foreground w-6 text-right">0.1</span>
        <input
          type="range" min="0.1" max="3" step="0.05"
          bind:value={playerBackgroundRenderScale}
          class="h-1.5 flex-1 appearance-none rounded-full bg-muted cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:size-3 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-primary"
        />
        <span class="text-xs tabular-nums text-muted-foreground w-8">{playerBackgroundRenderScale.toFixed(2)}</span>
      </div>
    </SettingCard>

    <SettingCard title="背景静态模式" description="固定背景流动状态，不随播放动态变化">
      <Switch bind:checked={playerBackgroundStaticMode} />
    </SettingCard>
  </div>
</div>
