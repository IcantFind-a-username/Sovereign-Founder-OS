import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { formatMoney, requestFor, render, refresh, dispatch } from './app.js';

const view = { epoch: '0'.repeat(32), revision: 2, demo_day: 0, maturity: 'synthetic_experiment', available_commands: ['save_company', 'reset'],
  case: { company: { name: 'A', goal: 'G' }, service: { name: 'S', scope: 'X', currency: 'SGD', unit_price_cents: 250000 } },
  summary: { currency: 'SGD', next_step: 'Next' } };
test('request envelope binds epoch and revision', () => {
  const before = structuredClone(view);
  assert.deepEqual(requestFor(view, 'reset', {}), { epoch: view.epoch,
    expected_revision: 2, command: { type: 'reset', data: {} } });
  assert.deepEqual(view, before);
});
test('money formatting does not use floating point totals', () => {
  for (const [cents, expected] of [[0, 'SGD 0.00'], [1, 'SGD 0.01'],
    [350000, 'SGD 3,500.00'], [100000000, 'SGD 1,000,000.00']]) {
    assert.equal(formatMoney(cents), expected);
  }
});
test('module import performs no browser or network work', () => {
  const source = `globalThis.fetch = () => { throw Error('import fetched'); };
    globalThis.setTimeout = () => { throw Error('import scheduled a timer'); };
    globalThis.localStorage = new Proxy({}, { get() { throw Error('import used storage'); } });
    await import(${JSON.stringify(new URL('./app.js', import.meta.url).href)});`;
  const child = spawnSync(process.execPath,
    ['--experimental-default-type=module', '--input-type=module', '-e', source], { encoding: 'utf8' });
  assert.equal(child.status, 0, child.stderr);
  assert.equal(child.stdout, '');
});
test('render has the frozen void interface', () => {
  assert.equal(render(view, 'company'), undefined);
});

function success(value = view) {
  return { ok: true, status: 200, json: async () => ({ ok: true, view: structuredClone(value) }) };
}
test('one in-flight command and exact fetch policy', async () => {
  const original = globalThis.fetch;
  const calls = [];
  let release;
  globalThis.fetch = async (url, options) => {
    calls.push([url, options]);
    if (url === '/api/demo') return success();
    return new Promise(resolve => { release = resolve; });
  };
  try {
    await refresh();
    const pending = dispatch('save_company', { name: 'Example' });
    await dispatch('reset', {});
    assert.equal(calls.length, 2);
    assert.deepEqual(calls[0], ['/api/demo', { credentials: 'omit', cache: 'no-store', redirect: 'error' }]);
    assert.equal(calls[1][0], '/api/demo/command');
    const options = calls[1][1];
    assert.equal(options.redirect, 'error');
    assert.equal(options.credentials, 'omit');
    assert.equal(options.cache, 'no-store');
    assert.deepEqual(options.headers, { 'Content-Type': 'application/json', 'X-Founder-Demo': '1' });
    assert.equal(JSON.parse(options.body).expected_revision, 2);
    release(success({ ...view, revision: 3 }));
    await pending;
  } finally { globalThis.fetch = original; }
});
test('409 refreshes once without retrying the POST and failures clear authority', async () => {
  const original = globalThis.fetch;
  const calls = [];
  globalThis.fetch = async url => {
    calls.push(url);
    if (url === '/api/demo') return success();
    return { ok: false, status: 409, json: async () => ({ ok: false, error: { code: 'stale_revision', field: null } }) };
  };
  try {
    await refresh(); await dispatch('reset', {});
    assert.deepEqual(calls, ['/api/demo', '/api/demo/command', '/api/demo']);
    globalThis.fetch = async () => { throw Error('offline'); };
    await refresh();
    let attempts = 0;
    globalThis.fetch = async () => { attempts++; return success(); };
    await dispatch('reset', {});
    assert.equal(attempts, 0);
  } finally { globalThis.fetch = original; }
});
