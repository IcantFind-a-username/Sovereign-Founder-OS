"use strict";
// Team view (hire, run) plus the proposals inbox and work suggestions that
// the Today view shows. Functions only; app.js calls initTeam().

/**
 * @typedef {{id: string, title_en: string, title_zh: string, description_en: string, description_zh: string, reads: string, delivers: string, cannot: string}} RoleCard
 * @typedef {{id: string, role: string, name: string, status: string, hired_at: number, runs: number, last_run_at: number|null}} Employee
 * @typedef {{id: string, employee_id: string, role: string, title: string, summary: string, change: any, evidence: string[], provider_id: string, provider_trust: string, model_backed: boolean, status: string, created_at: number, decided_at: number|null, outcome: string|null}} Decision
 * @typedef {{employee_id: string, role: string, subject: {customer_id: string|null, document_id: string|null, project_id: string|null}, subject_name: string, reason: string}} WorkSuggestion
 */

/** @type {RoleCard[]} */
let roleCards = [];
/** @type {WorkSuggestion[]} */
let workSuggestions = [];

async function loadRoles() {
  const data = await api("/api/workspace/roles");
  if (data.ok) roleCards = data.roles;
  renderTeam();
}

function roleTitle(card) { return lang === "zh" ? card.title_zh : card.title_en; }
function roleDescription(card) { return lang === "zh" ? card.description_zh : card.description_en; }

function renderTeam() {
  if (!ws) return;
  const employees = /** @type {Employee[]} */ (ws.employees || []);
  const grid = $("tm-roles");
  grid.replaceChildren(...roleCards.map(card => {
    const node = el("div", "card");
    const head = el("div", "card-head");
    head.appendChild(el("strong", null, roleTitle(card)));
    const hired = employees.find(e => e.role === card.id);
    if (hired) head.appendChild(badge(hired.status === "hired" ? "good" : "warn", hired.status === "hired" ? t("tm_hired") : t("tm_paused")));
    node.appendChild(head);
    node.appendChild(el("p", null, roleDescription(card)));
    const facts = el("dl", "role-facts");
    [["tm_reads", card.reads], ["tm_delivers", card.delivers], ["tm_cannot", card.cannot]].forEach(([key, value]) => {
      facts.appendChild(el("dt", null, t(key)));
      facts.appendChild(el("dd", null, value));
    });
    node.appendChild(facts);
    const bar = el("div", "toolbar");
    if (!hired) {
      const name = /** @type {HTMLInputElement} */ (el("input"));
      name.placeholder = t("tm_name_placeholder"); name.maxLength = 60; name.value = roleTitle(card);
      const hire = el("button", "primary small", t("tm_hire"));
      hire.addEventListener("click", () => withBusy(hire, async () => {
        const result = await api("/api/workspace/employee/hire", { role: card.id, name: name.value });
        if (await applyResult(result, t("toast_hired")(name.value))) loadWorkSuggestions();
      }));
      bar.appendChild(name); bar.appendChild(hire);
    } else {
      bar.appendChild(el("span", "status-line", hired.name + " · " + t("tm_runs")(hired.runs)));
      const toggle = el("button", "ghost small", hired.status === "hired" ? t("tm_pause") : t("tm_resume"));
      toggle.addEventListener("click", () => withBusy(toggle, async () => {
        const result = await api("/api/workspace/employee/status", { employee_id: hired.id, status: hired.status === "hired" ? "paused" : "hired" });
        if (await applyResult(result)) loadWorkSuggestions();
      }));
      bar.appendChild(toggle);
    }
    node.appendChild(bar);
    return node;
  }));

  const select = $("tm-employee");
  const selected = select.value;
  const active = employees.filter(e => e.status === "hired");
  select.replaceChildren(...active.map(e => {
    const option = /** @type {HTMLOptionElement} */ (el("option", null, t("role_title")(e.role) + " · " + e.name));
    option.value = e.id;
    return option;
  }));
  if (selected && active.some(e => e.id === selected)) select.value = selected;
  $("tm-run-panel").hidden = !active.length;
  $("tm-none").hidden = !!active.length;
  renderSubjectOptions();
  renderHistory();
}

/**
 * Subjects an employee can work on, from state, per role.
 * @returns {Array<{label: string, subject: any}>}
 */
