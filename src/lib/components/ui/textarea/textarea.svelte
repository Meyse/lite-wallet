<script lang="ts" module>
  import { type VariantProps, tv } from 'tailwind-variants';

  export const textareaVariants = tv({
    base: 'text-foreground selection:bg-primary selection:text-primary-foreground ring-offset-background placeholder:text-foreground/55 dark:placeholder:text-foreground/60 flex w-full min-w-0 rounded-md border border-transparent shadow-none transition-[border-color,box-shadow,background-color] outline-none disabled:cursor-not-allowed disabled:opacity-50',
    variants: {
      variant: {
        default:
          'bg-muted/90 dark:bg-muted/65 min-h-24 px-4 py-3 text-sm focus-visible:ring-[3px] focus-visible:ring-ring/60',
        surface:
          'bg-background min-h-[90px] px-3 py-2 text-sm focus-visible:ring-2 focus-visible:ring-ring',
      },
    },
    defaultVariants: {
      variant: 'default',
    },
  });

  export type TextareaVariant = VariantProps<typeof textareaVariants>['variant'];
</script>

<script lang="ts">
  import type { HTMLTextareaAttributes } from 'svelte/elements';
  import { cn, type WithElementRef } from '$lib/utils.js';

  type Props = WithElementRef<HTMLTextareaAttributes, HTMLTextAreaElement> & {
    variant?: TextareaVariant;
  };

  let {
    ref = $bindable(null),
    value = $bindable(),
    variant = 'default',
    class: className,
    'data-slot': dataSlot = 'textarea',
    ...restProps
  }: Props = $props();
</script>

<textarea
  bind:this={ref}
  data-slot={dataSlot}
  class={cn(textareaVariants({ variant }), className)}
  bind:value
  {...restProps}
></textarea>
