<script lang="ts">
  import type { ContactIdentity } from '$lib/types/addressBook';
  import { identityKey } from '$lib/contacts/identity';
  import { identityProfiles, loadIdentityProfile, profileImage } from '$lib/contacts/profiles';
  import { contactSession } from '$lib/contacts/session';
  import IdentityAvatar from '../sections/identity/IdentityAvatar.svelte';

  let {
    identity,
    name,
    class: className = 'size-8',
    load = true,
  }: { identity?: ContactIdentity | null; name: string; class?: string; load?: boolean } = $props();
  let visible = $state(false);
  const entry = $derived(identity ? $identityProfiles[identityKey(identity)] : null);
  function observe(node: HTMLElement) {
    if (typeof IntersectionObserver === 'undefined') {
      visible = true;
      return undefined;
    }
    const observer = new IntersectionObserver(([entry]) => {
      visible = entry.isIntersecting;
    });
    observer.observe(node);
    return { destroy: () => observer.disconnect() };
  }
  $effect(() => {
    if (identity && visible && load && $contactSession)
      void loadIdentityProfile(identity).catch(() => {});
  });
</script>

<span use:observe class="inline-flex shrink-0" aria-hidden="true">
  {#if identity}
    <IdentityAvatar
      seed={identity.identityAddress}
      label={name}
      imageUrl={profileImage(entry?.profile)}
      class={className}
    />
  {:else}
    <span
      class={`inline-flex shrink-0 items-center justify-center rounded-full bg-muted font-medium text-settings-muted-foreground ${className}`}
    >
      {Array.from(name.trim())[0]?.toLocaleUpperCase() ?? '@'}
    </span>
  {/if}
</span>
