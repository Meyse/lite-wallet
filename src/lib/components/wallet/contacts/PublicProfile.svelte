<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { ContactIdentity } from '$lib/types/addressBook';
  import { contactChainId, identityKey } from '$lib/contacts/identity';
  import { identityProfiles, loadIdentityProfile, profileImage } from '$lib/contacts/profiles';
  import { contactSession } from '$lib/contacts/session';
  import { i18nStore } from '$lib/i18n';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import IdentityAvatar from '../sections/identity/IdentityAvatar.svelte';
  let {
    identity,
    compact = false,
    showName = true,
    nameAction,
  }: {
    identity: ContactIdentity;
    compact?: boolean;
    showName?: boolean;
    nameAction?: Snippet;
  } = $props();
  const i18n = $derived($i18nStore);
  const entry = $derived($identityProfiles[identityKey(identity)]);
  const supported = $derived(
    identity.network === $contactSession?.network &&
      identity.chainId === contactChainId(identity.network)
  );
  // Include queued requests, but keep confirmed content visible during refreshes.
  const loading = $derived(
    supported && !entry?.profile && !entry?.unavailable && (!entry || entry.loading)
  );
  $effect(() => {
    if ($contactSession) void loadIdentityProfile(identity, false, true).catch(() => {});
  });
</script>

<div class={compact ? 'space-y-3' : 'space-y-6'} aria-busy={loading}>
  {#if loading}
    <Skeleton
      class="shrink-0 rounded-full bg-muted motion-reduce:animate-none {compact
        ? 'size-14'
        : 'size-16'}"
      aria-hidden="true"
    />
  {:else}
    <IdentityAvatar
      seed={identity.identityAddress}
      label={identity.fullyQualifiedName}
      imageUrl={profileImage(entry?.profile)}
      class={compact ? 'size-14 text-base' : 'size-16 text-lg'}
    />
  {/if}
  {#if showName}
    <div class="flex min-w-0 items-start gap-2">
      <h3
        class={compact
          ? 'min-w-0 text-lg leading-6 font-semibold break-words'
          : 'min-w-0 text-2xl leading-[30px] font-semibold tracking-tight break-words'}
      >
        {identity.fullyQualifiedName}
      </h3>
      {#if nameAction}<div class="shrink-0">{@render nameAction()}</div>{/if}
    </div>
  {/if}
  {#if entry?.profile?.description?.value}
    <p
      class={compact
        ? 'text-[13px] leading-[19px] break-words whitespace-pre-wrap text-settings-muted-foreground'
        : 'text-sm leading-[21px] break-words whitespace-pre-wrap'}
    >
      {entry.profile.description.value}
    </p>
  {:else if loading}
    <div class="space-y-2" role="status">
      <span class="sr-only">{i18n.t('common.loading')}</span>
      <Skeleton class="h-3 w-full rounded bg-muted motion-reduce:animate-none" aria-hidden="true" />
      <Skeleton class="h-3 w-3/4 rounded bg-muted motion-reduce:animate-none" aria-hidden="true" />
    </div>
  {/if}
  {#if entry?.unavailable || entry?.profile?.state === 'unavailable' || !supported}
    <p class="text-xs text-settings-muted-foreground">
      {i18n.t('wallet.contacts.profileUnavailable')}
    </p>
  {/if}
</div>
