<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Spinner } from "$lib/components/ui/spinner";

  let quote = $state("");
  let quoteFrom = $state("");
  let loading = $state(true);
  let displayedText = $state("");
  let isTyping = $state(false);

  let fetchTimer: ReturnType<typeof setInterval>;
  let typeTimer: ReturnType<typeof setInterval>;

  function getGreeting(): string {
    const h = new Date().getHours();
    if (h < 6) return "夜深了";
    if (h < 9) return "早上好";
    if (h < 12) return "上午好";
    if (h < 14) return "中午好";
    if (h < 18) return "下午好";
    return "晚上好";
  }

  let greeting = $state(getGreeting());

  function startTyping(text: string) {
    clearInterval(typeTimer);
    displayedText = "";
    isTyping = true;
    let i = 0;
    typeTimer = setInterval(() => {
      if (i <= text.length) {
        displayedText = text.slice(0, i);
        i++;
      } else {
        clearInterval(typeTimer);
        isTyping = false;
      }
    }, 100);
  }

  async function fetchQuote() {
    loading = true;
    try {
      const res = await fetch("https://v1.hitokoto.cn/?encode=json");
      const data = await res.json();
      quote = data.hitokoto;
      quoteFrom = data.from || "";
      startTyping(quote);
    } catch {
      quote = "受尽苦难而不厌，此乃阿修罗之道。";
      quoteFrom = "";
      startTyping(quote);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    fetchQuote();
    fetchTimer = setInterval(fetchQuote, 360_000);
  });

  onDestroy(() => {
    clearInterval(fetchTimer);
    clearInterval(typeTimer);
  });
</script>

<div class="flex flex-col gap-1 shrink-0">
  <p class="text-xl font-bold text-foreground">{greeting}</p>
  <p class="text-sm font-medium text-muted-foreground">
    {#if loading}
      <Spinner class="size-4" />
    {:else}
      <span>{displayedText}</span>
      {#if isTyping}
        <span class="inline-block w-0.5 h-3.5 bg-primary align-middle ml-0.5 animate-pulse"></span>
      {/if}
      {#if !isTyping && quoteFrom}
        <span class="text-muted-foreground/60">—— {quoteFrom}</span>
      {/if}
    {/if}
  </p>
</div>
