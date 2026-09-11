"use strict";
// Company, Customers, and Documents views. This file only defines functions;
// app.js calls initCrm() once the shell (helpers, state) exists. Checked by
// tsc --checkJs together with the other assets; JSDoc types only.

/**
 * @typedef {{id: string, customer_id: string, offer_id: string|null, name: string, status: string, budget_cents: number|null, created_at: number, done_at: number|null, acceptance_criteria: string[]}} Project
 * @typedef {{id: string, project_id: string, title: string, due_at: number|null, done_at: number|null, origin: string}} Task
 * @typedef {{id: string, customer_id: string, due_at: number, note: string, done_at: number|null, origin: string}} FollowUp
 * @typedef {{invoice_id: string, customer_name: string, title: string, amount_cents: number, paid_cents: number, outstanding_cents: number, status: string, due_at: number|null}} Receivable
 */

/** @type {string|null} */
let selectedCustomerId = null;
/** @type {string|null} */
let editingDocumentId = null;

const currencyCode = () => (ws && ws.venture && ws.venture.currency) || "SGD";
const money = (cents) => currencyCode() + " " + (cents / 100).toLocaleString(locale(), { minimumFractionDigits: 2, maximumFractionDigits: 2 });
const fmtDate = (unix) => unix ? new Date(unix * 1000).toLocaleDateString(locale()) : "—";
const dateValue = (unix) => unix ? new Date(unix * 1000).toISOString().slice(0, 10) : "";
/** "1,200,000.50" → cents, or null when blank; invalid text stays a string for the server to reject. */
function centsFromText(text) {
  const clean = String(text || "").replace(/,/g, "").trim();
  if (!clean) return null;
  const parts = clean.split(".");
  if (parts.length > 2 || !/^\d+$/.test(parts[0]) || (parts[1] !== undefined && !/^\d{0,2}$/.test(parts[1]))) return NaN;
  return Number(parts[0]) * 100 + Number((parts[1] || "").padEnd(2, "0"));
}
function unixFromDate(text) {
  if (!text) return null;
  const stamp = Date.parse(text + "T00:00:00Z");
  return Number.isNaN(stamp) ? null : Math.floor(stamp / 1000);
}

/** Apply a mutation result: refresh the shared workspace and dependent views. */
async function applyResult(result, okMessage) {
  if (!result.ok) { toast("bad", result.error); return false; }
  ws = result.workspace;
  renderAll();
  loadState();
  loadCommandCenter();
  if (okMessage) toast("good", okMessage);
  return true;
}

/* ─────────────── Company ─────────────── */

function renderCompany() {
  if (!ws) return;
  const v = ws.venture;
  const focused = document.activeElement;
  const set = (id, value) => { const node = $(id); if (node && focused !== node) node.value = value; };
  if (v) {
    set("v-name", v.name); set("v-service", v.service);
    set("p-jurisdiction", v.jurisdiction || "");
    set("p-currency", v.currency || "SGD");
    set("p-uen", v.uen || "");
    if (focused !== $("p-gst")) $("p-gst").checked = !!v.gst_registered;
    set("p-incorporated", dateValue(v.incorporated_at));
    set("p-fye", String(v.fiscal_year_end_month || 12));
    set("p-revenue", v.revenue_estimate_cents != null ? (v.revenue_estimate_cents / 100).toFixed(2) : "");
  }
}

async function saveCompany() {
  const revenue = centsFromText($("p-revenue").value);
  if (Number.isNaN(revenue)) { toast("bad", t("ws_amount") + ": " + $("p-revenue").value); return; }
  const result = await api("/api/workspace/profile", {
    name: $("v-name").value, service: $("v-service").value,
    jurisdiction: $("p-jurisdiction").value, currency: $("p-currency").value, uen: $("p-uen").value,
    gst_registered: $("p-gst").checked,
    incorporated_at: unixFromDate($("p-incorporated").value),
    fiscal_year_end_month: Number($("p-fye").value),
    revenue_estimate_cents: revenue,
  });
  if (await applyResult(result, t("saved"))) $("v-status").textContent = t("saved");
}

/* ─────────────── Customers ─────────────── */

