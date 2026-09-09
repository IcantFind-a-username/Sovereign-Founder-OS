import {createCatalog, formatUsd, translate} from './i18n.js';

/** @typedef {import('./app.js').StateResponse} StateResponse */
/** @typedef {import('./app.js').Action} Action */
/** @typedef {import('./i18n.js').Locale} Locale */

const labels = {
  'company-heading': 'company_label', 'company-name-label': 'company_label',
  'offer-heading': 'offer_label', 'offer-name-label': 'offer_label', 'price-label': 'price_label',
  'relationship-heading': 'relationship_label', 'organization-label': 'organization_label',
  'contact-name-label': 'contact_name_label', 'contact-email-label': 'contact_email_label',
  'stage-label': 'stage_label', 'discovery-heading': 'discovery_label',
  'problem-label': 'problem_label', 'budget-label': 'budget_label',
  'constraint-label': 'constraint_label', 'discovery-next-step-label': 'next_step_label',
  'next-step-heading': 'next_step_label',
};
/** @type {Record<Action, string>} */
const actionKeys = {
  CorrectOfferPrice: 'action_correct_price', PromoteAcmeToCustomer: 'action_promote_customer',
  ShowReportingSearch: 'action_show_reporting_search', Reset: 'action_reset',
};
let busy = false;

/** @param {string} id @returns {HTMLElement} */
function node(id) {
  const element = document.getElementById(id);
  if (!element) throw new Error('MissingElement');
  return element;
}
/** @param {string} id @param {string} value */
function put(id, value) { node(id).textContent = value; }

/** @param {boolean} value */
export function setBusy(value) {
  busy = value;
  node('main').setAttribute('aria-busy', String(value));
  for (const button of document.querySelectorAll('button[data-action]')) {
    /** @type {HTMLButtonElement} */ (button).disabled = value || node('business').hidden;
  }
}

export function showFailure() {
  node('business').hidden = true;
  node('search-results').hidden = true;
  node('loading').hidden = true;
  for (const button of document.querySelectorAll('[aria-controls="search-results"]')) {
    button.setAttribute('aria-expanded', 'false');
  }
  for (const id of ['language-en', 'language-zh']) {
    /** @type {HTMLButtonElement} */ (node(id)).disabled = true;
  }
  setBusy(false);
  node('failure').hidden = false;
  node('failure').focus();
}

/** @param {StateResponse} snapshot @param {Locale} locale @param {boolean} searchVisible */
export function render(snapshot, locale, searchVisible) {
  const catalog = createCatalog(snapshot.catalog);
  const state = snapshot.state;
  const {search, guidance} = snapshot.teaching;
  /** @param {string} key */
  const tr = key => translate(catalog, key, locale);
  document.documentElement.lang = locale === 'en' ? 'en' : 'zh-CN';
  document.title = tr('page_title');
  put('page-title', tr('page_title'));
  put('boundary-notice', tr('boundary_notice'));
  for (const [id, key] of Object.entries(labels)) put(id, tr(key));
  for (const choice of ['en', 'zh']) {
    const button = /** @type {HTMLButtonElement} */ (node(`language-${choice}`));
    button.textContent = tr(`language_${choice}`);
    button.setAttribute('aria-pressed', String(locale === choice));
    button.hidden = false;
    button.disabled = false;
  }
  put('company-name', state.company_name);
  put('offer-name', tr(state.offer_name_key));
  put('price', formatUsd(state.offer_price_usd_cents, locale));
  put('organization', state.relationship_organization);
  put('contact-name', state.relationship_contact_name);
  put('contact-email', state.relationship_contact_email);
  put('stage', tr(state.relationship_stage));
  put('problem', tr(state.discovery_problem_key));
  put('budget', `${formatUsd(state.discovery_budget_min_usd_cents, locale)} – ${formatUsd(state.discovery_budget_max_usd_cents, locale)}`);
  put('constraint', tr(state.discovery_constraint_key));
  put('discovery-next-step', tr(state.discovery_next_step_key));

  const guidanceItems = [];
  for (const key of [guidance.next_step_key, guidance.detail_key, guidance.completion_key]) {
    if (key !== null) {
      const paragraph = document.createElement('p');
      paragraph.textContent = tr(key);
      guidanceItems.push(paragraph);
    }
  }
  node('guidance-items').replaceChildren(...guidanceItems);
  for (const [action, key] of Object.entries(actionKeys)) {
    const button = /** @type {HTMLButtonElement} */ (
      document.querySelector(`[data-action="${action}"]`));
    button.textContent = tr(key);
    button.hidden = false;
    const suggested = guidance.suggested_action === action;
    button.classList.toggle('suggested', suggested);
    if (suggested) button.setAttribute('aria-describedby', 'guidance-items');
    else button.removeAttribute('aria-describedby');
    if (action === 'ShowReportingSearch') button.setAttribute('aria-expanded', String(searchVisible));
  }

  node('search-results').hidden = !searchVisible;
  node('hits').replaceChildren();
  if (searchVisible) {
    put('search-heading', tr(search.query_key));
    for (const hit of search.hits) {
      const item = document.createElement('li');
      item.textContent = `${tr(hit.section_key)}: ${tr(hit.fact_key)}`;
      node('hits').append(item);
    }
  }
  node('loading').hidden = true;
  node('failure').hidden = true;
  node('business').hidden = false;
  setBusy(busy);
}
