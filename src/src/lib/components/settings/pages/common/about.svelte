<script lang="ts">
  // 引入了 MessageCircle 作为联系我们的图标
  import { ExternalLink, FileText, MessageCircle, RefreshCw } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";
  import { Label } from "$lib/components/ui/label";
  import SettingCard from "$lib/components/cards/settings-card.svelte";
  import * as Dialog from "$lib/components/ui/dialog"; // 引入标准 Dialog
  import { NONSPLAYER_NAME } from "$lib/const";
  import { isTauri } from "@tauri-apps/api/core";
  import { openUrl as tauriOpenUrl } from "@tauri-apps/plugin-opener";
  import { fly } from "svelte/transition";
  import { checkForUpdateInteractive } from "$lib/services/update-service";
  import { toast } from "svelte-sonner";
  const version = __APP_VERSION__;

  import favicon from "$lib/assets/favicon.png";

  function openUrl(url: string) {
    tauriOpenUrl(url);
  }

  let checking = $state(false);

  async function checkForUpdates() {
    if (checking) return;
    checking = true;
    const toastId = toast.loading("正在检查更新...");

    await checkForUpdateInteractive({
      onStatus(status) {
        switch (status.stage) {
          case "none":
            toast.success("已是最新版本", { id: toastId });
            checking = false;
            break;
          case "available":
            toast.success(`发现新版本 v${status.version}`, { id: toastId });
            break;
          case "downloading":
            toast.loading(`正在下载更新 ${status.progress}%`, { id: toastId });
            break;
          case "installing":
            toast.loading("正在安装更新...", { id: toastId });
            break;
          case "done":
            toast.success("更新完成，即将重启", { id: toastId });
            break;
          case "error":
            toast.error(`更新失败: ${status.message}`, { id: toastId });
            checking = false;
            break;
        }
      },
    });
  }
</script>

<div
  class="grid h-full grid-rows-2"
  in:fly={{ y: 16, duration: 300, opacity: 0 }}
>
  <div>
    <div class="mb-1 flex justify-center">
      <img src={favicon} alt="App Logo" class="h-64 w-64 rounded-md" />
    </div>
    <div class="mb-6 flex flex-col items-center gap-2 text-center">
      <h2
        class="text-3xl font-extrabold tracking-wide text-stone-800 dark:text-stone-100"
      >
        {NONSPLAYER_NAME}
      </h2>
      <p class="text-sm text-muted-foreground">
        多音源 | 高性能 | 跨平台 | 多音源
      </p>
    </div>
    <div class="space-y-3">
      <SettingCard title="版本号" description="当前应用版本。">
        <Label>v{version}</Label>
      </SettingCard>

      <SettingCard
        title="联系我们"
        description="加入我们的社区交流或反馈问题。"
      >
        <Dialog.Root>
          <Dialog.Trigger>
            <Button variant="outline" size="sm">
              <MessageCircle size={13} class="mr-1.5" />
              联系我们
            </Button>
          </Dialog.Trigger>
          <Dialog.Content class="sm:max-w-[425px]">
            <Dialog.Header>
              <Dialog.Title>联系我们</Dialog.Title>
              <Dialog.Description
                >请选择你要加入的平台进行交流或反馈问题。</Dialog.Description
              >
            </Dialog.Header>
            <div class="grid gap-3 py-4">
              <Button
                variant="outline"
                onclick={() => openUrl("https://qm.qq.com/q/5zKXit2G7m")}
                class="w-full justify-start"
              >
                <MessageCircle size={16} class="mr-2" />
                加入 QQ 群
              </Button>

              <Button
                variant="outline"
                onclick={() => openUrl("https://discord.gg/")}
                class="w-full justify-start"
                disabled
              >
                <ExternalLink size={16} class="mr-2" />
                加入 Discord 服务器
              </Button>
            </div>
          </Dialog.Content>
        </Dialog.Root>
      </SettingCard>

      <SettingCard title="开源许可" description="本项目基于 GPL-3.0 协议开源。">
        <Button
          variant="outline"
          size="sm"
          onclick={() =>
            openUrl(
              "https://github.com/Miaoyww/NonsPlayer/blob/next-svelte/LICENSE",
            )}
        >
          <FileText size={13} class="mr-1.5" />
          GPL-3.0
        </Button>
      </SettingCard>

      <SettingCard title="GitHub 仓库" description="查看源代码或提交 Issue。">
        <Button
          variant="outline"
          size="sm"
          onclick={() => openUrl("https://github.com/Miaoyww/NonsPlayer")}
        >
          <ExternalLink size={13} class="mr-1.5" />
          Miaoyww/NonsPlayer
        </Button>
      </SettingCard>

      {#if isTauri()}
        <SettingCard title="检查更新" description="跟上新版本!">
          <Button variant="outline" size="sm" disabled={checking} onclick={checkForUpdates}>
            <RefreshCw size={13} class={`mr-1.5 ${checking ? "animate-spin" : ""}`} />
            {checking ? "检查中..." : "检查更新"}
          </Button>
        </SettingCard>
      {/if}
    </div>
  </div>
</div>
