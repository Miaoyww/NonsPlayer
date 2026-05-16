<script lang="ts">
  import "../app.css";
  import '../lib/css/components.css';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { globalSettings } from '$lib/stores/global-settings.store';
  import { onMount } from 'svelte';

  let { children } = $props();

  onMount(() => {
    const settings = globalSettings;
    let currentSettings: { welcomeCompleted: boolean } | null = null;
    const unsub = settings.subscribe((s) => {
      currentSettings = s;
    });
    
    if (currentSettings && !currentSettings.welcomeCompleted && window.location.pathname !== '/welcome') {
      goto('/welcome');
    }
    
    unsub();
  });
</script>

<div>
  <main>
    {@render children?.()}
  </main>
</div>
