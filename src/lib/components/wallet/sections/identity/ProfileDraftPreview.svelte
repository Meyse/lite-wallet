<script lang="ts">
  import type { Snippet } from 'svelte';
  import IdentityAvatar from './IdentityAvatar.svelte';
  let {
    identityAddress,
    displayName,
    avatarUrl,
    headerUrl,
    description,
    headerControls,
    avatarControls,
    body,
    compact = false,
  }: {
    identityAddress: string;
    displayName: string;
    avatarUrl: string | null;
    headerUrl: string | null;
    description?: string | null;
    headerControls?: Snippet;
    avatarControls?: Snippet;
    body?: Snippet;
    compact?: boolean;
  } = $props();
</script>

{#if compact}
  <div class="flex min-h-[94px] items-center gap-4" data-profile-preview>
    <div class="relative h-[94px] w-[225px] shrink-0">
      <div class="absolute inset-x-0 top-0 h-[38px] overflow-hidden rounded-lg bg-background">
        {#if headerUrl}<img src={headerUrl} alt="" class="size-full object-cover" />{/if}
      </div>
      <div class="absolute top-[22px] left-3 rounded-full border-[3px] border-muted bg-muted">
        <IdentityAvatar
          seed={identityAddress}
          label={displayName}
          imageUrl={avatarUrl}
          class="size-12"
        />
      </div>
      <p class="absolute right-0 bottom-2 left-[69px] truncate text-xs font-semibold">
        {displayName}
      </p>
    </div>
    {#if description}<p class="line-clamp-4 min-w-0 text-[13px] leading-[18px] break-words">
        {description}
      </p>{/if}
  </div>
{:else}<div data-profile-preview>
    <div class="relative aspect-[6/1] min-h-[84px] rounded-[14px] bg-muted">
      {#if headerUrl}<img
          src={headerUrl}
          alt=""
          class="absolute inset-0 size-full rounded-[14px] object-cover"
        />{/if}
      {#if headerControls}<div class="absolute right-3 bottom-3">
          {@render headerControls()}
        </div>{/if}
    </div>
    <div class="relative mt-3.5 flex min-h-[54px] items-center gap-2 pl-[120px]">
      <div
        class="absolute bottom-1 left-5 rounded-full border-4 border-background bg-background dark:border-app-canvas dark:bg-app-canvas [&>div]:text-[30px] [&>div]:font-medium"
      >
        <IdentityAvatar
          seed={identityAddress}
          label={displayName}
          imageUrl={avatarUrl}
          class="size-20 text-2xl"
        />
      </div>
      <h2 class="min-w-0 text-lg leading-6 font-semibold break-words">{displayName}</h2>
      {#if avatarControls}{@render avatarControls()}{/if}
    </div>
    {#if body}<div class="mt-6">{@render body()}</div>
    {:else if description}<p class="mt-1 text-sm leading-5 break-words whitespace-pre-wrap">
        {description}
      </p>{/if}
  </div>{/if}
