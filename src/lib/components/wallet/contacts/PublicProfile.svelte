<script lang="ts">
  import type { ContactIdentity } from '$lib/types/addressBook';
  import { contactChainId, identityKey } from '$lib/contacts/identity';
  import { identityProfiles, loadIdentityProfile, profileImage } from '$lib/contacts/profiles';
  import { contactSession } from '$lib/contacts/session';
  import { i18nStore } from '$lib/i18n';
  import IdentityAvatar from '../sections/identity/IdentityAvatar.svelte';
  let {
    identity,
    compact = false,
    showName = true,
  }: { identity: ContactIdentity; compact?: boolean; showName?: boolean } = $props();
  const i18n = $derived($i18nStore);
  const entry = $derived($identityProfiles[identityKey(identity)]);
  $effect(() => {
    if ($contactSession) void loadIdentityProfile(identity, false, true).catch(() => {});
  });
</script>

<div class={compact ? 'space-y-3' : 'space-y-6'}>
  <IdentityAvatar
    seed={identity.identityAddress}
    label={identity.fullyQualifiedName}
    imageUrl={profileImage(entry?.profile)}
    class={compact ? 'size-14 text-base' : 'size-16 text-lg'}
  />
  {#if showName}
    <h3
      class={compact
        ? 'text-lg leading-6 font-semibold break-words'
        : 'text-2xl leading-[30px] font-semibold tracking-tight break-words'}
    >
      {identity.fullyQualifiedName}
    </h3>
  {/if}
  {#if entry?.profile?.description?.value}
    <p
      class={compact
        ? 'text-[13px] leading-[19px] break-words whitespace-pre-wrap text-settings-muted-foreground'
        : 'text-sm leading-[21px] break-words whitespace-pre-wrap'}
    >
      {entry.profile.description.value}
    </p>
  {:else if entry?.loading}
    <div
      class="h-[19px] w-3/4 rounded bg-muted motion-safe:animate-pulse"
      aria-label={i18n.t('common.loading')}
    ></div>
  {/if}
  {#if entry?.unavailable || entry?.profile?.state === 'unavailable' || identity.network !== $contactSession?.network || identity.chainId !== contactChainId(identity.network)}
    <p class="text-xs text-settings-muted-foreground">
      {i18n.t('wallet.contacts.profileUnavailable')}
    </p>
  {/if}
</div>