function stageBadge(stage) {
  return badge(stage === "customer" ? "good" : "neutral", stage === "customer" ? t("cu_stage_customer") : t("cu_stage_lead"));
}

function renderCustomers() {
  if (!ws) return;
  const box = $("customers");
  if (!ws.customers.length) {
    box.replaceChildren(el("div", "empty", t("ws_no_customers")));
  } else {
    box.replaceChildren(table(
      [t("th_customer"), t("th_stage"), t("th_email"), t("th_location"), t("th_consent"), t("th_updated"), ""],
      ws.customers.map(c => {
        const open = el("button", "ghost small", t("cu_open"));
        open.addEventListener("click", () => { selectedCustomerId = c.id; renderCustomerDetail(); $("customer-detail").scrollIntoView({ behavior: "smooth", block: "start" }); });
        return [c.name, stageBadge(c.stage), c.email || "—", c.jurisdiction || "—", c.personal_data_consent ? t("yes") : t("no"), fmtDate(c.updated_at || c.created_at), open];
      })));
  }
  renderCustomerDetail();
  const select = $("d-customer");
  const selected = select.value;
  select.replaceChildren(...ws.customers.map(c => {
    const option = /** @type {HTMLOptionElement} */ (el("option", null, c.name));
    option.value = c.id;
    return option;
  }));
  if (selected && ws.customers.some(c => c.id === selected)) select.value = selected;
}

async function addCustomer() {
  const result = await api("/api/workspace/customer", { name: $("c-name").value, email: $("c-email").value, notes: $("c-notes").value });
  if (await applyResult(result)) { $("c-name").value = ""; $("c-email").value = ""; $("c-notes").value = ""; }
}

function renderCustomerDetail() {
  const panel = $("customer-detail");
  const customer = ws && ws.customers.find(c => c.id === selectedCustomerId);
  if (!customer) { panel.hidden = true; return; }
  panel.hidden = false;
  $("cd-title").textContent = t("cu_detail_title")(customer);
  const focused = document.activeElement;
  const set = (id, value) => { const node = $(id); if (focused !== node) node.value = value; };
  set("cd-name", customer.name); set("cd-email", customer.email); set("cd-notes", customer.notes);
  set("cd-discovery", customer.discovery_notes || ""); set("cd-stage", customer.stage || "lead");
  set("cd-jurisdiction", customer.jurisdiction || "");
  if (focused !== $("cd-consent")) $("cd-consent").checked = !!customer.personal_data_consent;
  renderProjects(customer);
  renderFollowUps(customer);
  renderCustomerDocuments(customer);
  renderAskTeam(customer);
  loadTimeline(customer.id);
}

async function saveCustomer() {
  const result = await api("/api/workspace/customer/update", {
    customer_id: selectedCustomerId, name: $("cd-name").value, email: $("cd-email").value, notes: $("cd-notes").value,
    discovery_notes: $("cd-discovery").value, stage: $("cd-stage").value, jurisdiction: $("cd-jurisdiction").value,
    personal_data_consent: $("cd-consent").checked,
  });
  await applyResult(result, t("saved"));
}

