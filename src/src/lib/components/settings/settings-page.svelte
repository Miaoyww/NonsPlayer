<script lang="ts">
  import { Settings, Info } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";
  import GeneralPage from "$lib/components/settings/pages/common/general.svelte";
  import AboutPage from "$lib/components/settings/pages/common/about.svelte";
  import { fly } from "svelte/transition";
  import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
  const version = __APP_VERSION__;

  let activeSection = $state<Section>("general");
  type Section = "general" | "about";

  interface NavItem {
    key: Section;
    label: string;
    icon: typeof Settings;
  }

  let NAV_TOP: NavItem[] = $state([
    { key: "general", label: "常规", icon: Settings },
  ]);
  let NAV_BOTTOM: NavItem[] = $state([
    { key: "about", label: "关于", icon: Info },
  ]);
</script>

<div
  class="h-dvh w-screen flex items-center justify-center overflow-hidden bg-linear-to-br from-slate-100 to-stone-200 dark:from-slate-900 dark:to-stone-900"
>
  <div
    class="flex w-[calc(100vw-40px)] max-w-[1024px] h-[85vh] overflow-hidden rounded-xl border shadow-xl"
    in:fly={{ y: 16, duration: 300, opacity: 0 }}
  >
    <!-- 左侧导航 -->
    <div class="flex w-[240px] shrink-0 flex-col bg-muted/50">
      <div class="px-5 pt-5 pb-2">
        <h1 class="text-[26px] font-bold leading-none tracking-tight">设置</h1>
        <p class="mt-1.5 text-sm text-muted-foreground">个性化与全局设置</p>
      </div>
      <div class="flex flex-1 flex-col gap-0.5 px-3 pt-3">
        {#each NAV_TOP as item}
          <Button
            class="w-full cursor-pointer justify-start gap-2.5 px-3 h-9"
            variant={activeSection === item.key ? "secondary" : "ghost"}
            onclick={() => (activeSection = item.key)}
          >
            <item.icon size={18} />
            <span class="text-sm">{item.label}</span>
          </Button>
        {/each}
      </div>
      <div class="mt-auto flex flex-col gap-0.5 px-3 pt-2">
        {#each NAV_BOTTOM as item}
          <Button
            class="w-full cursor-pointer justify-start gap-2.5 px-3 h-9"
            variant={activeSection === item.key ? "secondary" : "ghost"}
            onclick={() => (activeSection = item.key)}
          >
            <item.icon size={18} />
            <span class="text-sm">{item.label}</span>
          </Button>
        {/each}
      </div>
      <div class="flex flex-col gap-1 px-5 pb-5">
        <div class="flex items-center gap-2">
          <span class="text-sm font-semibold">NonsPlayer</span>
        </div>
        <span class="text-xs text-muted-foreground"
          >Version {version}</span
        >
      </div>
    </div>

    <!-- 右侧内容 -->
    <div class="flex flex-1 flex-col bg-background">
      <ScrollArea class="h-full w-full">
        <div class="p-10">
          {#if activeSection === "general"}<GeneralPage />{/if}
          {#if activeSection === "about"}<AboutPage />{/if}
        </div>
      </ScrollArea>
    </div>
  </div>
</div>
