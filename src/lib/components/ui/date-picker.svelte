<script lang="ts">
	import CalendarIcon from '@lucide/svelte/icons/calendar';
	import {
		type DateValue,
		DateFormatter,
		parseDate,
		getLocalTimeZone
	} from '@internationalized/date';
	import { cn } from '$lib/utils.js';
	import { Calendar } from '$lib/components/ui/calendar/index.js';
	import * as Popover from '$lib/components/ui/popover/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	const df = new DateFormatter('zh-CN', { dateStyle: 'long' });

	let {
		value = $bindable<DateValue | undefined>(undefined),
		placeholder = '选择日期',
		class: className = ''
	}: {
		value?: DateValue;
		placeholder?: string;
		class?: string;
	} = $props();

	/** 文本输入框的内容，与 value 双向同步 */
	let inputText = $state(value ? value.toString() : '');

	$effect(() => {
		// 当日历选择改变时，同步文本框
		inputText = value ? value.toString() : '';
	});

	function handleInput(e: Event) {
		const raw = (e.target as HTMLInputElement).value;
		// 只保留数字和连字符，自动插入分隔符
		const digits = raw.replace(/[^\d-]/g, '');
		inputText = digits;
		(e.target as HTMLInputElement).value = digits;
		// 尝试解析 YYYY-MM-DD
		if (/^\d{4}-\d{2}-\d{2}$/.test(digits)) {
			try {
				value = parseDate(digits);
			} catch {
				// 日期无效，不更新 value
			}
		} else if (digits === '') {
			value = undefined;
		}
	}
</script>

<div class={cn('flex items-center gap-1', className)}>
	<Input
		type="date"
		value={inputText}
		oninput={handleInput}
		placeholder="YYYY-MM-DD"
		inputmode="numeric"
		maxlength={10}
		class="h-9 min-w-0 flex-1 rounded-lg border border-input bg-transparent px-3
			   text-sm transition-colors outline-none placeholder:text-muted-foreground
			   focus:border-ring focus:ring-2 focus:ring-ring/20
			   [&::-webkit-calendar-picker-indicator]:hidden [&::-webkit-inner-spin-button]:appearance-none"
	/>
	<Popover.Root>
		<Popover.Trigger>
			{#snippet child({ props })}
				<Button
					{...props}
					type="button"
					class="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border border-input
						   bg-transparent text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
					aria-label="打开日期选择器"
				>
					<CalendarIcon class="size-4" />
				</Button>
			{/snippet}
		</Popover.Trigger>
		<Popover.Content class="w-auto p-0" align="end">
			<Calendar bind:value type="single" initialFocus captionLayout="dropdown" />
		</Popover.Content>
	</Popover.Root>
</div>
