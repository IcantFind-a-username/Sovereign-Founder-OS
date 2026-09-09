/**
 * @typedef {{id:string,name:string,goal:string,origin:string}} Company
 * @typedef {{id:string,company_id:string,name:string,scope:string,currency:string,unit_price_cents:number,origin:string}} Service
 * @typedef {{id:string,input_revision:number,company:Company,service:Service}} Case
 * @typedef {{currency:string,next_step:string}} Summary
 * @typedef {{epoch:string,revision:number,demo_day:number,maturity:string,available_commands:string[],case:Case,summary:Summary}} View
 */
/** @type {View|null} */
let currentView = null;
let busy = false;
const fields = ['name', 'goal', 'service_name', 'service_scope', 'currency', 'unit_price'];

export function formatMoney(cents) {
  const whole = Math.floor(cents / 100).toLocaleString('en-US');
  return `SGD ${whole}.${String(cents % 100).padStart(2, '0')}`;
}

export function requestFor(view, type, data) {
  return { epoch: view.epoch, expected_revision: view.revision, command: { type, data } };
}

function route() {
  return typeof location !== 'undefined' && location.hash === '#company' ? 'company' : 'today';
}

function element(tag, text = '') {
  const node = document.createElement(tag);
  node.textContent = text;
  return node;
}

function button(text, action) {
  const node = document.createElement('button');
  node.type = 'button';
  node.textContent = text;
  node.addEventListener('click', action);
  return node;
}

function statusNode() {
  const node = element('p');
  node.id = 'status';
  node.setAttribute('role', 'status');
  node.setAttribute('aria-live', 'polite');
  return node;
}

function setBusy(value) {
  busy = value;
  if (typeof document === 'undefined') return;
  document.querySelector('#app')?.setAttribute('aria-busy', String(value));
  document.querySelectorAll('button, input, textarea').forEach(node => {
    const control = /** @type {HTMLButtonElement|HTMLInputElement|HTMLTextAreaElement} */ (node);
    control.disabled = value || (control.dataset.route !== undefined && !currentView);
  });
}

function notice(message) {
  if (typeof document === 'undefined') return;
  const node = document.querySelector('#status');
  if (node) node.textContent = message;
}

function failure() {
  currentView = null;
  if (typeof document === 'undefined') return;
  const app = document.querySelector('#app');
  const heading = element('h2', 'Exercise unavailable');
  const text = statusNode();
  text.textContent = 'The result is unknown. Reload exercise state before continuing.';
  const reload = button('Reload exercise state', () => { void refresh(); });
  reload.id = 'reload';
  app.replaceChildren(heading, text, reload);
  setBusy(busy);
}

function validView(view) {
  return view && /^[0-9a-f]{32}$/.test(view.epoch) && Number.isInteger(view.revision)
    && view.revision >= 0 && view.revision <= 10000 && view.demo_day === 0
    && view.maturity === 'synthetic_experiment'
    && Array.isArray(view.available_commands) && view.available_commands.length === 2
    && view.available_commands[0] === 'save_company' && view.available_commands[1] === 'reset'
    && typeof view.case?.company?.name === 'string' && typeof view.case.company.goal === 'string'
    && typeof view.case.service?.name === 'string' && typeof view.case.service.scope === 'string'
    && view.case.service.currency === 'SGD' && Number.isSafeInteger(view.case.service.unit_price_cents)
    && view.case.service.unit_price_cents > 0 && view.case.service.unit_price_cents <= 100000000
    && view.summary?.currency === 'SGD' && typeof view.summary.next_step === 'string';
}

function validError(body) {
  return body?.ok === false && typeof body.error?.code === 'string'
    && (body.error.field === null || fields.includes(body.error.field));
}

function showFieldError(field) {
  if (typeof document === 'undefined') return;
  document.querySelectorAll('[aria-invalid]').forEach(node => node.removeAttribute('aria-invalid'));
  if (fields.includes(field)) {
    const input = /** @type {HTMLElement|null} */ (document.querySelector(`[name="${field}"]`));
    input?.setAttribute('aria-invalid', 'true');
    input?.setAttribute('aria-describedby', 'status');
    notice(`Check ${field.replaceAll('_', ' ')}. Your unsaved entries are still in this form.`);
  } else {
    notice('Check the form values. Your unsaved entries are still in this form.');
  }
}

export async function refresh() {
  if (busy) return;
  setBusy(true);
  try {
    const response = await fetch('/api/demo', { credentials: 'omit', cache: 'no-store', redirect: 'error' });
    const body = await response.json();
    if (!response.ok || body.ok !== true || !validView(body.view)) throw new Error('invalid state');
    currentView = body.view;
    render(currentView, route());
  } catch {
    failure();
  } finally {
    setBusy(false);
  }
}

