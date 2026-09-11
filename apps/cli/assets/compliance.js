"use strict";
// Compliance view: run checks, read findings with their sources, search the
// rule pack. Functions only; app.js calls initCompliance().

/** @type {any[]} */
let rulePacks = [];

function findingKind(status) {
  return { pass: "good", attention: "bad", needs_review: "warn", not_applicable: "neutral", unknown: "warn" }[status] || "neutral";
}

function overallKind(overall) {
  return { attention: "bad", needs_review: "warn", unknown: "warn", not_covered: "neutral", clear_within_pack: "good" }[overall] || "neutral";
}

/** Compact report line used inside proposal cards and the history list. */
function renderReportSummary(report) {
  const row = el("div", "toolbar");
  row.appendChild(badge(overallKind(report.overall), t("cp_overall")(report.overall)));
  row.appendChild(el("span", null, t("cp_report_line")(report) + " · " + fmtTime(report.at)));
  return row;
}

function renderReport(report) {
  const box = $("cp-report");
  if (!report) { box.replaceChildren(el("div", "empty", t("cp_no_reports"))); return; }
  const children = [];
  const head = el("div", "toolbar");
  head.appendChild(badge(overallKind(report.overall), t("cp_overall")(report.overall)));
  head.appendChild(el("span", "status-line", t("cp_report_line")(report) + " · " + fmtTime(report.at)));
  if (report.pack_version) head.appendChild(el("span", "mono", report.pack_id + " " + report.pack_version));
  children.push(head);
  if (report.review_status) children.push(el("div", "honest", report.review_status));
  if (report.model_summary) {
    const summary = el("div", "panel pad");
    summary.appendChild(el("strong", null, t("cp_summary") + " "));
    summary.appendChild(badge("good", t("cp_summary_model")));
    summary.appendChild(el("p", null, report.model_summary));
    children.push(summary);
  }
  const order = ["attention", "needs_review", "unknown", "pass", "not_applicable"];
  report.findings.slice().sort((a, b) => order.indexOf(a.status) - order.indexOf(b.status)).forEach(f => {
    const row = el("div", "finding " + f.status);
    const top = el("div", "toolbar");
    top.appendChild(badge(findingKind(f.status), t("cp_status")(f.status)));
    top.appendChild(el("strong", null, f.title));
    top.appendChild(el("span", "pill", t("cp_basis")(f.basis)));
    top.appendChild(el("span", "mono", f.rule_id));
    row.appendChild(top);
    row.appendChild(el("p", null, f.detail));
    const meta = el("div", "status-line");
    meta.appendChild(document.createTextNode(t("cp_source") + ": "));
    if (f.source_url) {
      const link = /** @type {HTMLAnchorElement} */ (el("a", null, f.source_authority + " — " + f.source_title));
      link.href = f.source_url; link.target = "_blank"; link.rel = "noopener noreferrer";
      meta.appendChild(link);
    } else {
      meta.appendChild(document.createTextNode(f.source_authority + " — " + f.source_title));
    }
    row.appendChild(meta);
    row.appendChild(el("div", "status-line", t("cp_escalation") + ": " + f.escalation));
    row.appendChild(el("div", "status-line", t("cp_facts") + ": " + f.facts_used.map(k => t("fact_label")(k)).join(", ")));
    children.push(row);
  });
  box.replaceChildren(...children);
}

function renderCompliance() {
  if (!ws) return;
  const jurisdiction = (ws.venture && ws.venture.jurisdiction) || "";
  const pack = rulePacks.find(p => p.jurisdiction === jurisdiction);
  $("cp-pack").textContent = pack ? t("cp_pack")(pack) : t("cp_no_pack");
  const select = $("cp-invoice");
  const selected = select.value;
  const invoices = ws.documents.filter(d => d.kind === "invoice");
  select.replaceChildren(el("option", null, t("cp_pick_invoice")), ...invoices.map(d => {
    const option = /** @type {HTMLOptionElement} */ (el("option", null, d.title));
    option.value = d.id;
    return option;
  }));
  if (selected && invoices.some(d => d.id === selected)) select.value = selected;
  const reports = (ws.compliance_reports || []).slice().reverse();
  renderReport(reports[0] || null);
  const history = $("cp-history");
  // The newest report is shown above; this list is everything before it.
  if (reports.length <= 1) { history.replaceChildren(el("div", "empty", t(reports.length ? "cp_no_earlier" : "cp_no_reports"))); return; }
  history.replaceChildren(...reports.slice(1).map(r => {
    const row = renderReportSummary(r);
    const open = el("button", "ghost small", t("cu_open"));
    open.addEventListener("click", () => renderReport(r));
    row.appendChild(open);
    return row;
  }));
}

async function runCheck(documentId) {
  $("cp-status").textContent = t("cp_running");
  const result = await api("/api/workspace/compliance/check", { document_id: documentId || null });
  $("cp-status").textContent = "";
  if (!result.ok) { toast("bad", result.error); return; }
  toast("good", t("toast_checked"));
  await loadWorkspace();
  loadState();
  loadCommandCenter();
}

async function loadPacks() {
  const data = await api("/api/workspace/compliance/rules", { query: "" });
  if (data.ok) rulePacks = data.packs;
  renderCompliance();
}

async function searchRules() {
  const query = $("cp-query").value;
  const data = await api("/api/workspace/compliance/rules", { query });
  const box = $("cp-hits");
  if (!data.ok || !data.hits.length) { box.replaceChildren(el("div", "empty", t("cp_no_hits"))); return; }
  box.replaceChildren(...data.hits.map(h => {
    const row = el("div", "finding");
    const top = el("div", "toolbar");
    top.appendChild(el("strong", null, h.title));
    top.appendChild(el("span", "pill", t("cp_basis")(h.basis)));
    top.appendChild(el("span", "mono", h.rule_id));
    row.appendChild(top);
    row.appendChild(el("p", null, h.summary));
    const link = /** @type {HTMLAnchorElement} */ (el("a", "status-line", t("cp_source") + ": " + h.source_title));
    link.href = h.source_url; link.target = "_blank"; link.rel = "noopener noreferrer";
    row.appendChild(link);
    row.appendChild(el("div", "status-line", t("cp_matched")(h.matched)));
    return row;
  }));
}

function initCompliance() {
  $("cp-run-company").addEventListener("click", () => withBusy($("cp-run-company"), () => runCheck(null)));
  $("cp-run-invoice").addEventListener("click", () => withBusy($("cp-run-invoice"), () => runCheck($("cp-invoice").value)));
  $("cp-search").addEventListener("click", () => withBusy($("cp-search"), searchRules));
  $("cp-query").addEventListener("keydown", (event) => { if (event.key === "Enter") searchRules(); });
}
