<script lang="ts">
  import { resolveCoinPresentationById } from '$lib/coins/presentation.js';
  import type { CoinGeneratedIcon, CoinIcon as ResolvedIcon } from '$lib/coins/presentation.js';
  import type { Protocol } from '$lib/types/wallet.js';

  interface CoinIconProps {
    coinId: string;
    coinName?: string;
    proto?: Protocol;
    size?: number;
    showBadge?: boolean;
    decorative?: boolean;
    privateMuted?: boolean;
    class?: string;
  }

  const props: CoinIconProps = $props();
  const coinId = $derived(props.coinId);
  const coinName = $derived(props.coinName);
  const proto = $derived(props.proto);
  const size = $derived(props.size ?? 28);
  const showBadge = $derived(props.showBadge ?? false);
  const decorative = $derived(props.decorative ?? false);
  const privateMuted = $derived(props.privateMuted ?? false);
  const className = $derived(props.class ?? '');
  const iconSurfaceClass = $derived(
    privateMuted ? 'coin-icon-surface coin-icon-surface--private-muted' : 'coin-icon-surface'
  );

  const resolved = $derived(resolveCoinPresentationById(coinId, proto));
  const presentation = $derived(
    resolved ?? {
      id: coinId,
      displayName: coinName ?? coinId,
      displayTicker: coinId,
      proto: proto ?? 'vrsc',
      icon: { kind: 'generated', seed: coinId, logoMapped: false } as const,
      badgeCoinId: null,
    }
  );

  const mainIcon = $derived(presentation.icon as ResolvedIcon);
  const iconAlt = $derived(`${coinName ?? presentation.displayName} icon`);

  const badgePresentation = $derived(
    showBadge && presentation.badgeCoinId ? resolveCoinPresentationById(presentation.badgeCoinId) : null
  );
  const badgeIcon = $derived((badgePresentation?.icon ?? null) as ResolvedIcon | null);
  const badgeSize = $derived(Math.max(10, Math.round(size * 0.45)));

  function hashCode(seed: string): number {
    let hash = 0;
    for (let i = 0; i < seed.length; i += 1) {
      hash = seed.charCodeAt(i) + ((hash << 5) - hash);
      hash |= 0;
    }
    return hash;
  }

  function intToColor(value: number): string {
    const hex = (value & 0x00ffffff).toString(16).toUpperCase();
    return `#${'000000'.slice(0, 6 - hex.length)}${hex}`;
  }

  function buildPalette(seed: string): string[] {
    const seedHash = hashCode(seed);
    const colors: string[] = [];

    for (let i = 0; i < 16; i += 1) {
      const wave = Math.sin(i + seedHash) * (10000 + seedHash);
      colors.push(intToColor(Math.floor(wave)));
    }

    return colors;
  }

  function generatedColors(icon: CoinGeneratedIcon): string[] {
    return buildPalette(icon.seed ?? coinId);
  }
</script>

<div
  class={`relative inline-flex shrink-0 items-center justify-center ${className}`}
  style={`width: ${size}px; height: ${size}px;`}
>
  {#if mainIcon.kind === 'asset'}
    <img
      class={`${iconSurfaceClass} h-full w-full object-contain`}
      src={mainIcon.dark}
      alt={decorative ? '' : iconAlt}
      aria-hidden={decorative}
      loading="lazy"
      draggable="false"
    />
  {:else if mainIcon.kind === 'fiat-symbol'}
    <div
      class={`${iconSurfaceClass} text-foreground border-border/30 bg-muted/25 flex h-full w-full items-center justify-center rounded-full border text-[11px] font-semibold`}
      aria-hidden={decorative}
      title={decorative ? undefined : iconAlt}
    >
      {mainIcon.symbol}
    </div>
  {:else}
    <div
      class={`${iconSurfaceClass} border-border/30 grid h-full w-full grid-cols-4 overflow-hidden rounded-full border`}
      aria-hidden={decorative}
      title={decorative ? undefined : iconAlt}
    >
      {#each generatedColors(mainIcon) as color}
        <span style={`background-color: ${color};`} class="block h-full w-full"></span>
      {/each}
    </div>
  {/if}

  {#if badgeIcon && showBadge}
    <div
      class="border-background bg-background absolute -right-0.5 -bottom-0.5 overflow-hidden rounded-full border"
      style={`width: ${badgeSize}px; height: ${badgeSize}px;`}
    >
      {#if badgeIcon.kind === 'asset'}
        <img
          class="h-full w-full object-contain"
          src={badgeIcon.dark}
          alt=""
          aria-hidden="true"
          loading="lazy"
          draggable="false"
        />
      {:else if badgeIcon.kind === 'fiat-symbol'}
        <div class="text-foreground flex h-full w-full items-center justify-center text-[8px] font-semibold">
          {badgeIcon.symbol}
        </div>
      {:else}
        <div class="grid h-full w-full grid-cols-4 overflow-hidden rounded-full">
          {#each generatedColors(badgeIcon) as color}
            <span style={`background-color: ${color};`} class="block h-full w-full"></span>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>