function subjectOptionsFor(role) {
  /** @type {Array<{label: string, subject: any}>} */
  const customers = ws.customers.map(c => ({ label: c.name, subject: { customer_id: c.id } }));
  switch (role) {
    case "analyst":
    case "proposal_writer":
      return customers;
    case "delivery_planner":
      return ws.documents.filter(d => d.kind === "offer" && (d.status === "approved_pending_delivery" || d.status === "delivered"))
        .map(d => ({ label: d.title, subject: { customer_id: d.customer_id, document_id: d.id } })).concat(customers);
    case "invoice_clerk":
      return ws.projects.filter(p => p.status === "done").map(p => ({ label: p.name, subject: { customer_id: p.customer_id, project_id: p.id } }));
    case "quality_checker":
      return ws.documents.filter(d => d.status === "draft").map(d => ({ label: d.title, subject: { customer_id: d.customer_id, document_id: d.id } }));
    case "compliance_checker": {
      /** @type {Array<{label: string, subject: any}>} */
      const company = [{ label: (ws.venture && ws.venture.name) || "—", subject: {} }];
      return company.concat(ws.documents.filter(d => d.kind === "invoice").map(d => ({ label: d.title, subject: { document_id: d.id } })));
    }
    default:
      return [];
  }
}

function renderSubjectOptions() {
  const employee = (ws.employees || []).find(e => e.id === $("tm-employee").value);
  const select = $("tm-subject");
  const options = employee ? subjectOptionsFor(employee.role) : [];
  select.replaceChildren(...options.map((option, index) => {
    const node = /** @type {HTMLOptionElement} */ (el("option", null, option.label));
    node.value = String(index);
    return node;
  }));
  select.disabled = !options.length;
  if (!options.length) select.replaceChildren(el("option", null, t("tm_pick_subject")));
}

async function runEmployee(employeeId, subject) {
  const result = await api("/api/workspace/employee/run", Object.assign({ employee_id: employeeId }, subject));
  if (!result.ok) { toast("bad", result.error); return false; }
  toast("good", t("toast_ran")(result.decision.title));
  await loadWorkspace();
  loadCommandCenter();
  loadState();
  loadWorkSuggestions();
  return true;
}

async function runSelectedEmployee() {
  const employee = (ws.employees || []).find(e => e.id === $("tm-employee").value);
  if (!employee) return;
  const options = subjectOptionsFor(employee.role);
  const option = options[Number($("tm-subject").value)];
  $("tm-run-status").textContent = "…";
  const ok = await runEmployee(employee.id, option ? option.subject : {});
  $("tm-run-status").textContent = ok ? t("saved") : "";
}

/* ─────────────── Proposals inbox ─────────────── */

function renderChange(change) {
  const box = el("div", "change");
  const list = (label, items) => {
    if (!items || !items.length) return;
    box.appendChild(el("div", "status-line", label));
    const ul = el("ul", "criteria");
    items.forEach(item => ul.appendChild(el("li", null, typeof item === "string" ? item : (item.title ? item.title + " · " + item.due_in_days + "d" : item.kind + ": " + item.detail))));
    box.appendChild(ul);
  };
  switch (change.kind) {
    case "discovery_summary":
      list("Problems", change.problems); list("Constraints", change.constraints);
      box.appendChild(el("div", null, "Budget: " + change.budget));
      list("Open questions", change.open_questions); list("Assumptions", change.assumptions);
      break;
    case "offer_draft":
    case "invoice_draft": {
      const head = el("div", null, change.title + (change.amount_cents != null ? " · " + money(change.amount_cents) : ""));
      box.appendChild(head);
      const details = el("details");
      details.appendChild(el("summary", null, t("ws_view_content")));
      details.appendChild(el("pre", null, change.body));
      box.appendChild(details);
      list("Assumptions", change.assumptions);
      if (change.due_in_days) box.appendChild(el("div", "status-line", t("doc_due")(change.due_in_days + "d")));
      break;
    }
    case "delivery_plan":
      box.appendChild(el("div", null, change.project_name));
      list(t("cu_tasks"), change.tasks); list("Acceptance", change.acceptance_criteria);
      break;
    case "review_findings":
      list("Findings", change.findings.length ? change.findings : ["—"]);
      break;
    case "compliance_report": {
      const report = (ws.compliance_reports || []).find(r => r.id === change.report_id);
      if (report) box.appendChild(renderReportSummary(report));
      break;
    }
    default:
      box.appendChild(el("pre", null, JSON.stringify(change, null, 2)));
  }
  return box;
}

