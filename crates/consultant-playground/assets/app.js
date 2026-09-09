import {createCatalog} from './i18n.js';
import {render, setBusy, showFailure} from './consultant-ui.js';

/** @typedef {import('./i18n.js').Locale} Locale */
/** @typedef {import('./i18n.js').CatalogEntry} CatalogEntry */
/** @typedef {'CorrectOfferPrice' | 'PromoteAcmeToCustomer' | 'ShowReportingSearch' | 'Reset'} Action */
/** @typedef {{profile: 'synthetic_playground', real_data_enabled: false, persistence: 'none'}} Metadata */
/**
 * @typedef {Metadata & {
 * company_name: string, offer_name_key: string, offer_price_usd_cents: number,
 * relationship_organization: string, relationship_contact_name: string,
 * relationship_contact_email: string, relationship_stage: 'lead' | 'customer',
 * discovery_problem_key: string, discovery_budget_min_usd_cents: number,
 * discovery_budget_max_usd_cents: number, discovery_constraint_key: string,
 * discovery_next_step_key: string
 * }} PlaygroundState
 */
/** @typedef {{section_key: string, fact_key: string}} ReportingHit */
/** @typedef {{query_key: string, hits: [ReportingHit, ReportingHit]}} ReportingSearch */
/** @typedef {{next_step_key: string, suggested_action: Action | null, detail_key: string | null, completion_key: string | null}} Guidance */
/** @typedef {Metadata & {search: ReportingSearch, guidance: Guidance}} Teaching */
/** @typedef {Metadata & {state: PlaygroundState, teaching: Teaching, catalog: CatalogEntry[]}} StateResponse */

/** @param {unknown} value @returns {value is Action} */
function isAction(value) {
  return value === 'CorrectOfferPrice' || value === 'PromoteAcmeToCustomer'
    || value === 'ShowReportingSearch' || value === 'Reset';
}

/** @param {unknown} value @returns {Record<string, unknown>} */
function object(value) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error('InvalidState');
  }
  return /** @type {Record<string, unknown>} */ (value);
}

/** @param {Record<string, unknown>} value */
function checkMetadata(value) {
  if (value.profile !== 'synthetic_playground' || value.real_data_enabled !== false
    || value.persistence !== 'none') throw new Error('InvalidMetadata');
}

/** @param {unknown} value @returns {StateResponse} */
function validateState(value) {
  const root = object(value);
  const state = object(root.state);
  const teaching = object(root.teaching);
  for (const layer of [root, state, teaching]) checkMetadata(layer);
  const catalog = createCatalog(root.catalog);
  /** @param {unknown} key */
  const checkKey = key => {
    if (typeof key !== 'string' || !catalog.has(key)) throw new Error('InvalidStateKey');
  };
  for (const field of ['company_name', 'relationship_organization',
    'relationship_contact_name', 'relationship_contact_email']) {
    if (typeof state[field] !== 'string') throw new Error('InvalidStateField');
  }
  for (const field of ['offer_price_usd_cents', 'discovery_budget_min_usd_cents',
    'discovery_budget_max_usd_cents']) {
    const cents = state[field];
    if (typeof cents !== 'number' || !Number.isInteger(cents) || cents < 0 || cents > 4294967295) {
      throw new Error('InvalidStateAmount');
    }
  }
  if (state.relationship_stage !== 'lead' && state.relationship_stage !== 'customer') {
    throw new Error('InvalidStateStage');
  }
  for (const field of ['offer_name_key', 'relationship_stage', 'discovery_problem_key',
    'discovery_constraint_key', 'discovery_next_step_key']) checkKey(state[field]);

  const search = object(teaching.search);
  checkKey(search.query_key);
  if (!Array.isArray(search.hits) || search.hits.length !== 2) throw new Error('InvalidSearch');
  for (const entry of search.hits) {
    const hit = object(entry);
    checkKey(hit.section_key);
    checkKey(hit.fact_key);
  }
  const guidance = object(teaching.guidance);
  checkKey(guidance.next_step_key);
  for (const field of ['detail_key', 'completion_key']) {
    if (guidance[field] !== null) checkKey(guidance[field]);
  }
  if (guidance.suggested_action !== null && !isAction(guidance.suggested_action)) {
    throw new Error('InvalidSuggestedAction');
  }
  // The complete closed DTO has been checked before exposing it to the renderer.
  return /** @type {StateResponse} */ (value);
}

let inFlight = false;
/** @param {Action | null} action @returns {Promise<StateResponse>} */
export async function requestState(action = null) {
  if (action !== null && !isAction(action)) throw new Error('InvalidAction');
  if (inFlight) throw new Error('RequestInFlight');
  inFlight = true;
  try {
    /** @type {RequestInit} */
    const options = {
      method: action === null ? 'GET' : 'POST', mode: 'same-origin',
      credentials: 'omit', cache: 'no-store', redirect: 'error',
    };
    if (action !== null) {
      options.headers = {'Content-Type': 'application/json'};
      options.body = JSON.stringify({action});
    }
    const response = await fetch(action === null
      ? '/api/playground/consultant' : '/api/playground/consultant/action', options);
    if (response.status !== 200) throw new Error('HttpStatus');
    return validateState(await response.json());
  } finally {
    inFlight = false;
  }
}

export function bootstrap() {
  if (typeof document === 'undefined') return;
  /** @type {StateResponse | null} */
  let snapshot = null;
  /** @type {Locale} */
  let locale = 'en';
  let busy = false;
  let searchVisible = false;
  let failed = false;

  /** @param {Action | null} action @param {HTMLButtonElement | null} button */
  async function load(action, button = null) {
    if (busy || failed || (action !== null && snapshot === null)) return;
    busy = true;
    setBusy(true);
    try {
      const next = await requestState(action);
      if (action === 'ShowReportingSearch') searchVisible = true;
      if (action === 'Reset') searchVisible = false;
      snapshot = next;
      render(snapshot, locale, searchVisible);
      busy = false;
      setBusy(false);
      button?.focus();
    } catch {
      snapshot = null;
      searchVisible = false;
      busy = false;
      failed = true;
      showFailure();
    }
  }

  for (const choice of /** @type {Locale[]} */ (['en', 'zh'])) {
    document.getElementById(`language-${choice}`)?.addEventListener('click', () => {
      if (!snapshot || failed) return;
      locale = choice;
      render(snapshot, locale, searchVisible);
    });
  }
  for (const action of /** @type {Action[]} */ ([
    'CorrectOfferPrice', 'PromoteAcmeToCustomer', 'ShowReportingSearch', 'Reset',
  ])) {
    const button = /** @type {HTMLButtonElement} */ (
      document.querySelector(`[data-action="${action}"]`));
    button?.addEventListener('click', () => { void load(action, button); });
  }
  void load(null);
}

if (typeof document !== 'undefined') bootstrap();
