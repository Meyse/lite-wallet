<script lang="ts">
  import CheckIcon from '@lucide/svelte/icons/check';
  import CopyIcon from '@lucide/svelte/icons/copy';
  import { Button, type ButtonSize, type ButtonVariant } from '$lib/components/ui/button';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  export type CopyActionState = 'idle' | 'copied' | 'failed';

  type Props = Omit<HTMLButtonAttributes, 'children'> & {
    state?: CopyActionState;
    label: string;
    copiedLabel: string;
    failedLabel?: string;
    variant?: ButtonVariant;
    size?: ButtonSize;
    iconClass?: string;
    copiedIconClass?: string;
  };

  let {
    state = 'idle',
    label,
    copiedLabel,
    failedLabel,
    variant = 'secondary',
    size = 'sm',
    iconClass = 'size-3.5',
    copiedIconClass = 'size-3.5',
    class: className,
    type = 'button',
    ...restProps
  }: Props = $props();

  const resolvedLabel = $derived(
    state === 'copied' ? copiedLabel : state === 'failed' ? (failedLabel ?? label) : label
  );
  const resolvedAriaPressed = $derived(state === 'copied');
</script>

<Button
  {variant}
  {size}
  {type}
  class={className}
  aria-pressed={resolvedAriaPressed}
  {...restProps}
>
  {#if state === 'copied'}
    <CheckIcon class={copiedIconClass} />
  {:else}
    <CopyIcon class={iconClass} />
  {/if}
  {resolvedLabel}
</Button>
