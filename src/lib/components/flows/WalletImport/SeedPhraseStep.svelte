<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Textarea } from '$lib/components/ui/textarea';
  import * as Tabs from '$lib/components/ui/tabs';
  import { i18nStore } from '$lib/i18n';

  type SeedPhraseStepProps = {
    seedPhraseInput?: string;
    entryMode?: EntryMode;
    showEntryModeTabs?: boolean;
    onInputChanged?: typeof defaultOnInputChanged;
    onNormalizedChanged?: typeof defaultOnNormalizedChanged;
    onValidityChanged?: typeof defaultOnValidityChanged;
  };

  type EntryMode = 'paste' | 'manual';

  const REQUIRED_WORDS = 24;
  const MIN_SUGGESTION_CHARS = 3;
  const MAX_SUGGESTIONS = 6;

  const defaultOnInputChanged = (value: string) => {
    void value;
  };
  const defaultOnNormalizedChanged = (value: string) => {
    void value;
  };
  const defaultOnValidityChanged = (valid: boolean) => {
    void valid;
  };

  let {
    seedPhraseInput = '',
    entryMode: controlledEntryMode = undefined,
    showEntryModeTabs = true,
    onInputChanged = defaultOnInputChanged,
    onNormalizedChanged = defaultOnNormalizedChanged,
    onValidityChanged = defaultOnValidityChanged
  }: SeedPhraseStepProps = $props();

  let entryMode = $state<EntryMode>('paste');
  let words = $state<string[]>(Array(REQUIRED_WORDS).fill(''));
  let currentWordIndex = $state(0);
  let currentWordInput = $state('');
  let pasteInput = $state('');
  let normalizedSeed = $state('');
  let validationErrorKey = $state('');
  let isValidating = $state(false);
  let validationRunId = 0;
  let wordList = $state<string[]>([]);
  let wordListErrorKey = $state('');
  let entryErrorKey = $state('');
  let entryErrorWord = $state('');
  let lastEmittedNormalized = $state('');
  let isInputFocused = $state(false);
  let highlightedSuggestionIndex = $state(-1);
  let blurTimer: ReturnType<typeof setTimeout> | null = null;
  let previousEntryMode = $state<EntryMode>('paste');

  const i18n = $derived($i18nStore);
  const wordCount = $derived(words.filter((word) => word.length > 0).length);
  const suggestions = $derived(
    getSuggestions(normalizeWord(currentWordInput), wordList).slice(0, MAX_SUGGESTIONS)
  );
  const showSuggestionDropdown = $derived(
    entryMode === 'manual' && isInputFocused && suggestions.length > 0
  );
  const entryErrorMessage = $derived(
    entryErrorKey === 'walletImport.seed.error.invalidWord'
      ? i18n.t(entryErrorKey, { word: entryErrorWord })
      : entryErrorKey
        ? i18n.t(entryErrorKey)
        : ''
  );
  const validationMessage = $derived(
    isValidating
      ? i18n.t('walletImport.seed.validating')
      : validationErrorKey === 'walletImport.seed.error.wordCount'
        ? i18n.t(validationErrorKey, { required: REQUIRED_WORDS, current: wordCount })
        : validationErrorKey && validationErrorKey !== 'walletImport.seed.error.empty'
          ? i18n.t(validationErrorKey)
          : ''
  );

  $effect(() => {
    if (!controlledEntryMode) return;
    if (entryMode === controlledEntryMode) return;
    entryMode = controlledEntryMode;
  });

  $effect(() => {
    if (entryMode === previousEntryMode) return;
    if (entryMode === 'paste') {
      pasteInput = normalizedSeed;
    } else {
      currentWordInput = words[currentWordIndex] || '';
    }
    previousEntryMode = entryMode;
  });

  function createEmptyWords(): string[] {
    return Array(REQUIRED_WORDS).fill('');
  }

  function normalizeSeedPhrase(value: string): string {
    return value
      .toLowerCase()
      .trim()
      .split(/\s+/)
      .filter(Boolean)
      .join(' ');
  }

  function normalizeWord(value: string): string {
    return value.toLowerCase().replace(/\s+/g, '').trim();
  }

  function splitToWords(value: string): string[] {
    const normalized = normalizeSeedPhrase(value);
    if (!normalized) return createEmptyWords();

    const parts = normalized.split(' ');
    const nextWords = createEmptyWords();
    for (let index = 0; index < Math.min(parts.length, REQUIRED_WORDS); index += 1) {
      nextWords[index] = parts[index];
    }

    return nextWords;
  }

  function buildNormalizedSeed(currentWords: string[]): string {
    return currentWords
      .map((word) => normalizeWord(word))
      .filter(Boolean)
      .join(' ');
  }

  function findNextIndex(currentWords: string[], startAt = 0): number {
    const boundedStart = Math.min(Math.max(startAt, 0), REQUIRED_WORDS - 1);
    const nextAfterCurrent = currentWords.findIndex(
      (word, index) => index >= boundedStart && word.length === 0
    );
    if (nextAfterCurrent !== -1) return nextAfterCurrent;

    const firstEmpty = currentWords.findIndex((word) => word.length === 0);
    if (firstEmpty !== -1) return firstEmpty;

    return boundedStart;
  }

  function getSuggestions(prefix: string, list: string[]): string[] {
    if (!prefix || prefix.length < MIN_SUGGESTION_CHARS || list.length === 0) return [];
    return list.filter((word) => word.startsWith(prefix));
  }

  function resolveWord(value: string): string | null {
    const normalized = normalizeWord(value);
    if (!normalized) return null;
    if (wordList.length === 0) return normalized;
    if (wordList.includes(normalized)) return normalized;
    if (normalized.length >= 4) {
      const key = normalized.slice(0, 4);
      const match = wordList.find((word) => word.slice(0, 4) === key);
      if (match) return match;
    }
    return null;
  }

  function setWords(
    nextWords: string[],
    options: {
      syncPasteInput?: boolean;
      emit?: boolean;
    } = {}
  ) {
    const syncPasteInput = options.syncPasteInput ?? true;
    const emit = options.emit ?? true;
    words = nextWords;
    normalizedSeed = buildNormalizedSeed(nextWords);

    if (syncPasteInput) {
      pasteInput = normalizedSeed;
    }

    if (emit) {
      lastEmittedNormalized = normalizedSeed;
      onInputChanged(normalizedSeed);
      onNormalizedChanged(normalizedSeed);
    }
  }

  function applyPhraseToSlots(value: string, options: { syncPasteInput?: boolean } = {}) {
    const nextWords = splitToWords(value);
    const nextIndex = findNextIndex(nextWords);
    setWords(nextWords, { syncPasteInput: options.syncPasteInput, emit: true });
    currentWordIndex = nextIndex;
    currentWordInput = nextWords[nextIndex] || '';
    entryErrorKey = '';
    entryErrorWord = '';
    highlightedSuggestionIndex = -1;
  }

  function handlePastePhraseInput(value: string) {
    pasteInput = value;
    const nextWords = splitToWords(value);
    const nextIndex = findNextIndex(nextWords);
    setWords(nextWords, { syncPasteInput: false, emit: true });
    currentWordIndex = nextIndex;
    currentWordInput = nextWords[nextIndex] || '';
    entryErrorKey = '';
    entryErrorWord = '';
  }

  function selectWordSlot(index: number) {
    currentWordIndex = index;
    currentWordInput = words[index] || '';
    entryErrorKey = '';
    entryErrorWord = '';
  }

  function clearCurrentWord() {
    const nextWords = [...words];
    nextWords[currentWordIndex] = '';
    setWords(nextWords, { emit: true });
    currentWordInput = '';
    entryErrorKey = '';
    entryErrorWord = '';
    highlightedSuggestionIndex = -1;
  }

  function applyWord(value: string) {
    const rawValue = value.trim().toLowerCase();
    if (!rawValue) return;

    if (rawValue.includes(' ')) {
      applyPhraseToSlots(rawValue);
      return;
    }

    const resolved = resolveWord(rawValue);
    if (!resolved) {
      entryErrorKey = 'walletImport.seed.error.invalidWord';
      entryErrorWord = rawValue;
      return;
    }

    const nextWords = [...words];
    nextWords[currentWordIndex] = resolved;
    const nextIndex = findNextIndex(nextWords, currentWordIndex + 1);
    setWords(nextWords, { emit: true });
    currentWordIndex = nextIndex;
    currentWordInput = nextWords[nextIndex] || '';
    entryErrorKey = '';
    entryErrorWord = '';
    highlightedSuggestionIndex = -1;
  }

  function chooseSuggestion(index: number) {
    const suggestion = suggestions[index];
    if (!suggestion) return;
    applyWord(suggestion);
  }

  function handleManualInputPaste(event: ClipboardEvent) {
    const pastedValue = event.clipboardData?.getData('text') || '';
    const normalized = normalizeSeedPhrase(pastedValue);
    if (!normalized) return;

    if (normalized.split(' ').length > 1) {
      event.preventDefault();
      applyPhraseToSlots(normalized);
    }
  }

  function handleInputKeyDown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' && suggestions.length > 0) {
      event.preventDefault();
      highlightedSuggestionIndex =
        highlightedSuggestionIndex < 0
          ? 0
          : (highlightedSuggestionIndex + 1) % suggestions.length;
      return;
    }

    if (event.key === 'ArrowUp' && suggestions.length > 0) {
      event.preventDefault();
      if (highlightedSuggestionIndex <= 0) {
        highlightedSuggestionIndex = suggestions.length - 1;
      } else {
        highlightedSuggestionIndex -= 1;
      }
      return;
    }

    if (event.key === 'Escape') {
      highlightedSuggestionIndex = -1;
      return;
    }

    if (event.key === 'Enter') {
      event.preventDefault();
      if (highlightedSuggestionIndex >= 0 && highlightedSuggestionIndex < suggestions.length) {
        chooseSuggestion(highlightedSuggestionIndex);
      } else {
        applyWord(currentWordInput);
      }
    }
  }

  function handleInputFocus() {
    if (blurTimer) {
      clearTimeout(blurTimer);
      blurTimer = null;
    }
    isInputFocused = true;
  }

  function handleInputBlur() {
    blurTimer = setTimeout(() => {
      isInputFocused = false;
      highlightedSuggestionIndex = -1;
      blurTimer = null;
    }, 100);
  }

  $effect(() => {
    let active = true;

    (async () => {
      wordListErrorKey = '';
      try {
        const result = await invoke<string[]>('get_mnemonic_wordlist');
        if (!active) return;
        wordList = result.map((word) => word.toLowerCase());
      } catch {
        if (!active) return;
        wordListErrorKey = 'walletImport.seed.error.wordlist';
      }
    })();

    return () => {
      active = false;
    };
  });

  $effect(() => {
    if (!showSuggestionDropdown) {
      highlightedSuggestionIndex = -1;
      return;
    }

    if (highlightedSuggestionIndex < 0 || highlightedSuggestionIndex >= suggestions.length) {
      highlightedSuggestionIndex = 0;
    }
  });

  $effect(() => {
    const externalNormalized = normalizeSeedPhrase(seedPhraseInput);
    if (externalNormalized === lastEmittedNormalized) return;

    const localNormalized = buildNormalizedSeed(words);
    if (externalNormalized === localNormalized) {
      lastEmittedNormalized = externalNormalized;
      pasteInput = externalNormalized;
      return;
    }

    const nextWords = splitToWords(externalNormalized);
    const nextIndex = findNextIndex(nextWords);
    setWords(nextWords, { syncPasteInput: true, emit: false });
    currentWordIndex = nextIndex;
    currentWordInput = nextWords[nextIndex] || '';
    lastEmittedNormalized = externalNormalized;
  });

  $effect(() => {
    const normalized = normalizedSeed;
    const currentRun = ++validationRunId;

    if (!normalized) {
      isValidating = false;
      validationErrorKey = 'walletImport.seed.error.empty';
      onValidityChanged(false);
      return;
    }

    if (wordCount !== REQUIRED_WORDS) {
      isValidating = false;
      validationErrorKey = 'walletImport.seed.error.wordCount';
      onValidityChanged(false);
      return;
    }

    isValidating = true;
    validationErrorKey = '';
    onValidityChanged(false);

    (async () => {
      try {
        const validMnemonic = await invoke<boolean>('validate_mnemonic', {
          seed_phrase: normalized
        });
        if (currentRun !== validationRunId) return;
        validationErrorKey = validMnemonic ? '' : 'walletImport.seed.error.invalid';
        onValidityChanged(validMnemonic);
      } catch {
        if (currentRun !== validationRunId) return;
        validationErrorKey = 'walletImport.seed.error.invalid';
        onValidityChanged(false);
      } finally {
        if (currentRun === validationRunId) {
          isValidating = false;
        }
      }
    })();
  });

  $effect(() => {
    return () => {
      if (blurTimer) {
        clearTimeout(blurTimer);
      }
    };
  });
