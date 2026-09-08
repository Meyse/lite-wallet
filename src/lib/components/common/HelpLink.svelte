<!--
  Reusable help trigger that stays aligned with the standard inline text action
  primitive instead of introducing its own link treatment.
-->

<script lang="ts">
  import InlineTextActionButton from '$lib/components/common/InlineTextActionButton.svelte';

  let { text, onclick } = $props();
</script>

<InlineTextActionButton {onclick}>
  {text}
</InlineTextActionButton>
