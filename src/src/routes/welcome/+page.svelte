<script lang="ts">
  import { fly, fade, scale } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { goto } from "$app/navigation";
  import { Button } from "$lib/components/ui/button";
  import { NONSPLAYER_NAME, NONSPLAYER_TAGLINE, NONSPLAYER_DESCRIPTION } from "$lib/const";
  import { Music, ArrowRight } from "@lucide/svelte";
  import favicon from "$lib/assets/favicon.png";
</script>

<div class="welcome-step">
  <div class="welcome-logo">
    <div class="logo-icon">
      <img src={favicon} alt="App Logo" class="h-64 w-64 rounded-md" />
    </div>
  </div>
  <h1 class="welcome-title">
    {NONSPLAYER_NAME}
  </h1>
  <p class="welcome-tagline">
    {NONSPLAYER_TAGLINE}
  </p>
  <p class="welcome-desc">
    {NONSPLAYER_DESCRIPTION}
  </p>
  <div class="welcome-actions">
    <Button
      size="lg"
      class="start-btn"
      onclick={() => goto("/welcome/adapters")}
    >
      开始设置
      <ArrowRight class="size-5" />
    </Button>
  </div>
</div>

<style>
  .welcome-step {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 1rem;
  }

  .welcome-logo {
    margin-bottom: 0.5rem;
  }

  .welcome-title {
    font-size: 2.5rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--foreground);
    margin: 0;
  }

  .welcome-tagline {
    font-size: 1.125rem;
    color: var(--primary);
    font-weight: 500;
    margin: 0;
  }

  .welcome-desc {
    font-size: 0.9375rem;
    color: var(--muted-foreground);
    max-width: 380px;
    line-height: 1.6;
    margin: 0;
  }

  .welcome-actions {
    margin-top: 1rem;
  }
</style>