function renderProjects(customer) {
  const box = $("cd-projects");
  const projects = ws.projects.filter(p => p.customer_id === customer.id);
  const offerSelect = $("cd-project-offer");
  offerSelect.replaceChildren(el("option", null, "—"), ...ws.documents.filter(d => d.customer_id === customer.id && d.kind === "offer").map(d => {
    const option = /** @type {HTMLOptionElement} */ (el("option", null, d.title));
    option.value = d.id;
    return option;
  }));
  if (!projects.length) { box.replaceChildren(el("div", "empty", t("cu_no_projects"))); return; }
  box.replaceChildren(...projects.map(p => {
    const wrap = el("div", "doc");
    const head = el("div", "doc-head");
    head.appendChild(el("span", "title", p.name));
    head.appendChild(badge(p.status === "done" ? "good" : p.status === "active" ? "warn" : "neutral", t("cu_project_status")(p.status)));
    if (p.budget_cents != null) head.appendChild(el("span", "mono", money(p.budget_cents)));
    if (p.status === "proposed") {
      const start = el("button", "ghost small", t("cu_start"));
      start.addEventListener("click", () => withBusy(start, async () => applyResult(await api("/api/workspace/project/status", { project_id: p.id, status: "active" }))));
      head.appendChild(start);
    }
    const tasks = ws.tasks.filter(task => task.project_id === p.id)
      .sort((a, b) => (a.due_at || Infinity) - (b.due_at || Infinity));
    const openTasks = tasks.filter(task => !task.done_at).length;
    if (p.status !== "done" && !openTasks) {
      const finish = el("button", "ghost small", t("cu_finish"));
      finish.addEventListener("click", () => withBusy(finish, async () => applyResult(await api("/api/workspace/project/status", { project_id: p.id, status: "done" }))));
      head.appendChild(finish);
    } else if (p.status !== "done") {
      // The server refuses to close a project with open tasks; say so here
      // instead of offering a button that can only fail.
      head.appendChild(el("span", "status-line", t("tm_finish_first")));
    }
    wrap.appendChild(head);
    if (p.acceptance_criteria && p.acceptance_criteria.length) {
      const list = el("ul", "criteria");
      p.acceptance_criteria.forEach(c => list.appendChild(el("li", null, c)));
      wrap.appendChild(list);
    }
    const taskList = el("div", "tasks");
    tasks.forEach(task => {
      const row = el("div", "task-row");
      row.appendChild(badge(task.done_at ? "good" : "neutral", task.done_at ? t("cu_task_done") : (task.due_at ? fmtDate(task.due_at) : "—")));
      row.appendChild(el("span", task.done_at ? "done" : null, task.title));
      row.appendChild(el("span", "status-line", t("cu_task_origin")(task.origin)));
      if (!task.done_at && p.status !== "done") {
        const done = el("button", "ghost small", t("cu_task_done"));
        done.addEventListener("click", () => withBusy(done, async () => applyResult(await api("/api/workspace/task/done", { task_id: task.id }))));
        row.appendChild(done);
      }
      taskList.appendChild(row);
    });
    if (p.status !== "done") {
      const add = el("div", "toolbar");
      const title = /** @type {HTMLInputElement} */ (el("input"));
      title.placeholder = t("cu_new_task"); title.maxLength = 200;
      const due = /** @type {HTMLInputElement} */ (el("input"));
      due.type = "date";
      const button = el("button", "ghost small", t("cu_add_task"));
      button.addEventListener("click", () => withBusy(button, async () => {
        if (await applyResult(await api("/api/workspace/task", { project_id: p.id, title: title.value, due_at: due.value || null }))) title.value = "";
      }));
      add.appendChild(title); add.appendChild(due); add.appendChild(button);
      taskList.appendChild(add);
    }
    wrap.appendChild(taskList);
    return wrap;
  }));
}

async function addProject() {
  const budget = centsFromText($("cd-project-budget").value);
  if (Number.isNaN(budget)) { toast("bad", t("ws_amount")); return; }
  const result = await api("/api/workspace/project", {
    customer_id: selectedCustomerId, name: $("cd-project-name").value,
    offer_id: $("cd-project-offer").value || null,
    budget: budget == null ? "" : (budget / 100).toFixed(2),
  });
  if (await applyResult(result)) $("cd-project-name").value = "";
}

function renderFollowUps(customer) {
  const box = $("cd-followups");
  const items = ws.follow_ups.filter(f => f.customer_id === customer.id);
  if (!items.length) { box.replaceChildren(el("div", "empty", t("cu_no_followups"))); return; }
  const now = Date.now() / 1000;
  box.replaceChildren(...items.map(f => {
    const row = el("div", "task-row");
    const overdue = !f.done_at && f.due_at < now;
    row.appendChild(badge(f.done_at ? "good" : overdue ? "bad" : "neutral", f.done_at ? t("cu_followup_done") : overdue ? t("cu_overdue") : fmtDate(f.due_at)));
    row.appendChild(el("span", f.done_at ? "done" : null, f.note));
    row.appendChild(el("span", "status-line", t("cu_task_origin")(f.origin)));
    if (!f.done_at) {
      const done = el("button", "ghost small", t("cu_followup_done"));
      done.addEventListener("click", () => withBusy(done, async () => applyResult(await api("/api/workspace/follow-up/done", { follow_up_id: f.id }))));
      row.appendChild(done);
    }
    return row;
  }));
}

