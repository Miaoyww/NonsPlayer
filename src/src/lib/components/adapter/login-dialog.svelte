<script lang="ts">
  import { onMount } from "svelte";
  import * as Dialog from "$lib/components/ui/dialog";
  import Button from "$lib/components/ui/button/button.svelte";
  import { loginQrUrl, checkLogin } from "$lib/services/adapter-service";
  import type { Account } from "$lib/types/account";
  import { QrCode, Check, X, AlertCircle, Loader2 } from "@lucide/svelte";

  type LoginState = "loading" | "waiting" | "scanned" | "confirmed" | "timeout" | "error";

  interface Props {
    adapter: string;
    open?: boolean;
    onlogin?: (account: Account) => void;
  }

  let {
    adapter,
    open = $bindable(false),
    onlogin,
  }: Props = $props();

  let loginState = $state<LoginState>("loading");
  let qrUrl = $state("");
  let key = $state("");
  let message = $state("");
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  const qrImageUrl = $derived(
    qrUrl
      ? `https://api.qrserver.com/v1/create-qr-code/?size=220x220&data=${encodeURIComponent(qrUrl)}`
      : "",
  );

  async function startLogin() {
    loginState = "loading";
    message = "";
    console.log("[login] 开始获取二维码, adapter:", adapter);
    try {
      const [k, url] = await loginQrUrl(adapter);
      console.log("[login] 获取二维码成功, key:", k, "url:", url);
      key = k;
      qrUrl = url;
      loginState = "waiting";
      startPolling();
    } catch (e) {
      console.error("[login] 获取二维码失败:", e);
      loginState = "error";
      message = `获取二维码失败: ${e}`;
    }
  }

  function startPolling() {
    stopPolling();
    console.log("[login] 开始轮询登录状态, key:", key);
    pollTimer = setInterval(async () => {
      try {
        console.log("[login] 检查登录状态...");
        const result = await checkLogin(adapter, key);
        console.log("[login] 登录状态:", result.status);
        switch (result.status) {
          case "waiting":
            break;
          case "scanned":
            loginState = "scanned";
            break;
          case "confirmed":
            console.log("[login] 登录成功, account:", result.account);
            loginState = "confirmed";
            stopPolling();
            setTimeout(() => {
              onlogin?.(result.account);
              open = false;
            }, 800);
            break;
          case "timeout":
            console.warn("[login] 二维码已过期");
            loginState = "timeout";
            message = "二维码已过期，请重新获取";
            stopPolling();
            break;
          case "cancelled":
            console.warn("[login] 登录已取消");
            loginState = "error";
            message = "登录已取消";
            stopPolling();
            break;
        }
      } catch (e) {
        console.warn("[login] 轮询出错:", e);
        // Ignore polling errors, keep trying
      }
    }, 2000);
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  // 监听 open 变化：程序化打开时开始登录流程
  $effect(() => {
    console.log("[login] $effect open 变化, open:", open);
    if (open) {
      startLogin();
    }
  });

  // 仅处理用户手动关闭弹窗（点X、按Escape）
  function handleOpenChange(o: boolean) {
    console.log("[login] handleOpenChange 用户关闭, open:", o);
    open = o;
    if (!o) {
      stopPolling();
    }
  }

  console.log("[login] LoginDialog 组件初始化, adapter:", adapter);

  function handleRetry() {
    startLogin();
  }

  onMount(() => {
    return () => stopPolling();
  });
</script>

<Dialog.Root open={open} onOpenChange={handleOpenChange}>
  <Dialog.Portal>
    <Dialog.Overlay />
    <Dialog.Content class="sm:max-w-md" showCloseButton={loginState !== "loading"}>
      <Dialog.Header>
        <Dialog.Title class="flex items-center gap-2">
          <QrCode class="size-5" />
          扫码登录
        </Dialog.Title>
        <Dialog.Description>
          请使用手机端 App 扫描二维码登录
        </Dialog.Description>
      </Dialog.Header>

      <div class="flex flex-col items-center gap-4 py-4">
        {#if loginState === "loading"}
          <div class="flex flex-col items-center gap-3 py-8">
            <Loader2 class="size-10 animate-spin text-muted-foreground" />
            <p class="text-sm text-muted-foreground">正在生成二维码...</p>
          </div>
        {:else if loginState === "waiting" || loginState === "scanned"}
          <div class="relative">
            <img
              src={qrImageUrl}
              alt="登录二维码"
              class="size-[220px] rounded-lg border border-border"
              class:opacity-40={loginState === "scanned"}
            />
            {#if loginState === "scanned"}
              <div class="absolute inset-0 flex flex-col items-center justify-center gap-2">
                <div class="size-14 rounded-full bg-primary/90 flex items-center justify-center">
                  <Check class="size-8 text-primary-foreground" />
                </div>
                <p class="text-sm font-medium text-foreground">已扫码</p>
                <p class="text-xs text-muted-foreground">请在手机上确认登录</p>
              </div>
            {/if}
          </div>
          {#if loginState === "waiting"}
            <p class="text-sm text-muted-foreground">等待扫码中...</p>
          {/if}
        {:else if loginState === "confirmed"}
          <div class="flex flex-col items-center gap-3 py-8">
            <div class="size-14 rounded-full bg-emerald-500/20 flex items-center justify-center">
              <Check class="size-8 text-emerald-500" />
            </div>
            <p class="text-sm font-medium text-foreground">登录成功</p>
          </div>
        {:else if loginState === "timeout"}
          <div class="flex flex-col items-center gap-3 py-8">
            <div class="size-14 rounded-full bg-amber-500/20 flex items-center justify-center">
              <AlertCircle class="size-8 text-amber-500" />
            </div>
            <p class="text-sm font-medium text-foreground">{message}</p>
            <Button variant="outline" size="sm" class="cursor-pointer" onclick={handleRetry}>
              重新获取二维码
            </Button>
          </div>
        {:else if loginState === "error"}
          <div class="flex flex-col items-center gap-3 py-8">
            <div class="size-14 rounded-full bg-destructive/20 flex items-center justify-center">
              <X class="size-8 text-destructive" />
            </div>
            <p class="text-sm font-medium text-foreground">{message || "登录失败"}</p>
            <Button variant="outline" size="sm" class="cursor-pointer" onclick={handleRetry}>
              重试
            </Button>
          </div>
        {/if}
      </div>

      {#if loginState === "waiting" || loginState === "scanned"}
        <Dialog.Footer>
          <Button variant="outline" size="sm" class="cursor-pointer" onclick={() => (open = false)}>
            取消
          </Button>
        </Dialog.Footer>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
