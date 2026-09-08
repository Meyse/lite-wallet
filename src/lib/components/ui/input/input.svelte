<script lang="ts" module>
	import { type VariantProps, tv } from "tailwind-variants";

	export const inputVariants = tv({
		base: "bg-muted/90 dark:bg-muted/65 text-foreground selection:bg-primary selection:text-primary-foreground ring-offset-background placeholder:text-foreground/55 dark:placeholder:text-foreground/60 flex w-full min-w-0 border border-transparent shadow-none transition-[border-color,box-shadow,background-color] outline-none disabled:cursor-not-allowed disabled:opacity-50",
		variants: {
			variant: {
				default: "h-11 rounded-md px-4 py-2 text-base md:text-sm",
				lg: "h-12 rounded-xl px-5 py-3 text-lg font-semibold md:text-base",
			},
		},
		defaultVariants: {
			variant: "default",
		},
	});

	export type InputVariant = VariantProps<typeof inputVariants>["variant"];
</script>

<script lang="ts">
	import type { HTMLInputAttributes, HTMLInputTypeAttribute } from "svelte/elements";
	import EyeIcon from "@lucide/svelte/icons/eye";
	import EyeOffIcon from "@lucide/svelte/icons/eye-off";
	import { cn, type WithElementRef } from "$lib/utils.js";

	type InputType = Exclude<HTMLInputTypeAttribute, "file">;

	type Props = WithElementRef<
		Omit<HTMLInputAttributes, "type"> &
			({ type: "file"; files?: FileList } | { type?: InputType; files?: undefined })
	> & {
		variant?: InputVariant;
	};

	let {
		ref = $bindable(null),
		value = $bindable(),
		type,
		files = $bindable(),
		variant = "default",
		class: className,
		"data-slot": dataSlot = "input",
		...restProps
	}: Props = $props();

	let showPassword = $state(false);
</script>

{#if type === "file"}
	<input
		bind:this={ref}
		data-slot={dataSlot}
		class={cn(
			"selection:bg-primary selection:text-primary-foreground ring-offset-background placeholder:text-muted-foreground flex h-11 w-full min-w-0 rounded-md border border-transparent bg-transparent px-3 pt-1.5 text-sm font-medium shadow-none transition-[border-color,box-shadow] outline-none disabled:cursor-not-allowed disabled:opacity-50",
			"focus-visible:border-transparent focus-visible:ring-ring/60 focus-visible:ring-[3px]",
			"aria-invalid:ring-destructive/30 dark:aria-invalid:ring-destructive/45 aria-invalid:border-transparent",
			className
		)}
		type="file"
		bind:files
		bind:value
		{...restProps}
	/>
{:else}
	<div class="relative w-full">
		<input
			bind:this={ref}
			data-slot={dataSlot}
			class={cn(
				inputVariants({ variant }),
				"focus-visible:border-transparent focus-visible:ring-ring/60 focus-visible:ring-[3px]",
				"aria-invalid:ring-destructive/30 dark:aria-invalid:ring-destructive/45 aria-invalid:border-transparent",
				type === "password" && "pe-11",
				type === "password" && !showPassword && "tracking-[0.2em] placeholder:tracking-normal",
				className
			)}
			type={type === "password" && showPassword ? "text" : type}
			bind:value
			{...restProps}
		/>
		{#if type === "password"}
			<button
				type="button"
				aria-label={showPassword ? "Hide password" : "Show password"}
				aria-pressed={showPassword}
				class="text-foreground/55 hover:text-foreground/80 focus-visible:ring-ring/50 absolute inset-y-0 end-0 inline-flex items-center justify-center px-3 outline-none focus-visible:ring-[2px]"
				onclick={() => {
					showPassword = !showPassword;
				}}
			>
				{#if showPassword}
					<EyeOffIcon class="size-4" />
				{:else}
					<EyeIcon class="size-4" />
				{/if}
			</button>
		{/if}
	</div>
{/if}