async function addFollowUp() {
  const result = await api("/api/workspace/follow-up", { customer_id: selectedCustomerId, due_at: $("cd-followup-due").value, note: $("cd-followup-note").value });
  if (await applyResult(result)) $("cd-followup-note").value = "";
}

function statusKind(status) {
  return { draft: "neutral", pending_approval: "warn", approved_pending_delivery: "good", rejected: "bad", revoked: "neutral", delivered: "good" }[status] || "neutral";
}

function renderCustomerDocuments(customer) {
  const box = $("cd-documents");
  const docs = ws.documents.filter(d => d.customer_id === customer.id);
  if (!docs.length) { box.replaceChildren(el("div", "empty", t("cu_no_documents"))); return; }
  box.replaceChildren(...docs.map(d => {
    const row = el("div", "task-row");
    row.appendChild(el("span", "badge neutral", t(d.kind === "offer" ? "kind_offer" : "kind_invoice")));
    row.appendChild(el("span", null, d.title));
    row.appendChild(badge(statusKind(d.status), t("status_" + d.status)));
    if (d.amount_cents != null) row.appendChild(el("span", "mono", money(d.amount_cents)));
    if (d.accepted_at) row.appendChild(badge("good", t("cu_accepted")));
    if (d.kind === "offer" && !d.accepted_at && (d.status === "approved_pending_delivery" || d.status === "delivered")) {
      const accept = el("button", "ghost small", t("cu_accept_offer"));
      accept.addEventListener("click", () => withBusy(accept, async () => {
        if (!await confirmAction("accept", d.title)) return;
        await applyResult(await api("/api/workspace/offer/accepted", { document_id: d.id }), t("toast_accepted")(d.title));
      }));
      row.appendChild(accept);
    }
    return row;
  }));
}

async function loadTimeline(customerId) {
  const box = $("cd-timeline");
  const data = await api("/api/workspace/timeline", { customer_id: customerId });
  if (selectedCustomerId !== customerId) return;
  if (!data.ok || !data.timeline.length) { box.replaceChildren(el("div", "empty", t("cu_no_timeline"))); return; }
  box.replaceChildren(table([t("th_time"), t("th_action"), t("th_resource")],
    data.timeline.slice().reverse().map(e => [fmtTime(e.at), t("event_label")(e.action), e.subject || e.resource])));
}

function renderAskTeam(customer) {
  const box = $("cd-team");
  const hires = (ws.employees || []).filter(e => e.status === "hired" && ["analyst", "proposal_writer", "delivery_planner"].includes(e.role));
  if (!hires.length) { box.replaceChildren(el("span", "status-line", t("tm_no_employees"))); return; }
  box.replaceChildren(...hires.map(e => {
    const button = el("button", "ghost small", employeeLabel(e));
    button.addEventListener("click", () => withBusy(button, () => runEmployee(e.id, { customer_id: customer.id })));
    return button;
  }));
}

/* ─────────────── Documents ─────────────── */

function renderDocuments() {
  if (!ws) return;
  const documents = $("documents");
  if (!ws.documents.length) {
    documents.replaceChildren(el("div", "empty", t("ws_no_documents")));
  } else {
    documents.replaceChildren(...[...ws.documents].reverse().map(d => renderDocumentCard(d)));
  }
  renderApprovals();
  loadReceivables();
}

