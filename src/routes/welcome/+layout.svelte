<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { globalSettings } from "$lib/stores/global-settings-store";
  import { Button } from "$lib/components/ui/button";
  import { ArrowRight, ArrowLeft, CircleCheck } from "@lucide/svelte";
  const STEPS = [
    { path: "/welcome", label: "欢迎" },
    { path: "/welcome/adapter", label: "适配器" },
    { path: "/welcome/theme", label: "主题" },
    { path: "/welcome/local", label: "本地" },
  ];

  let currentStepIndex = $derived(
    STEPS.findIndex((s) => $page.url.pathname === s.path),
  );
  let isFirst = $derived(currentStepIndex === 0);
  let isLast = $derived(currentStepIndex === STEPS.length - 1);

  function goNext() {
    if (!isLast) goto(STEPS[currentStepIndex + 1].path);
  }

  function goPrev() {
    if (!isFirst) goto(STEPS[currentStepIndex - 1].path);
  }

  function skipWizard() {
    globalSettings.completeWelcome();
    goto("/");
  }

  function completeWizard() {
    globalSettings.completeWelcome();
    goto("/");
  }

  let { children } = $props();
</script>

<div class="welcome-container">
  <!-- Background decoration -->
  <div class="bg-decoration">
    <div class="bg-circle bg-circle-1"></div>
    <div class="bg-circle bg-circle-2"></div>
    <div class="bg-circle bg-circle-3"></div>
  </div>

  <!-- Skip button -->
  <button class="skip-btn" onclick={skipWizard}>
    跳过引导
    <ArrowRight class="size-3.5" />
  </button>

  <!-- Child content -->
  <div class="step-content">
    {#key $page.url.pathname}
      <div class="step-slot">
        {@render children?.()}
      </div>
    {/key}
  </div>

  <!-- Navigation -->
  {#if !isFirst}
    <div class="step-nav">
      <Button variant="outline" onclick={goPrev}>
        <ArrowLeft class="size-4" />
        上一步
      </Button>

      {#if !isLast}
        <Button onclick={goNext}>
          下一步
          <ArrowRight class="size-4" />
        </Button>
      {:else}
        <Button onclick={completeWizard}>
          <CircleCheck class="size-4" />
          完成设置
        </Button>
      {/if}
    </div>
  {/if}

  <!-- Step indicator -->
  <div class="step-indicator">
    {#each STEPS as step, i}
      <button
        class="step-dot"
        class:active={i === currentStepIndex}
        class:completed={i < currentStepIndex}
        onclick={() => goto(step.path)}
        aria-label={step.label}
      >
        {#if i < currentStepIndex}
          <CircleCheck class="size-3" />
        {:else}
          {i + 1}
        {/if}
      </button>
      {#if i < STEPS.length - 1}
        <div class="step-line" class:filled={i < currentStepIndex}></div>
      {/if}
    {/each}
  </div>
</div>

<style>
  .welcome-container {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    padding: 2rem;
    overflow: hidden;
  }

  /* ── Background Decoration ── */
  .bg-decoration {
    position: fixed;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }

  .bg-circle {
    position: absolute;
    border-radius: 50%;
    opacity: 0.15;
    filter: blur(80px);
  }

  .bg-circle-1 {
    width: 500px;
    height: 500px;
    background: var(--primary);
    top: -150px;
    right: -100px;
    animation: float1 12s ease-in-out infinite;
  }

  .bg-circle-2 {
    width: 350px;
    height: 350px;
    background: var(--chart-1);
    bottom: -100px;
    left: -80px;
    animation: float2 15s ease-in-out infinite;
  }

  .bg-circle-3 {
    width: 250px;
    height: 250px;
    background: var(--chart-2);
    top: 50%;
    left: 50%;
    animation: float3 10s ease-in-out infinite;
  }

  @keyframes float1 {
    0%,
    100% {
      transform: translate(0, 0) scale(1);
    }
    33% {
      transform: translate(-30px, 20px) scale(1.1);
    }
    66% {
      transform: translate(20px, -10px) scale(0.95);
    }
  }

  @keyframes float2 {
    0%,
    100% {
      transform: translate(0, 0) scale(1);
    }
    33% {
      transform: translate(20px, -15px) scale(1.05);
    }
    66% {
      transform: translate(-15px, 10px) scale(0.9);
    }
  }

  @keyframes float3 {
    0%,
    100% {
      transform: translate(0, 0) scale(1);
    }
    50% {
      transform: translate(-15px, -15px) scale(1.08);
    }
  }

  /* ── Skip Button ── */
  .skip-btn {
    position: fixed;
    top: 1.5rem;
    right: 1.5rem;
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    font-size: 0.8125rem;
    color: var(--muted-foreground);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0.375rem 0.75rem;
    border-radius: 0.5rem;
    transition: all 0.2s;
    z-index: 10;
  }

  .skip-btn:hover {
    color: var(--foreground);
    background: var(--muted);
  }

  /* ── Step Indicator ── */
  .step-indicator {
    display: flex;
    align-items: center;
    gap: 0;
    margin-top: 2rem;
    z-index: 1;
  }

  .step-dot {
    width: 2rem;
    height: 2rem;
    border-radius: 50%;
    border: 2px solid var(--border);
    background: var(--background);
    color: var(--muted-foreground);
    font-size: 0.75rem;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.3s;
    flex-shrink: 0;
  }

  .step-dot.active {
    border-color: var(--primary);
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .step-dot.completed {
    border-color: var(--primary);
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .step-line {
    width: 3rem;
    height: 2px;
    background: var(--border);
    transition: background 0.3s;
  }

  .step-line.filled {
    background: var(--primary);
  }

  /* ── Step Content ── */
  .step-content {
    width: 100%;
    max-width: 520px;
    z-index: 1;
  }

  .step-slot {
    animation: stepFlyIn 300ms cubic-bezier(0.16, 1, 0.3, 1) both;
  }

  @keyframes stepFlyIn {
    from {
      opacity: 0;
      transform: translateY(24px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  /* ── Navigation ── */
  .step-nav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    max-width: 520px;
    margin-top: 1.5rem;
    z-index: 1;
  }
</style>
