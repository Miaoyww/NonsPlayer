<script lang="ts">
	// 引入了 MessageCircle 作为联系我们的图标
	import { ExternalLink, FileText, ShieldCheck, MessageCircle } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { Label } from '$lib/components/ui/label';
	import SettingCard from '$lib/components/cards/settings-card.svelte';
	import * as Dialog from '$lib/components/ui/dialog'; // 引入标准 Dialog
	import { NONSPLAYER_NAME } from '$lib/const';
	import { isTauri } from '@tauri-apps/api/core';
	import { fly } from 'svelte/transition';
	const version = __APP_VERSION__;

	const API_URL = 'https://api.github.com/repos/Miaoyww/Veto/releases/latest';
	import favicon from '$lib/assets/favicon.png';
	async function checkForUpdates() {}
</script>

<div class="grid h-full grid-rows-2" in:fly={{ y: 8, duration: 320, opacity: 0 }}>
	<div>
		<div class="mb-1 flex justify-center">
			<img src={favicon} alt="App Logo" class="h-64 w-64 rounded-md" />
		</div>
		<div class="mb-6 flex flex-col items-center gap-2 text-center">
			<h2 class="text-3xl font-extrabold tracking-wide text-stone-800 dark:text-stone-100">
				{VETO_NAME}
			</h2>
			<p class="text-sm text-muted-foreground">否决权</p>
			<p class="text-sm text-muted-foreground">模拟联合国会议系统</p>
		</div>
		<div class="space-y-3">
			<SettingCard title="版本号" description="当前应用版本。">
				<Label>v{version}</Label>
			</SettingCard>

			<!-- 新增：联系我们模块 -->
			<SettingCard title="联系我们" description="加入我们的社区交流或反馈问题。">
				<Dialog.Root>
					<!-- 如果你使用的是 Svelte 5 的最新 shadcn 规范，这里的 Button 可能需要用 {#snippet child({ props })} 包裹，具体取决于你的 shadcn-svelte 版本 -->
					<Dialog.Trigger>
						<Button variant="outline" size="sm">
							<MessageCircle size={13} class="mr-1.5" />
							联系我们
						</Button>
					</Dialog.Trigger>
					<Dialog.Content class="sm:max-w-[425px]">
						<Dialog.Header>
							<Dialog.Title>联系我们</Dialog.Title>
							<Dialog.Description>请选择你要加入的平台进行交流或反馈问题。</Dialog.Description>
						</Dialog.Header>
						<div class="grid gap-3 py-4">
							<!-- QQ 群按钮，记得替换 href 为真实的加群链接 -->
							<Button
								variant="outline"
								href="https://qm.qq.com/q/pTvxDLoiVq"
								target="_blank"
								class="w-full justify-start"
							>
								<MessageCircle size={16} class="mr-2" />
								加入 QQ 群
							</Button>

							<!-- Discord 按钮，记得替换 href 为真实的邀请链接 -->
							<Button
								variant="outline"
								href="https://discord.gg/"
								target="_blank"
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
					href="https://github.com/Miaoyww/Veto/blob/master/LICENSE"
					target="_blank"
				>
					<FileText size={13} class="mr-1.5" />
					GPL-3.0
				</Button>
			</SettingCard>

			<SettingCard title="GitHub 仓库" description="查看源代码或提交 Issue。">
				<Button variant="outline" size="sm" href="https://github.com/Miaoyww/Veto" target="_blank">
					<ExternalLink size={13} class="mr-1.5" />
					Miaoyww/Veto
				</Button>
			</SettingCard>

			{#if isTauri()}
				<SettingCard title="检查更新" description="跟上新版本!">
					<Button variant="outline" size="sm" onclick={checkForUpdates}>检查更新</Button>
				</SettingCard>
			{/if}
		</div>
	</div>
</div>
