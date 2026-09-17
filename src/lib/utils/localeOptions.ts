import { SUPPORTED_LOCALES } from '$lib/i18n';
import type { Locale } from '$lib/i18n';

type TranslateFn = (key: string) => string;

export interface LocaleOption {
  value: Locale;
  label: string;
}

const localeMetadata: Record<Locale, { labelKey: string }> = {
  en: { labelKey: 'languageGate.option.en' },
  nl: { labelKey: 'languageGate.option.nl' },
  de: { labelKey: 'languageGate.option.de' },
  es: { labelKey: 'languageGate.option.es' },
};

export function buildLocaleOptions(t: TranslateFn): LocaleOption[] {
  return SUPPORTED_LOCALES.map((locale) => ({
    value: locale,
    label: t(localeMetadata[locale].labelKey),
  }));
}