function renderProposal(decision, showActions) {
  const wrap = el("div", "doc");
  const head = el("div", "doc-head");
  head.appendChild(el("span", "title", decision.title));
  head.appendChild(badge(decision.status === "approved" ? "good" : decision.status === "rejected" ? "bad" : "warn", t("prop_status")(decision.status)));
  head.appendChild(badge(decision.model_backed ? "good" : "neutral", decision.model_backed ? t("prop_model") : t("prop_template")));
  head.appendChild(el("span", "status-line", t("prop_provider") + ": " + decision.provider_id));
  wrap.appendChild(head);
  wrap.appendChild(el("p", null, decision.summary));
  const details = el("details");
  details.appendChild(el("summary", null, t("prop_change_heading")));
  details.appendChild(renderChange(decision.change));
  details.appendChild(el("div", "status-line", t("prop_evidence") + ": " + decision.evidence.join(", ")));
  wrap.appendChild(details);
  if (decision.outcome) wrap.appendChild(el("div", "mono", t("prop_outcome") + ": " + decision.outcome));
  if (showActions && decision.status === "pending") {
    const bar = el("div", "toolbar");
    const approve = el("button", "primary small", t("prop_approve"));
    const reject = el("button", "ghost small", t("prop_reject"));
    approve.addEventListener("click", () => withBusy(approve, () => decideProposal(decision, true)));
    reject.addEventListener("click", () => withBusy(reject, () => decideProposal(decision, false)));
    bar.appendChild(approve); bar.appendChild(reject);
    wrap.appendChild(bar);
  }
  return wrap;
}

async function decideProposal(decision, approve) {
  if (!await confirmAction(approve ? "approve_prop" : "reject_prop", decision.title)) return;
  const result = await api("/api/workspace/decision", { decision_id: decision.id, approve });
  if (await applyResult(result, t(approve ? "toast_prop_approved" : "toast_prop_rejected")(decision.title))) loadWorkSuggestions();
}

function renderProposalsInbox() {
  if (!ws) return;
  const box = $("cc-proposals");
  const pending = (ws.decisions || []).filter(d => d.status === "pending");
  if (!pending.length) { box.className = "empty"; box.replaceChildren(document.createTextNode(t("cc_no_proposals"))); return; }
  box.className = "";
  box.replaceChildren(...pending.slice().reverse().map(d => renderProposal(d, true)));
}

function renderHistory() {
  const box = $("tm-history");
  const decisions = (ws.decisions || []).slice().reverse();
  if (!decisions.length) { box.replaceChildren(el("div", "empty", t("tm_no_history"))); return; }
  box.replaceChildren(...decisions.map(d => renderProposal(d, false)));
}

/* ─────────────── Work suggestions ─────────────── */

async function loadWorkSuggestions() {
  const data = await api("/api/workspace/work-suggestions");
  workSuggestions = data.ok ? data.suggestions : [];
  renderWorkSuggestions();
}

function renderWorkSuggestions() {
  const box = $("cc-work");
  if (!workSuggestions.length) { box.className = "empty"; box.replaceChildren(document.createTextNode(t("cc_no_work"))); return; }
  box.className = "";
  box.replaceChildren(...workSuggestions.map(s => {
    const row = el("div", "pad toolbar");
    row.appendChild(badge("neutral", t("role_title")(s.role)));
    row.appendChild(el("span", null, t("work_reason")(s.reason) + " · " + s.subject_name));
    const run = el("button", "primary small", t("cc_run"));
    run.addEventListener("click", () => withBusy(run, () => runEmployee(s.employee_id, {
      customer_id: s.subject.customer_id, document_id: s.subject.document_id, project_id: s.subject.project_id,
    })));
    row.appendChild(run);
    return row;
  }));
}

function initTeam() {
  $("tm-employee").addEventListener("change", renderSubjectOptions);
  $("tm-run").addEventListener("click", () => withBusy($("tm-run"), runSelectedEmployee));
}
