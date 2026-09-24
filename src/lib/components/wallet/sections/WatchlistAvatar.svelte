<script lang="ts">
  import { onMount } from 'svelte';
  import { contactChainId, identityKey } from '$lib/contacts/identity';
  import { identityProfiles, loadIdentityProfile, profileImage } from '$lib/contacts/profiles';
  import { contactSession } from '$lib/contacts/session';
  import type { WalletNetwork } from '$lib/types/wallet';
  import type { WatchlistTargetKind } from '$lib/types/watchlist';
  import { cn } from '$lib/utils';
  import IdentityAvatar from './identity/IdentityAvatar.svelte';

  let {
    address,
    displayName,
    targetKind,
    network,
    class: className = '',
  }: {
    address: string;
    displayName: string;
    targetKind: WatchlistTargetKind;
    network: WalletNetwork;
    class?: string;
  } = $props();

  let element: HTMLSpanElement;
  let visible = $state(false);
  const identity = $derived({
    identityAddress: address,
    fullyQualifiedName: displayName,
    network,
    chainId: contactChainId(network),
  });
  const profile = $derived(
    $contactSession?.network === network
      ? ($identityProfiles[identityKey(identity)]?.profile ?? null)
      : null
  );
  const avatarUrl = $derived(profileImage(profile));

  $effect(() => {
    if (targetKind !== 'identity' || !visible || $contactSession?.network !== network) return;
    void loadIdentityProfile(identity).catch(() => {
      // The initials remain visible when the public profile cannot be loaded.
    });
  });

  onMount(() => {
    if (typeof IntersectionObserver === 'undefined') {
      visible = true;
      return () => {};
    }
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) {
        visible = true;
        observer.disconnect();
      }
    });
    observer.observe(element);
    return () => observer.disconnect();
  });
</script>

<span bind:this={element} class="inline-flex shrink-0" data-testid="watchlist-avatar">
  {#if targetKind === 'identity'}
    <IdentityAvatar
      seed={address}
      label={displayName}
      imageUrl={avatarUrl}
      class={cn('size-12', className)}
    />
  {:else}
    <span
      class={cn(
        'inline-flex size-12 shrink-0 items-center justify-center rounded-full bg-muted text-sm font-medium text-muted-foreground',
        className
      )}
      aria-hidden="true"
      data-testid="watchlist-address-avatar"
    >
      {displayName.trim().slice(0, 1).toUpperCase() || 'R'}
    </span>
  {/if}
</span>
