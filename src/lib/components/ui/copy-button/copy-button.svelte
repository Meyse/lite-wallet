<script lang="ts" module>
  import { type WithElementRef } from '$lib/utils.js';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import { type VariantProps, tv } from 'tailwind-variants';

  export const copyButtonVariants = tv({
    base: 'inline-flex shrink-0 items-center justify-center transition-colors outline-none disabled:pointer-events-none disabled:opacity-50',
    variants: {
      variant: {
        muted: 'text-muted-foreground hover:text-foreground focus-visible:ring-ring/50 focus-visible:ring-2',
        inverse:
          'text-primary-foreground/75 hover:text-primary-foreground focus-visible:ring-primary-foreground/60 focus-visible:ring-2',
        outline:
          'border bg-background text-foreground hover:bg-accent hover:text-accent-foreground dark:border-input dark:bg-input/30 dark:hover:bg-input/50 focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]',
      },
      size: {
        default: 'h-8 w-8 rounded-md',
        sm: 'h-8 w-8 rounded-sm',
        xs: 'h-6 w-6 rounded-sm',
        icon: 'size-9 rounded-md',
      },
    },
    defaultVariants: {
      variant: 'muted',
      size: 'default',
    },
  });

  export type CopyButtonVariant = VariantProps<typeof copyButtonVariants>['variant'];
  export type CopyButtonSize = VariantProps<typeof copyButtonVariants>['size'];
  export type CopyButtonProps = WithElementRef<HTMLButtonAttributes, HTMLButtonElement> & {
    copied?: boolean;
    variant?: CopyButtonVariant;
    size?: CopyButtonSize;
    iconClass?: string;
    copiedIconClass?: string;
  };
</script>

<script lang="ts">
  import CheckIcon from '@lucide/svelte/icons/check';
  import CopyIcon from '@lucide/svelte/icons/copy';
  import { cn } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    copied = false,
    variant = 'muted',
    size = 'default',
    iconClass = 'size-4',
    copiedIconClass = 'size-4 text-emerald-600 dark:text-emerald-400',
    class: className,
    'data-slot': dataSlot = 'copy-button',
    type = 'button',
    ...restProps
  }: CopyButtonProps = $props();
</script>

<button
  bind:this={ref}
  data-slot={dataSlot}
  class={cn(copyButtonVariants({ variant, size }), className)}
  {type}
  {...restProps}
>
  {#if copied}
    <CheckIcon class={copiedIconClass} />
  {:else}
    <CopyIcon class={iconClass} />
  {/if}
</button>