function renderDocumentCard(d) {
  const wrap = el("div", "doc");
  const head = el("div", "doc-head");
  head.appendChild(el("span", "badge neutral", t(d.kind === "offer" ? "kind_offer" : "kind_invoice")));
  head.appendChild(el("span", "title", d.title));
  head.appendChild(badge(statusKind(d.status), t("status_" + d.status)));
  head.appendChild(el("span", "status-line", t("doc_revision")(d.revision || 1)));
  if (d.amount_cents != null) head.appendChild(el("span", "mono", money(d.amount_cents)));
  if (d.due_at) head.appendChild(el("span", "status-line", t("doc_due")(fmtDate(d.due_at))));
  const editable = d.status === "draft" || d.status === "rejected" || d.status === "revoked";
  if (editable) {
    const edit = el("button", "ghost small", t("doc_edit"));
    edit.addEventListener("click", () => { editingDocumentId = editingDocumentId === d.id ? null : d.id; renderDocuments(); });
    head.appendChild(edit);
  }
  if (d.status === "draft") {
    const submit = el("button", "ghost small", t("ws_submit_send"));
    submit.addEventListener("click", () => withBusy(submit, async () => applyResult(await api("/api/workspace/request-send", { document_id: d.id }))));
    head.appendChild(submit);
  }
  if (d.status === "approved_pending_delivery") {
    const delivered = el("button", "ghost small", t("ws_mark_delivered"));
    delivered.addEventListener("click", () => withBusy(delivered, async () => {
      if (!await confirmAction("deliver", d.title)) return;
      await applyResult(await api("/api/workspace/confirm-delivery", { document_id: d.id }), t("toast_delivered")(d.title));
    }));
    head.appendChild(delivered);
    const revoke = el("button", "ghost small", t("ws_revoke"));
    revoke.addEventListener("click", () => withBusy(revoke, async () => {
      if (!await confirmAction("revoke", d.title)) return;
      await applyResult(await api("/api/workspace/revoke", { document_id: d.id }), t("toast_revoked")(d.title));
    }));
    head.appendChild(revoke);
  }
  // The next step after sending an offer is hearing back; it belongs where
  // the founder just was, not only on the customer page.
  if (d.kind === "offer" && !d.accepted_at && (d.status === "approved_pending_delivery" || d.status === "delivered")) {
    const accept = el("button", "ghost small", t("cu_accept_offer"));
    accept.addEventListener("click", () => withBusy(accept, async () => {
      if (!await confirmAction("accept", d.title)) return;
      await applyResult(await api("/api/workspace/offer/accepted", { document_id: d.id }), t("toast_accepted")(d.title));
    }));
    head.appendChild(accept);
  }
  if (d.accepted_at) head.appendChild(badge("good", t("cu_accepted")));
  wrap.appendChild(head);
  if (d.status === "rejected" || d.status === "revoked") wrap.appendChild(el("div", "status-line", t("doc_reopen_hint")));
  const approvalWithEvidence = ws.approvals.find(a => a.document_id === d.id && a.evidence);
  if (approvalWithEvidence) {
    wrap.appendChild(el("div", "mono", t("ws_evidence")(approvalWithEvidence.evidence)));
    const outbox = approvalWithEvidence.evidence.outbox;
    if (outbox) {
      wrap.appendChild(el("div", "mono", t("ws_outbox")(outbox)));
      // Composing is not sending: the founder sends it. Hand them the file
      // rather than a path into a hidden folder.
      if (d.status === "approved_pending_delivery" || d.status === "delivered") {
        const bar = el("div", "toolbar");
        // Same shape as the export link on the Company page.
        const link = /** @type {HTMLAnchorElement} */ (el("a"));
        link.href = "/api/outbox/" + encodeURIComponent(outbox.relative_path);
        link.setAttribute("download", outbox.relative_path);
        link.appendChild(el("button", "ghost small", t("msg_download")));
        bar.appendChild(link);
        if (d.status === "approved_pending_delivery") bar.appendChild(el("span", "status-line", t("msg_download_hint")));
        wrap.appendChild(bar);
      }
    }
  }
  if (editingDocumentId === d.id && editable) {
    const form = el("div", "form-grid");
    const title = /** @type {HTMLInputElement} */ (el("input")); title.value = d.title; title.maxLength = 200;
    const body = /** @type {HTMLTextAreaElement} */ (el("textarea")); body.value = d.body; body.style.minHeight = "220px";
    const amount = /** @type {HTMLInputElement} */ (el("input")); amount.value = d.amount_cents != null ? (d.amount_cents / 100).toFixed(2) : ""; amount.placeholder = "2500.00";
    const bar = el("div", "toolbar");
    const save = el("button", "primary small", t("doc_save"));
    save.addEventListener("click", () => withBusy(save, async () => {
      if (await applyResult(await api("/api/workspace/document/update", { document_id: d.id, title: title.value, body: body.value, amount: amount.value }), t("saved"))) editingDocumentId = null;
    }));
    const cancel = el("button", "ghost small", t("doc_cancel"));
    cancel.addEventListener("click", () => { editingDocumentId = null; renderDocuments(); });
    bar.appendChild(save); bar.appendChild(cancel);
    form.appendChild(title); form.appendChild(body);
    if (d.kind === "invoice") form.appendChild(amount);
    form.appendChild(bar);
    wrap.appendChild(form);
  } else {
    const details = el("details");
    details.appendChild(el("summary", null, t("ws_view_content")));
    details.appendChild(el("pre", null, d.body));
    wrap.appendChild(details);
  }
  return wrap;
}