export async function dispatch(type, data) {
  if (busy || !currentView) return;
  setBusy(true);
  notice('Saving exercise state…');
  try {
    const response = await fetch('/api/demo/command', {
      method: 'POST', credentials: 'omit', cache: 'no-store', redirect: 'error',
      headers: { 'Content-Type': 'application/json', 'X-Founder-Demo': '1' },
      body: JSON.stringify(requestFor(currentView, type, data))
    });
    const body = await response.json();
    if (response.status === 409 && validError(body)) {
      setBusy(false);
      await refresh();
      if (currentView) notice('The exercise changed or reached a limit. State reloaded; review it before saving again.');
      return;
    }
    if (response.status === 422 && validError(body)) {
      showFieldError(body.error.field);
      return;
    }
    if (!response.ok || body.ok !== true || !validView(body.view)) throw new Error('invalid response');
    currentView = body.view;
    render(currentView, route());
    notice(type === 'reset' ? 'Practice seed restored. Previous edits were cleared.' : 'Saved in this exercise only.');
  } catch {
    failure();
  } finally {
    setBusy(false);
  }
}

function addField(form, labelText, name, value, multiline = false) {
  const label = element('label', labelText);
  label.htmlFor = `field-${name}`;
  const input = multiline ? document.createElement('textarea') : document.createElement('input');
  input.id = `field-${name}`;
  input.name = name;
  input.value = value;
  input.autocomplete = 'off';
  input.spellcheck = false;
  if (name === 'unit_price') input.inputMode = 'decimal';
  if (name === 'currency') input.readOnly = true;
  label.append(input);
  form.append(label);
}

export function render(view, selectedRoute) {
  if (typeof document === 'undefined') return;
  const app = document.querySelector('#app');
  const page = selectedRoute === 'company' || selectedRoute === '#company' ? 'company' : 'today';
  app.replaceChildren();
  document.querySelectorAll('[data-route]').forEach(node => {
    if (/** @type {HTMLElement} */ (node).dataset.route === page) node.setAttribute('aria-current', 'page');
    else node.removeAttribute('aria-current');
  });
  const company = view.case.company;
  const service = view.case.service;
  const heading = element('h2', page === 'company' ? 'Company and service' : 'Today · exercise day 0');
  app.append(heading);
  if (page === 'today') {
    app.append(element('h3', company.name), element('p', company.goal),
      element('h3', service.name), element('p', service.scope), element('p', formatMoney(service.unit_price_cents)),
      element('p', view.summary.next_step));
    const edit = button('Edit company and service', () => { location.hash = 'company'; });
    app.append(edit, statusNode());
  } else {
    const origin = company.origin === 'seeded_synthetic' ? 'Seeded synthetic records' : 'Unverified experiment input';
    app.append(element('p', `${origin}. Use invented data only.`));
    const form = document.createElement('form');
    form.id = 'company-form';
    form.noValidate = true;
    form.autocomplete = 'off';
    addField(form, 'Company name', 'name', company.name);
    addField(form, 'Company goal', 'goal', company.goal, true);
    addField(form, 'Service name', 'service_name', service.name);
    addField(form, 'Service scope', 'service_scope', service.scope, true);
    addField(form, 'Currency', 'currency', service.currency);
    const cents = service.unit_price_cents;
    addField(form, 'Unit price', 'unit_price', `${Math.floor(cents / 100)}.${String(cents % 100).padStart(2, '0')}`);
    const save = document.createElement('button');
    save.type = 'submit';
    save.textContent = 'Save company and service';
    form.append(save, statusNode());
    form.addEventListener('submit', event => {
      event.preventDefault();
      if (!busy) void dispatch('save_company', Object.fromEntries(new FormData(form)));
    });
    app.append(form);
  }
  const confirmation = element('div');
  confirmation.id = 'reset-confirmation';
  confirmation.hidden = true;
  const reset = button('Reset exercise', () => {
    confirmation.hidden = false;
    confirm.focus();
  });
  reset.id = 'reset';
  const confirm = button('Confirm reset', () => { void dispatch('reset', {}); });
  confirm.id = 'confirm-reset';
  const cancel = button('Cancel', () => { confirmation.hidden = true; reset.focus(); });
  cancel.id = 'cancel-reset';
  confirmation.append(element('p', 'Discard these practice edits and restore the seed?'), confirm, cancel);
  app.append(reset, confirmation);
  setBusy(busy);
}

if (typeof document !== 'undefined') {
  document.querySelectorAll('[data-route]').forEach(node => {
    node.addEventListener('click', () => {
      if (!busy && currentView) location.hash = /** @type {HTMLElement} */ (node).dataset.route;
    });
  });
  window.addEventListener('hashchange', () => {
    if (!busy && currentView) render(currentView, route());
  });
  void refresh();
}
