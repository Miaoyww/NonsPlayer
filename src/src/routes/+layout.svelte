<script lang="ts">
  import "../app.css";
  import "../lib/css/components.css";
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { globalSettings } from "$lib/stores/global-settings-store";
  import { onMount } from "svelte";
  import TitleBar from "$lib/components/titlebar.svelte";
  import SettingsDialog from "$lib/components/settings/settings-dialog.svelte";
  import { NONSPLAYER_NAME } from "$lib/const";
  import logo from "$lib/assets/logo.svg";
  import { isTauri } from "@tauri-apps/api/core";
  import { ModeWatcher } from "mode-watcher";

  let { children } = $props();

  onMount(() => {
    if (!$globalSettings.welcomeCompleted) {
      goto("/welcome");
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
  <main>
    {@render children?.()}
  </main>
</div>

<style>
  * {
    margin: 0;
  }
</style>
