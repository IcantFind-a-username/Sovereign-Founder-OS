import test from 'node:test';
import assert from 'node:assert/strict';

const keys = [
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
const actions = ['CorrectOfferPrice', 'PromoteAcmeToCustomer', 'ShowReportingSearch', 'Reset'];
const metadata = {profile: 'synthetic_playground', real_data_enabled: false, persistence: 'none'};
function fixture() {
  return {
    ...metadata,
    catalog: keys.map(key => ({key, en: `en:${key}`, zh: `zh:${key}`})),
    state: {
      ...metadata, company_name: 'Example Company', offer_name_key: 'reporting_clarity_sprint',
      offer_price_usd_cents: 250000, relationship_organization: 'Example Organization',
      relationship_contact_name: 'Example Person', relationship_contact_email: 'person@example.test',
      relationship_stage: 'lead', discovery_problem_key: 'weekly_reporting_takes_six_hours',
      discovery_budget_min_usd_cents: 300000, discovery_budget_max_usd_cents: 500000,
      discovery_constraint_key: 'finance_must_approve', discovery_next_step_key: 'thirty_minute_scoping_call',
    },
    teaching: {
      ...metadata,
      search: {query_key: 'reporting_search_query', hits: [
        {section_key: 'offer_label', fact_key: 'reporting_clarity_sprint'},
        {section_key: 'discovery_label', fact_key: 'weekly_reporting_takes_six_hours'},
      ]},
      guidance: {next_step_key: 'guidance_correct_price', suggested_action: 'CorrectOfferPrice',
        detail_key: null, completion_key: null},
    },
  };
}
const response = (value = fixture(), status = 200) => ({status, json: async () => value});

// Import with no document, plus traps for capabilities the modules must never use.
const originalFetch = globalThis.fetch;
let importFetches = 0;
globalThis.fetch = () => { importFetches++; throw new Error('Import must not fetch'); };
for (const name of ['localStorage', 'sessionStorage', 'indexedDB', 'navigator', 'location']) {
  Object.defineProperty(globalThis, name, {configurable: true, get() { throw new Error(`Forbidden ${name}`); }});
}
const {createCatalog, translate, formatUsd} = await import('../assets/i18n.js');
const {requestState, bootstrap} = await import('../assets/app.js');
await import('../assets/consultant-ui.js');
globalThis.fetch = originalFetch;

test('catalog_validation_and_locale_lookup_are_closed', async t => {
  const entries = fixture().catalog;
  const catalog = createCatalog(entries);
  assert.equal(catalog.size, 32);
  for (const key of keys) {
    assert.equal(translate(catalog, key, 'en'), `en:${key}`);
    assert.equal(translate(catalog, key, 'zh'), `zh:${key}`);
  }
  for (const locale of ['fr', '', null, 4]) {
    assert.throws(() => translate(catalog, 'page_title', locale));
    assert.throws(() => formatUsd(1250, locale));
  }
  assert.throws(() => translate(catalog, 'missing', 'en'));
  for (const [label, change] of [
    ['short', c => c.pop()], ['long', c => c.push(c[0])],
    ['duplicate within 32', c => { c[31] = c[0]; }],
    ['unknown key within 32', c => { c[31].key = 'unknown'; }],
    ...['key', 'en', 'zh'].flatMap(field => ['', ' ', null, 1].map(value =>
      [`${field}=${String(value)}`, c => { c[0][field] = value; }])),
  ]) {
    await t.test(label, () => {
      const changed = structuredClone(entries);
      change(changed);
      assert.throws(() => createCatalog(changed));
    });
  }
  for (const value of [null, {}, 'catalog']) assert.throws(() => createCatalog(value));
  const cents = 350000;
  assert.equal(formatUsd(cents, 'en'), '$3,500.00');
  assert.equal(formatUsd(cents, 'zh'), 'US$3,500.00');
  assert.equal(cents, 350000);
});

test('requests_use_only_two_literal_endpoints_and_closed_actions', async t => {
  const calls = [];
  t.mock.method(globalThis, 'fetch', async (...args) => { calls.push(args); return response(); });
  for (const action of [null, ...actions]) {
    assert.deepEqual(await requestState(action), fixture());
    const options = {method: action === null ? 'GET' : 'POST', mode: 'same-origin',
      credentials: 'omit', cache: 'no-store', redirect: 'error'};
    if (action !== null) {
      options.headers = {'Content-Type': 'application/json'};
      options.body = JSON.stringify({action});
      assert.ok(new TextEncoder().encode(options.body).length <= 256);
    }
    assert.deepEqual(calls.at(-1), [action === null ? '/api/playground/consultant'
      : '/api/playground/consultant/action', options]);
  }
  for (const action of ['', 'reset', 'Unknown', {}, [], 1, false]) {
    await assert.rejects(requestState(action));
  }
  assert.equal(calls.length, 5);
  let resolve;
  const pending = new Promise(done => { resolve = done; });
  globalThis.fetch = (...args) => { calls.push(args); return pending; };
  const first = requestState('Reset');
  await assert.rejects(requestState('CorrectOfferPrice'));
  assert.equal(calls.length, 6);
  resolve(response());
  await first;
  await new Promise(done => setImmediate(done));
  assert.equal(calls.length, 6, 'no automatic second POST');
});

test('request_failures_never_become_partial_state_or_retries', async t => {
  const errors = ['invalid_configuration', 'invalid_host', 'origin_forbidden', 'invalid_target',
    'not_found', 'method_not_allowed', 'payload_too_large', 'unexpected_body',
    'unsupported_media_type', 'invalid_action_request', 'session_unavailable'];
  const invalid = [];
  for (const layer of [null, 'state', 'teaching']) {
    for (const [field, values] of Object.entries({profile: ['real', null],
      real_data_enabled: [true, 'false'], persistence: ['disk', null]})) {
      for (const value of values) invalid.push([`${layer}.${field}=${value}`, dto => {
        (layer === null ? dto : dto[layer])[field] = value;
      }]);
    }
  }
  for (const field of Object.keys(fixture().state)) {
    invalid.push([`missing state.${field}`, dto => { delete dto.state[field]; }]);
    invalid.push([`wrong state.${field}`, dto => { dto.state[field] = {}; }]);
  }
  for (const field of ['offer_price_usd_cents', 'discovery_budget_min_usd_cents', 'discovery_budget_max_usd_cents']) {
    for (const value of [-1, 0.5, 4294967296, NaN]) {
      invalid.push([`invalid cents ${field}=${value}`, dto => { dto.state[field] = value; }]);
    }
  }
  for (const field of ['offer_name_key', 'relationship_stage', 'discovery_problem_key',
    'discovery_constraint_key', 'discovery_next_step_key']) {
    invalid.push([`missing render key ${field}`, dto => { dto.state[field] = 'missing'; }]);
  }
  invalid.push(
    ['empty state', dto => { dto.state = {}; }],
    ['empty teaching', dto => { dto.teaching = {}; }],
    ['empty catalog', dto => { dto.catalog = []; }],
    ['missing catalog label', dto => { dto.catalog[2].key = 'company_name_label'; }],
    ['empty catalog key', dto => { dto.catalog[2].key = ''; }],
    ['duplicate catalog', dto => { dto.catalog[31] = dto.catalog[0]; }],
    ['missing search', dto => { delete dto.teaching.search; }],
    ['bad search query', dto => { dto.teaching.search.query_key = 4; }],
    ['unknown query key', dto => { dto.teaching.search.query_key = 'missing'; }],
    ['missing hits', dto => { delete dto.teaching.search.hits; }],
    ['one hit', dto => { dto.teaching.search.hits.pop(); }],
    ['invalid hit', dto => { dto.teaching.search.hits[0] = null; }],
    ['unknown hit key', dto => { dto.teaching.search.hits[0].fact_key = 'missing'; }],
    ['missing hit section', dto => { delete dto.teaching.search.hits[0].section_key; }],
    ['missing guidance', dto => { delete dto.teaching.guidance; }],
  );
  for (const field of ['next_step_key', 'suggested_action', 'detail_key', 'completion_key']) {
    invalid.push([`missing guidance.${field}`, dto => { delete dto.teaching.guidance[field]; }]);
    for (const value of ['', 4, {}, 'missing']) {
      invalid.push([`invalid guidance.${field}=${JSON.stringify(value)}`, dto => {
        dto.teaching.guidance[field] = value;
      }]);
    }
  }
  const cases = [
    ...errors.map(error => [error, () => response({...metadata, error}, 400)]),
    ...[201, 204, 302, 500].map(status => [`status ${status}`, () => response(fixture(), status)]),
    ['network rejection', () => Promise.reject(new Error('network sentinel'))],
    ['bad JSON', () => ({status: 200, json: async () => { throw new SyntaxError('bad JSON'); }})],
    ...[null, [], 'value'].map(value => [`bad root ${value}`, () => response(value)]),
    ...invalid.map(([label, mutate]) => [label, () => {
      const dto = fixture(); mutate(dto); return response(dto);
    }]),
  ];
  for (const [label, result] of cases) {
    await t.test(label, async sub => {
      let calls = 0;
      sub.mock.method(globalThis, 'fetch', async () => { calls++; return result(); });
      await assert.rejects(requestState('CorrectOfferPrice'));
      await new Promise(done => setImmediate(done));
      assert.equal(calls, 1, 'failure does not retry or send another action');
    });
  }
  // Failure unlocks the request guard. Both nullable and completed guidance are valid.
  t.mock.method(globalThis, 'fetch', async () => {
    const dto = fixture();
    dto.teaching.guidance = {next_step_key: 'guidance_review_scoping_call', suggested_action: null,
      detail_key: 'thirty_minute_scoping_call', completion_key: 'guidance_example_changes_complete'};
    return response(dto);
  });
  assert.equal((await requestState()).teaching.guidance.suggested_action, null);
});

test('modules_import_without_DOM_storage_network_or_automatic_actions', () => {
  assert.equal(importFetches, 0);
  assert.equal(typeof globalThis.document, 'undefined');
  assert.doesNotThrow(() => bootstrap());
});