/** The exact message a send will compose, fetched when opened: what the
 *  founder approves is what they have read, headers and added facts too. */
function messagePreview(documentId) {
  const details = /** @type {HTMLDetailsElement} */ (el("details", "message-preview"));
  details.appendChild(el("summary", null, t("msg_preview")));
  const pre = el("pre", null, "…");
  details.appendChild(pre);
  details.addEventListener("toggle", async () => {
    if (!details.open || details.dataset.loaded) return;
    const result = await api("/api/workspace/message-preview", { document_id: documentId });
    if (!result.ok) { pre.textContent = result.error; return; }
    const p = result.preview;
    details.dataset.loaded = "1";
    // The envelope as a person reads it; the encoded original is one more
    // click away for anyone who wants the exact bytes.
    const envelope = el("dl", "role-facts");
    [["msg_from", p.from], ["msg_to", p.to], ["msg_subject", p.subject]].forEach(([key, value]) => {
      envelope.appendChild(el("dt", null, t(key)));
      envelope.appendChild(el("dd", null, value));
    });
    details.insertBefore(envelope, pre);
    pre.textContent = p.body;
    if (p.from_is_placeholder) details.appendChild(el("div", "status-line", t("msg_from_note")));
    if (p.to_is_placeholder) details.appendChild(el("div", "status-line", t("msg_to_note")));
    const raw = el("details");
    raw.appendChild(el("summary", null, t("msg_raw")));
    raw.appendChild(el("pre", null, p.message.replace(/\r\n/g, "\n")));
    details.appendChild(raw);
    details.appendChild(el("div", "status-line", t("msg_preview_note")));
  });
  return details;
}

/** Why a send waits for the founder, in their words; the engine's own
 *  wording stays one hover away rather than being the headline. */
function policyReason(action, reason) {
  const plain = action === "email.send";
  const node = el("div", "why", plain ? t("policy_send_reason") : reason);
  if (plain) node.title = reason;
  return node;
}

function renderApprovals() {
  const approvals = $("approvals");
  const pending = ws.approvals.filter(a => a.status === "pending");
  if (!pending.length) { approvals.replaceChildren(el("div", "empty", t("ws_no_approvals"))); return; }
  approvals.replaceChildren(...pending.map(a => {
    const doc = ws.documents.find(d => d.id === a.document_id);
    const row = el("div", "approval");
    const info = el("div");
    info.appendChild(el("div", null, t("ws_approval_for") + (doc ? doc.title : a.document_id)));
    info.appendChild(policyReason(a.action, a.policy_reason));
    info.appendChild(messagePreview(a.document_id));
    row.appendChild(info);
    const actions = el("div", "actions");
    const approve = el("button", "approve", "✓ " + t("ws_approve"));
    const reject = el("button", "reject", "✗ " + t("ws_reject"));
    const decide = async (yes) => {
      const title = doc ? doc.title : a.document_id;
      if (!await confirmAction(yes ? "approve" : "reject", title)) return;
      await applyResult(await api("/api/workspace/decide", { approval_id: a.id, approve: yes }), t(yes ? "toast_approved" : "toast_rejected")(title));
    };
    approve.addEventListener("click", () => withBusy(approve, () => decide(true)));
    reject.addEventListener("click", () => withBusy(reject, () => decide(false)));
    actions.appendChild(approve); actions.appendChild(reject);
    row.appendChild(actions);
    return row;
  }));
}

