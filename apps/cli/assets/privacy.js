"use strict";
// The data-sovereignty page: which route this device permits, and exactly
// what a public model would receive for a given task. Functions only;
// app.js calls initPrivacy().

/** @type {any} */
let privacyState = null;
/** @type {any} */
let lastPreview = null;

async function loadPrivacy() {
  const data = await api("/api/privacy/state");
  privacyState = data.ok ? data : null;
  renderPrivacy();
}

function renderPrivacy() {
  if (!privacyState || !ws) return;
  const preset = privacyState.preset;
  const box = $("pv-presets");
  box.replaceChildren(...[
    ["auto_protect", "pv_auto", "pv_auto_detail"],
    ["local_only", "pv_local", "pv_local_detail"],
  ].map(([value, nameKey, detailKey]) => {
    const row = el("div", "row");
    const bar = el("div", "toolbar");
    const mark = /** @type {HTMLButtonElement} */ (el("button",
      value === preset ? "primary small" : "ghost small",
      value === preset ? t("pv_selected") : t("pv_select")));
    mark.disabled = value === preset;
    mark.addEventListener("click", () => withBusy(mark, async () => {
      const result = await api("/api/privacy/preset", { preset: value });
      if (!result.ok) { toast("bad", result.error); return; }
      privacyState.preset = result.preset;
      renderPrivacy();
      loadState();
      toast("good", t("pv_preset_saved"));
    }));
    bar.appendChild(el("strong", null, t(nameKey)));
    bar.appendChild(mark);
    row.appendChild(bar);
    row.appendChild(el("p", null, t(detailKey)));
    return row;
  }));

  // Subjects to preview: a customer for the two drafting purposes, a
  // document for the review purpose.
  const purpose = $("pv-purpose").value;
  const select = $("pv-subject");
  const selected = select.value;
  const options = purpose === "review_draft"
    ? ws.documents.map(d => ({ label: d.title, id: d.id, kind: "document" }))
    : ws.customers.map(c => ({ label: c.name, id: c.id, kind: "customer" }));
  select.replaceChildren(...options.map(option => {
    const node = /** @type {HTMLOptionElement} */ (el("option", null, option.label));
    node.value = option.kind + ":" + option.id;
    return node;
  }));
  if (selected && options.some(o => o.kind + ":" + o.id === selected)) select.value = selected;
  select.disabled = !options.length;
  renderPreview();
}

async function runPreview() {
  const purpose = $("pv-purpose").value;
  const [kind, id] = ($("pv-subject").value || ":").split(":");
  if (!id) { toast("bad", t("pv_pick_subject")); return; }
  const body = { purpose };
  if (kind === "customer") body.customer_id = id;
  else { body.document_id = id; body.customer_id = (ws.documents.find(d => d.id === id) || {}).customer_id; }
  $("pv-status").textContent = "…";
  const data = await api("/api/privacy/preview", body);
  $("pv-status").textContent = "";
  if (!data.ok) { toast("bad", data.error); lastPreview = null; renderPreview(); return; }
  lastPreview = data.preview;
  renderPreview();
}

const OUTCOME_KIND = { sent: "warn", replaced: "good", omitted: "good", absent: "neutral" };

function renderPreview() {
  const box = $("pv-preview");
  if (!lastPreview) {
    box.className = "empty";
    box.replaceChildren(document.createTextNode(t("pv_no_preview")));
    return;
  }
  box.className = "";
  const p = lastPreview;
  const children = [];

  const head = el("div", "row toolbar");
  head.appendChild(badge(p.placement === "local" ? "good" : "warn", t("pv_placement")(p.placement)));
  head.appendChild(el("span", "mono", p.transform_id));
  head.appendChild(el("span", "status-line", t("pv_bytes")(p.outbound_bytes)));
  children.push(head);

  children.push(table(
    [t("pv_th_field"), t("pv_th_outcome"), t("pv_th_note")],
    p.fields.map(f => {
      const outcome = el("span", null);
      outcome.appendChild(badge(OUTCOME_KIND[f.outcome] || "neutral", t("pv_outcome")(f.outcome)));
      if (f.placeholder) outcome.appendChild(el("span", "mono", " " + f.placeholder));
      const note = el("span", null, f.note);
      if (f.warning) { note.appendChild(el("br")); note.appendChild(el("span", "badge warn", f.warning)); }
      return [f.field, outcome, note];
    })));

  const exact = el("div", "row");
  exact.appendChild(el("div", "status-line", t("pv_exact")));
  exact.appendChild(el("pre", null, p.outbound_text));
  exact.appendChild(el("div", "mono", "sha256 " + p.outbound_digest.slice(0, 24) + "…"));
  children.push(exact);

  if (!p.dispatch_available) {
    children.push(el("div", "row status-line", t("pv_no_dispatch")));
  }
  box.replaceChildren(...children);
}

function initPrivacy() {
  $("pv-purpose").addEventListener("change", () => { lastPreview = null; renderPrivacy(); });
  $("pv-preview-run").addEventListener("click", () => withBusy($("pv-preview-run"), runPreview));
}
