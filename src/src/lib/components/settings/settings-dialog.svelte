<script lang="ts">
  import { Settings, Info, X, Folder, MicVocal, Server } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";
  import GeneralPage from "./pages/common/general.svelte";
  import LocalPage from "./pages/common/local.svelte";
  import AmllPage from "./pages/common/amll.svelte";
  import AboutPage from "./pages/common/about.svelte";
  import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
  import * as Dialog from "$lib/components/ui/dialog";
  import { settingsDialogOpen } from "$lib/stores/global-ui-store";
  const version = __APP_VERSION__;

  let activeSection = $state<Section>("general");
  type Section = "general" | "local" | "amll" | "about";

  interface NavItem {
    key: Section;
    label: string;
    icon: typeof Settings;
  }

  let NAV_TOP: NavItem[] = $state([
    { key: "general", label: "常规设置", icon: Settings },
    { key: "amll", label: "歌词设置", icon: MicVocal },

    { key: "local", label: "本地设置", icon: Server },
  ]);
  let NAV_BOTTOM: NavItem[] = $state([
    { key: "about", label: "关于", icon: Info },
  ]);

  let open = $state(false);
  const unsub = settingsDialogOpen.subscribe((v) => (open = v));

  function onOpenChange(o: boolean) {
    settingsDialogOpen.set(o);
  }
</script>

<Dialog.Root {open} {onOpenChange}>
  <Dialog.Portal>
    <Dialog.Overlay />
    <Dialog.Content
      class="w-[1024px] max-w-[calc(100vw-40px)] sm:max-w-[1024px] h-[85vh] p-0 gap-0"
      showCloseButton={false}
    >
      <div class="flex h-full w-full overflow-hidden rounded-lg">
        <!-- 左侧导航 -->
        <div class="flex w-[240px] shrink-0 flex-col bg-muted/50">
          <div class="px-5 pt-5 pb-2">
            <h1 class="text-[26px] font-bold leading-none tracking-tight">
              设置
            </h1>
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
          <div class="mt-auto flex flex-col gap-0.5 px-3 pt-2 pb-5">
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
            <span class="text-xs text-muted-foreground">Version {version}</span>
          </div>
        </div>

        <!-- 右侧内容 -->
        <div class="flex flex-1 flex-col bg-background">
          <!-- 关闭按钮 -->
          <button
            class="absolute inset-e-4 top-4 z-10 opacity-70 transition-opacity hover:opacity-100"
            onclick={() => settingsDialogOpen.set(false)}
          >
            <X size={18} />
          </button>
          <ScrollArea class="h-full w-full">
            <div class="p-10">
              {#if activeSection === "general"}<GeneralPage />{/if}
              {#if activeSection === "local"}<LocalPage />{/if}
              {#if activeSection === "amll"}<AmllPage />{/if}
              {#if activeSection === "about"}<AboutPage />{/if}
            </div>
          </ScrollArea>
        </div>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
