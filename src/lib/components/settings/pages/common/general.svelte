<script lang="ts">
  import SettingCard from "$lib/components/cards/settings-card.svelte";
  import { Kbd, KbdGroup } from "$lib/components/ui/kbd";
  import * as Accordion from "$lib/components/ui/accordion/index.js";
  import { Button } from "$lib/components/ui/button";
  import { toast } from "svelte-sonner";
  import { Upload, Trash2, Sun, Moon, ChevronDown, Settings } from "@lucide/svelte";
  import { setMode, userPrefersMode } from "mode-watcher";
  import { fly } from "svelte/transition";
  import { globalSettings } from "$lib/stores/global-settings-store";
</script>

<div class="space-y-8" in:fly={{ y: 16, duration: 300, opacity: 0 }}>
  <!-- 界面 -->
  <div>
    <div class="mb-1 text-xl font-bold text-stone-800 dark:text-stone-100">
      界面
    </div>
    <div class="space-y-3">
      <!-- 主题 -->
      <SettingCard title="界面主题" description="选择浅色或暗色界面主题。">
        <div class="flex gap-1.5">
          <Button
            variant={userPrefersMode.current === "light"
              ? "secondary"
              : "ghost"}
            size="sm"
            onclick={() => setMode("light")}
          >
            <Sun size={13} class="mr-1.5" />
            浅色
          </Button>
          <Button
            variant={userPrefersMode.current === "dark" ? "secondary" : "ghost"}
            size="sm"
            onclick={() => setMode("dark")}
          >
            <Moon size={13} class="mr-1.5" />
            暗色
          </Button>
          <Button
            variant={userPrefersMode.current === "system" ? "secondary" : "ghost"}
            size="sm"
            onclick={() => setMode("system")}
          >
            <Settings size={13} class="mr-1.5" />
            跟随系统
          </Button>
        </div>
      </SettingCard>

      <!-- 语言 -->
      <SettingCard
        title="界面语言"
        description="切换界面显示语言。部分界面可能需要刷新后生效。"
      >
        <div class="flex gap-1.5">
          <Button
            variant={$globalSettings.language === "zh-cn"
              ? "secondary"
              : "ghost"}
            size="sm"
            onclick={() => globalSettings.patch({ language: "zh-cn" })}
          >
            中文
          </Button>
          <Button
            variant={$globalSettings.language === "en" ? "secondary" : "ghost"}
            size="sm"
            onclick={() => globalSettings.patch({ language: "en" })}
          >
            English
          </Button>
        </div>
      </SettingCard>

      <!-- 字体 -->
      <SettingCard
        title="全局字体"
        description="应用于整个应用的字体。输入 CSS font-family 值（如 &quot;HarmonyOS SansSC&quot; 或 &quot;MiSans, Noto Sans SC&quot;）。"
      >
        <div class="flex items-center gap-2 min-w-0 max-w-[320px]">
          <input
            type="text"
            class="h-9 flex-1 min-w-0 rounded-md border border-border bg-background px-3 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
            placeholder={$globalSettings.fontFamily === "default" ? "Figtree Variable" : ""}
            value={$globalSettings.fontFamily === "default" ? "" : $globalSettings.fontFamily}
            oninput={(e) => {
              const val = (e.target as HTMLInputElement).value.trim();
              globalSettings.patch({ fontFamily: val || "default" });
            }}
          />
          <Button
            variant="ghost"
            size="sm"
            disabled={$globalSettings.fontFamily === "default"}
            onclick={() => globalSettings.patch({ fontFamily: "default" })}
          >
            恢复默认
          </Button>
        </div>
      </SettingCard>
    </div>
  </div>

  <!-- 快捷键 -->
  <div>
    <div class="mb-1 text-xl font-bold text-stone-800 dark:text-stone-100">
      快捷键
    </div>
    <p class="mb-5 text-sm text-muted-foreground">
      当前版本快捷键为只读，后续版本支持自定义。
    </p>

    <div class="space-y-6">
      <!-- {#each GROUPS as group}
				<div>
					<Accordion.Root type="single">
						<Accordion.Item value="item-1">
							<Accordion.Trigger
								class="mb-2 text-sm font-semibold text-stone-500 dark:text-stone-400"
								>{group}</Accordion.Trigger
							>
							<Accordion.Content>
								{#if shortcutsByGroup[group].length === 0}
									<p class="text-xs text-muted-foreground">暂无快捷键</p>
								{:else}
									<div class="space-y-2">
										{#each shortcutsByGroup[group] as def}
											<SettingCard title={def.description}>
												<KbdGroup>
													{#if def.ctrl}<Kbd>Ctrl</Kbd>{/if}
													{#if def.shift}<Kbd>Shift</Kbd>{/if}
													{#if def.alt}<Kbd>Alt</Kbd>{/if}
													<Kbd>{def.key}</Kbd>
												</KbdGroup>
											</SettingCard>
										{/each}
									</div>
								{/if}</Accordion.Content
							>
						</Accordion.Item>
					</Accordion.Root>
				</div>
			{/each} -->
    </div>
  </div>
</div>
