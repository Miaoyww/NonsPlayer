<script lang="ts">
  import "../app.css";
  import "../lib/css/components.css";
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { globalSettings } from "$lib/stores/global-settings-store";
  import { adapterStore } from "$lib/stores/adapter-store.svelte";
  import { onMount } from "svelte";
  import TitleBar from "$lib/components/titlebar.svelte";
  import SettingsDialog from "$lib/components/settings/settings-dialog.svelte";
  import PlayerBar from "$lib/components/player-bar.svelte";
  import FullPlayer from "$lib/components/player/FullPlayer.svelte";
  import { playerUI } from "$lib/stores/player-ui-store.svelte";
  import { NONSPLAYER_NAME } from "$lib/const";
  import logo from "$lib/assets/logo.svg";
  import { isTauri } from "@tauri-apps/api/core";
  import { attachConsole } from "@tauri-apps/plugin-log";
  import { ModeWatcher } from "mode-watcher";

  let { children } = $props();

  onMount(async () => {
    if (isTauri()) {
      attachConsole()
        .catch((e) => console.error("[log] Failed to attach console:", e));
    }

    if (!$globalSettings.welcomeCompleted) {
      goto("/welcome");
      return;
    }

    // Initialize adapters from stored config
    try {
      await adapterStore.initialize({
        localMusicDirs: $globalSettings.localMusicFolders,
      });
    } catch (e) {
      console.warn("Failed to initialize adapters:", e);
    }
  });
</script>

<svelte:head>
  <title>{NONSPLAYER_NAME}</title>
  <meta name="title" content={NONSPLAYER_NAME} />
  <link rel="icon" type="image/x-icon" href={logo} />
</svelte:head>

<ModeWatcher />
<SettingsDialog />

<TitleBar />

<div class={isTauri() ? "pt-9" : ""}>
  <main class="h-[calc(100vh-2.5rem)] overflow-hidden">
    {@render children?.()}
  </main>
  <PlayerBar />
  <FullPlayer />
</div>

<style>
  * {
    margin: 0;
  }
</style>
