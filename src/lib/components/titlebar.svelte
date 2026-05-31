<script lang="ts">
  import { goto } from "$app/navigation";
  import { Minus, X, Settings, Square, SquaresUnite } from "@lucide/svelte";
  import { NONSPLAYER_NAME } from "$lib/const";
  import { Button } from "$lib/components/ui/button";
  import { isTauri } from "@tauri-apps/api/core";
  import { settingsDialogOpen } from "$lib/stores/global-ui-store";
  import NavigationButton from "./buttons/navigation-button.svelte";
  import favicon from "$lib/assets/favicon.png";
  import NavigationToButton from "$lib/components/buttons/navigation-to-button.svelte";
  import { onMount } from "svelte";
  import { page } from "$app/state";

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let appWindow = $state<any>(null);
  let isMaximized = $state(false);
  let titlebarVisible = $state(true);

  $effect(() => {
    if (!isTauri()) return;
    import("@tauri-apps/api/window").then(({ getCurrentWindow }) => {
      const win = getCurrentWindow();
      appWindow = win;
      win.isMaximized().then((v: boolean) => (isMaximized = v));
      win.onResized(async () => {
        isMaximized = await win.isMaximized();
      });
    });
  });

  function onDragMouseDown(e: MouseEvent) {
    if (e.target !== e.currentTarget) return;
    if (e.buttons === 1 && appWindow) {
      appWindow.startDragging();
    }
  }

  interface Navigations {
    href: string;
    label: string;
  }

  let navigations: Navigations[] = [
    { href: "/", label: "主页" },
    { href: "/explore", label: "发现" },
    { href: "/library", label: "音乐库" },
  ];

  $effect(() => {
    if (page.url.pathname.startsWith("/welcome")) {
      titlebarVisible = false;
    } else {
      titlebarVisible = true;
    }
  });
</script>

<div
  class="fixed top-0 right-0 left-0 flex h-10 items-stretch border-b border-border/30 bg-background select-none"
>
  <!-- 应用名 -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="flex w-28 shrink-0 items-center pl-4"
    onmousedown={onDragMouseDown}
  >
    <img src={favicon} alt="App Logo" class="size-6 rounded-md mr-2" />

    <span class="text-base font-semibold text-foreground/70 text-center">
      {NONSPLAYER_NAME}
    </span>
    <div class="mt-1 flex items-center">
      <NavigationButton />
      <NavigationButton direction="forward" />
    </div>
  </div>

  <!-- 导航 -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="flex flex-1 items-center justify-center gap-2"
    onmousedown={onDragMouseDown}
  >
    {#if titlebarVisible}
      {#each navigations as nav}
        <NavigationToButton href={nav.href} label={nav.label} />
      {/each}
    {/if}
  </div>

  <!-- 功能区 -->
  <div class="flex shrink-0 items-stretch">
    {#if titlebarVisible}
      <Button
        class="flex items-center justify-center px-3 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
        variant="ghost"
        onclick={() => settingsDialogOpen.set(true)}
        title="设置"
      >
        <Settings size={14} />
      </Button>

      <div class="mx-0.5 my-2 w-px bg-border/40"></div>
    {/if}
    <Button
      class="flex w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
      onclick={() => appWindow?.minimize()}
      variant="ghost"
      title="最小化"
    >
      <Minus size={14} />
    </Button>

    <Button
      class="flex w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
      onclick={() => appWindow?.toggleMaximize()}
      variant="ghost"
      title={isMaximized ? "向下还原" : "最大化"}
    >
      {#if isMaximized}
        <SquaresUnite size={11} />
      {:else}
        <Square size={11} />
      {/if}
    </Button>

    <Button
      class="close-btn flex w-11 items-center justify-center text-muted-foreground transition-colors"
      onclick={() => appWindow?.close()}
      variant="ghost"
      title="关闭"
    >
      <X size={14} />
    </Button>
  </div>
</div>
