<script lang="ts">
  import "../app.css";
  import "../lib/css/components.css";
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { globalSettings } from "$lib/stores/global-settings.store";
  import { onMount } from "svelte";
  import TitleBar from "$lib/components/titlebar.svelte";
  import SettingsDialog from "$lib/components/settings/settings-dialog.svelte";
  import { NONSPLAYER_NAME } from "$lib/const";
  import logo from "$lib/assets/logo.svg";
  import { isTauri } from "@tauri-apps/api/core";

  let { children } = $props();

  onMount(() => {
    const settings = globalSettings;
    let currentSettings: { welcomeCompleted: boolean } | null = null;
    const unsub = settings.subscribe((s) => {
      currentSettings = s;
    });

    if (
      currentSettings &&
      !currentSettings.welcomeCompleted &&
      !window.location.pathname.startsWith("/welcome")
    ) {
      goto("/welcome");
    }

    unsub();
  });
</script>

<svelte:head>
  <title>{NONSPLAYER_NAME}</title>
  <meta name="title" content={NONSPLAYER_NAME} />
  <link rel="icon" type="image/x-icon" href={logo} />
</svelte:head>

<TitleBar />

<SettingsDialog />

<div class={isTauri() ? 'pt-9' : ''}>
  <main>
    {@render children?.()}
  </main>
</div>

<style>
  * {
    margin: 0;
  }
</style>