async function loadReceivables() {
  const data = await api("/api/workspace/receivables");
  const box = $("rc-table");
  const select = $("rc-invoice");
  /** @type {Receivable[]} */
  const rows = data.ok ? data.receivables : [];
  const open = rows.filter(r => r.outstanding_cents > 0);
  const selected = select.value;
  select.replaceChildren(...open.map(r => {
    const option = /** @type {HTMLOptionElement} */ (el("option", null, r.title + " · " + r.customer_name));
    option.value = r.invoice_id;
    return option;
  }));
  if (selected && open.some(r => r.invoice_id === selected)) select.value = selected;
  $("rc-form").hidden = !open.length;
  if (!rows.length) { box.replaceChildren(el("div", "empty", t("rc_none"))); return; }
  box.replaceChildren(table([t("th_invoice"), t("th_customer"), t("th_amount"), t("th_paid"), t("th_outstanding"), t("th_due"), t("th_status")],
    rows.map(r => [r.title, r.customer_name, {num: money(r.amount_cents)}, {num: money(r.paid_cents)}, {num: money(r.outstanding_cents)}, fmtDate(r.due_at),
      badge(r.status === "paid" ? "good" : r.status === "overdue" ? "bad" : "warn", t("rc_status")(r.status))])));
}

async function recordPayment() {
  const result = await api("/api/workspace/payment", { invoice_id: $("rc-invoice").value, amount: $("rc-amount").value, note: "" });
  if (await applyResult(result, t("toast_payment"))) $("rc-amount").value = "";
}

async function createDocument(kind) {
  const body = { customer_id: $("d-customer").value };
  if (kind === "invoice") body.amount = $("d-amount").value;
  const result = await api("/api/workspace/" + kind, body);
  $("d-status").textContent = result.ok ? t("saved") : "";
  await applyResult(result);
}

async function draftAssist() {
  const customer_id = $("d-customer").value;
  if (!customer_id) { $("d-status").textContent = t("ws_no_customers"); return; }
  $("d-status").textContent = t("assist_running");
  const result = await api("/api/workspace/assist", { customer_id });
  if (!result.ok) { $("d-status").textContent = ""; toast("bad", result.error); return; }
  $("d-status").textContent = "";
  const s = result.suggestion;
  $("assist-box").hidden = false;
  $("assist-label").textContent = t("ws_assist_label");
  $("assist-text").value = s.text;
  $("assist-meta").textContent = t("ws_assist_meta")(s);
  loadState();
}

function initCrm() {
  $("v-save").addEventListener("click", () => withBusy($("v-save"), saveCompany));
  $("c-add").addEventListener("click", () => withBusy($("c-add"), addCustomer));
  $("cd-save").addEventListener("click", () => withBusy($("cd-save"), saveCustomer));
  $("cd-project-add").addEventListener("click", () => withBusy($("cd-project-add"), addProject));
  $("cd-followup-add").addEventListener("click", () => withBusy($("cd-followup-add"), addFollowUp));
  $("cd-close").addEventListener("click", () => { selectedCustomerId = null; renderCustomerDetail(); });
  $("d-assist").addEventListener("click", () => withBusy($("d-assist"), draftAssist));
  $("d-offer").addEventListener("click", () => withBusy($("d-offer"), () => createDocument("offer")));
  $("d-invoice").addEventListener("click", () => withBusy($("d-invoice"), () => createDocument("invoice")));
  $("rc-record").addEventListener("click", () => withBusy($("rc-record"), recordPayment));
  $("assist-copy").addEventListener("click", () => {
    const text = $("assist-text").value;
    if (navigator.clipboard) navigator.clipboard.writeText(text).catch(() => {});
    $("assist-copy").textContent = t("ws_assist_copied");
    setTimeout(() => { $("assist-copy").textContent = t("ws_assist_copy"); }, 1500);
  });
  const fye = $("p-fye");
  fye.replaceChildren(...Array.from({ length: 12 }, (_, i) => {
    const option = /** @type {HTMLOptionElement} */ (el("option", null, String(i + 1)));
    option.value = String(i + 1);
    return option;
  }));
}
