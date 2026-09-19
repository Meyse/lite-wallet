<script lang="ts">
  import type { ContactIdentity } from '$lib/types/addressBook';
  import { provideContactNavigation } from '$lib/contacts/navigation';
  import IdentityMention from '../IdentityMention.svelte';
  let {
    identity,
    onView,
  }: {
    identity: ContactIdentity;
    onView: (identity: ContactIdentity, focus: HTMLElement | null) => void;
  } = $props();
  provideContactNavigation((identity, focus) => onView(identity, focus));
</script>

<IdentityMention {identity} />