</script>

<div class="mx-auto w-full max-w-[560px] space-y-5">
  <Tabs.Root bind:value={entryMode}>
    {#if showEntryModeTabs}
      <div class="flex justify-center">
        <Tabs.List>
          <Tabs.Trigger value="paste">
            {i18n.t('walletImport.seed.modePaste')}
          </Tabs.Trigger>
          <Tabs.Trigger value="manual">
            {i18n.t('walletImport.seed.modeManual')}
          </Tabs.Trigger>
        </Tabs.List>
      </div>
    {/if}

    <Tabs.Content value="paste" class={showEntryModeTabs ? 'mt-4 space-y-3' : 'space-y-3'}>
      <Textarea
        id="wallet-import-seed-paste"
        variant="default"
        value={pasteInput}
        oninput={(event) => {
          handlePastePhraseInput((event.target as HTMLTextAreaElement).value);
        }}
        onblur={() => {
          pasteInput = normalizedSeed;
        }}
        placeholder={i18n.t('walletImport.seed.pastePlaceholder')}
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
      ></Textarea>
    </Tabs.Content>

    <Tabs.Content value="manual" class={showEntryModeTabs ? 'mt-4 space-y-4' : 'space-y-4'}>
      <div class="grid grid-cols-8 gap-1.5">
        {#each words as word, index}
          <button
            type="button"
            onclick={() => selectWordSlot(index)}
            class="focus-visible:ring-ring/50 h-10 rounded-md border px-1 text-left outline-none focus-visible:ring-[3px] {currentWordIndex ===
            index
              ? 'border-foreground/30 bg-muted text-foreground'
              : word
                ? 'border-border bg-muted/60 text-foreground'
                : 'border-input bg-background text-muted-foreground'}"
            aria-label={word
              ? i18n.t('walletImport.seed.wordCellFilled', { index: index + 1, word })
              : i18n.t('walletImport.seed.wordCellEmpty', { index: index + 1 })}
          >
            <span class="block text-[10px] leading-none font-semibold">{index + 1}</span>
            <span class="mt-1 block w-full truncate text-[9px] leading-none font-medium">
              {word || '—'}
            </span>
          </button>
        {/each}
      </div>

      <div class="space-y-2.5">
        <div class="flex items-center justify-between text-xs text-muted-foreground">
          <span>
            {i18n.t('walletImport.seed.currentWordProgress', {
              current: currentWordIndex + 1,
              total: REQUIRED_WORDS
            })}
          </span>
          <button
            type="button"
            onclick={clearCurrentWord}
            disabled={!words[currentWordIndex]}
            class="hover:text-foreground disabled:text-muted-foreground/50 text-xs font-medium transition-colors disabled:cursor-not-allowed"
          >
            {i18n.t('walletImport.seed.clearWord')}
          </button>
        </div>

        <div class="grid grid-cols-[1fr_auto] items-start gap-2">
          <div class="relative">
            <Input
              id="wallet-import-current-word"
              value={currentWordInput}
              oninput={(event) => {
                currentWordInput = (event.target as HTMLInputElement).value;
                entryErrorKey = '';
                entryErrorWord = '';
                highlightedSuggestionIndex = -1;
              }}
              onfocus={handleInputFocus}
              onblur={handleInputBlur}
              onkeydown={handleInputKeyDown}
              onpaste={handleManualInputPaste}
              autocomplete="off"
              autocapitalize="off"
              spellcheck="false"
              placeholder={i18n.t('walletImport.seed.currentWordPlaceholder')}
              aria-expanded={showSuggestionDropdown}
              aria-controls="wallet-import-suggestions"
              aria-autocomplete="list"
            />

            {#if showSuggestionDropdown}
              <div
                id="wallet-import-suggestions"
                role="listbox"
                class="bg-popover border-input absolute right-0 bottom-full left-0 z-20 mb-1 max-h-40 overflow-y-auto rounded-md border py-1 shadow-lg"
              >
                {#each suggestions as suggestion, index}
                  <button
                    type="button"
                    role="option"
                    aria-selected={index === highlightedSuggestionIndex}
                    onmousedown={(event) => {
                      event.preventDefault();
                    }}
                    onclick={() => chooseSuggestion(index)}
                    class="hover:bg-accent hover:text-accent-foreground flex h-8 w-full items-center px-3 text-left text-sm {index ===
                    highlightedSuggestionIndex
                      ? 'bg-accent text-accent-foreground'
                      : 'text-popover-foreground'}"
                  >
                    {suggestion}
                  </button>
                {/each}
              </div>
            {/if}
          </div>

          <Button onclick={() => applyWord(currentWordInput)} disabled={!currentWordInput.trim()}>
            {i18n.t('walletImport.seed.addWord')}
          </Button>
        </div>
      </div>
    </Tabs.Content>
  </Tabs.Root>

  <div class="space-y-2">
    <div class="text-muted-foreground flex items-center justify-between text-xs">
      <span>{i18n.t('walletImport.seed.wordCountLabel')}</span>
      <span>{i18n.t('walletImport.seed.wordCount', { current: wordCount, required: REQUIRED_WORDS })}</span>
    </div>

    {#if entryErrorMessage}
      <p class="text-destructive min-h-5 text-xs" aria-live="polite">{entryErrorMessage}</p>
    {:else if validationMessage}
      <p class={"min-h-5 text-xs " + (isValidating ? 'text-muted-foreground' : 'text-destructive')} aria-live="polite">
        {validationMessage}
      </p>
    {:else if entryMode === 'manual' && wordListErrorKey}
      <p class="text-muted-foreground min-h-5 text-xs" aria-live="polite">{i18n.t(wordListErrorKey)}</p>
    {:else}
      <p class="min-h-5 text-xs"></p>
    {/if}
  </div>
</div>
