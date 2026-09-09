/** @typedef {'en' | 'zh'} Locale */
/** @typedef {{key: string, en: string, zh: string}} CatalogEntry */

// Identifiers from the canonical Rust catalog, never a second translation table.
const REQUIRED_KEYS = [
  'page_title', 'boundary_notice', 'company_label', 'offer_label',
  'relationship_label', 'discovery_label', 'price_label', 'organization_label',
  'contact_name_label', 'contact_email_label', 'stage_label', 'problem_label',
  'budget_label', 'constraint_label', 'next_step_label', 'reporting_clarity_sprint',
  'weekly_reporting_takes_six_hours', 'finance_must_approve', 'thirty_minute_scoping_call',
  'lead', 'customer', 'action_correct_price', 'action_promote_customer',
  'action_show_reporting_search', 'action_reset', 'language_en', 'language_zh',
  'reporting_search_query', 'guidance_correct_price', 'guidance_promote_customer',
  'guidance_review_scoping_call', 'guidance_example_changes_complete',
];

/** @param {unknown} entries @returns {Map<string, CatalogEntry>} */
export function createCatalog(entries) {
  if (!Array.isArray(entries) || entries.length !== 32) {
    throw new Error('InvalidCatalog');
  }
  /** @type {Map<string, CatalogEntry>} */
  const catalog = new Map();
  for (const entry of entries) {
    if (!entry || typeof entry.key !== 'string' || !entry.key.trim()
      || typeof entry.en !== 'string' || !entry.en.trim()
      || typeof entry.zh !== 'string' || !entry.zh.trim()
      || catalog.has(entry.key)) {
      throw new Error('InvalidCatalog');
    }
    catalog.set(entry.key, entry);
  }
  if (REQUIRED_KEYS.some(key => !catalog.has(key))) throw new Error('MissingCatalogKey');
  return catalog;
}

/** @param {Map<string, CatalogEntry>} catalog @param {string} key @param {Locale} locale */
export function translate(catalog, key, locale) {
  if (locale !== 'en' && locale !== 'zh') throw new Error('InvalidLocale');
  const entry = catalog.get(key);
  if (!entry) throw new Error('MissingCatalogKey');
  return entry[locale];
}

/** @param {number} cents @param {Locale} locale */
export function formatUsd(cents, locale) {
  if (!Number.isInteger(cents) || cents < 0 || cents > 4294967295
    || (locale !== 'en' && locale !== 'zh')) {
    throw new Error('InvalidAmount');
  }
  return new Intl.NumberFormat(locale === 'en' ? 'en-US' : 'zh-CN', {
    style: 'currency', currency: 'USD',
  }).format(cents / 100);
}
