<script lang="ts">
  import ResolvedIdentityMention from '../ResolvedIdentityMention.svelte';
  let { chainId = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV' }: { chainId?: string } = $props();
  let value = $state('alex@');
  export function changeRecipient(next: string) {
    value = next;
  }
</script>

<ResolvedIdentityMention {value} {chainId} delay={0} showStatus />
