<script lang="ts">
  import { pickIdentityAvatarGradient } from '$lib/styles/identityAvatarGradients';

  type IdentityAvatarProps = {
    seed: string;
    label: string;
    class?: string;
  };

  function deriveInitials(value: string): string {
    const normalized = value.replace(/@/g, '').trim();
    if (!normalized) return '@';

    const segments = normalized.split('.').filter(Boolean);
    if (segments.length >= 2) {
      return `${segments[0].slice(0, 1)}${segments[1].slice(0, 1)}`.toUpperCase();
    }

    return normalized.slice(0, 2).toUpperCase();
  }

  let { seed, label, class: className = '' }: IdentityAvatarProps = $props();

  const gradient = $derived(pickIdentityAvatarGradient(seed));
  const initials = $derived(deriveInitials(label));
</script>

<div
  class={`inline-flex size-9 shrink-0 items-center justify-center rounded-full text-[11px] font-semibold tracking-wide text-white ${className}`}
  style={`background-image: linear-gradient(135deg, ${gradient[0]}, ${gradient[1]});`}
  aria-hidden="true"
>
  {initials}
</div>
