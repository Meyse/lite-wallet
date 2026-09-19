<script lang="ts">
  import ResolvedIdentityMention from '../ResolvedIdentityMention.svelte';
  let value = $state('alex@');
  export function changeRecipient(next: string) {
    value = next;
  }
</script>

<ResolvedIdentityMention {value} chainId="i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV" delay={0} />
