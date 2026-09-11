//! Compliance checks over the founder's recorded facts: retrieval over the
//! jurisdiction rule pack (keyword search with citations), deterministic
//! rule checks, and a report that never says "fully compliant". A real
//! model, when available, may add a plain-language summary that is kept
//! only if every rule it cites was actually retrieved. Model opinion never
//! changes a finding's status: the deterministic checks decide.

use super::compliance_pack::{pack_for, packs, CheckKind, Rule, RulePack, SG_GST_THRESHOLD_CENTS};
use super::model_config::providers_for;
use super::store::AuditEntry;
use super::util::now;
use super::*;

use super::crew_roles::Rejection;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use sovereign_contracts::{AutomationLevel, DataClass};
use sovereign_model::{ModelGateway, ModelRequest};
use uuid::Uuid;

const DAY_SECONDS: i64 = 86_400;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    /// The recorded facts satisfy the rule as this pack states it.
    Pass,
    /// Action is likely needed; the facts point at an obligation.
    Attention,
    /// The check cannot decide; a person (often a professional) must.
    NeedsReview,
    NotApplicable,
    /// A required fact is missing, so nothing can be said.
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceFinding {
    pub rule_id: String,
    pub category: String,
    pub basis: String,
    pub title: String,
    pub status: FindingStatus,
    pub detail: String,
    /// Field paths the check read (never values).
    pub facts_used: Vec<String>,
    pub source_authority: String,
    pub source_title: String,
    pub source_url: String,
    pub escalation: String,
}

/// One check run, bound to the facts it saw. `overall` is one of
/// `attention`, `needs_review`, `clear_within_pack`, `not_covered`, or
/// `unknown` — deliberately never "compliant".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub id: Uuid,
    pub at: i64,
    pub jurisdiction: String,
    pub pack_id: String,
    pub pack_version: String,
    pub review_status: String,
    /// `venture` or `document:<id>`.
    pub subject: String,
    pub coverage: String,
    pub overall: String,
    pub findings: Vec<ComplianceFinding>,
    /// Rule ids the retrieval step surfaced for the model's summary.
    pub retrieved_rule_ids: Vec<String>,
    pub model_summary: Option<String>,
    pub model_backed: bool,
    /// Why a model summary was refused, when one was received. A category,
    /// never the model's text.
    #[serde(default)]
    pub rejection: Option<String>,
    pub provider_id: String,
    /// Digest of the facts snapshot, so a later report can show the facts
    /// changed and this one no longer applies.
    pub facts_digest: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleHit {
    pub rule_id: String,
    pub title: String,
    pub summary: String,
    pub category: String,
    pub basis: String,
    pub source_title: String,
    pub source_url: String,
    pub score: u32,
    pub matched: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct ComplianceSubject {
    #[serde(default)]
    pub document_id: Option<Uuid>,
}

fn invalid(message: &str) -> WorkspaceError {
    WorkspaceError::Invalid(message.to_owned())
}

// ---------------------------------------------------------------------------
// Retrieval: keyword search over the pack with per-term weighting. ASCII
// text is tokenised on non-alphanumerics; CJK text is matched as bigrams.
// Small, deterministic, and inspectable — the first rung of the blueprint's
// "exact term match before vectors" ladder.
// ---------------------------------------------------------------------------

fn tokens(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut out = Vec::new();
    let mut word = String::new();
    let mut previous_cjk: Option<char> = None;
    for ch in lower.chars() {
        let cjk = ('\u{4e00}'..='\u{9fff}').contains(&ch);
        if ch.is_ascii_alphanumeric() || ch == '-' {
            word.push(ch);
            previous_cjk = None;
            continue;
        }
        if !word.is_empty() {
            out.push(std::mem::take(&mut word));
        }
        if cjk {
            if let Some(previous) = previous_cjk {
                out.push(format!("{previous}{ch}"));
            }
            previous_cjk = Some(ch);
        } else {
            previous_cjk = None;
        }
    }
    if !word.is_empty() {
        out.push(word);
    }
    out.retain(|token| token.len() >= 2 || token.chars().count() >= 2);
    out.dedup();
    out
}

fn rule_text(rule: &Rule, zh: bool) -> String {
    let mut text = format!(
        "{} {} {} {}",
        rule.title_en, rule.summary_en, rule.title_zh, rule.summary_zh
    );
    for tag in rule.tags {
        text.push(' ');
        text.push_str(tag);
    }
    if zh {
        text.push(' ');
        text.push_str(rule.category);
    }
    text
}

/// Top-k rules for a query, with the matched terms shown so the founder
/// can see why each hit surfaced. Empty query → empty result, never "all".
pub fn retrieve(pack: &RulePack, query: &str, k: usize, zh: bool) -> Vec<RuleHit> {
    let query_tokens = tokens(query);
    if query_tokens.is_empty() {
        return Vec::new();
    }
    let documents: Vec<Vec<String>> = pack
        .rules
        .iter()
        .map(|rule| tokens(&rule_text(rule, zh)))
        .collect();
    let mut hits: Vec<RuleHit> = pack
        .rules
        .iter()
        .zip(documents.iter())
        .filter_map(|(rule, document)| {
            let mut score = 0u32;
            let mut matched = Vec::new();
            for token in &query_tokens {
                let occurrences = document.iter().filter(|term| *term == token).count() as u32;
                if occurrences == 0 {
                    continue;
                }
                let containing = documents
                    .iter()
                    .filter(|other| other.contains(token))
                    .count() as u32;
                // Rarer terms weigh more: a term in every rule adds one,
                // a term in one rule adds the pack size.
                let weight = (pack.rules.len() as u32) / containing.max(1);
                score += weight * occurrences.min(3);
                matched.push(token.clone());
            }
            if score == 0 {
                return None;
            }
            Some(RuleHit {
                rule_id: rule.id.to_owned(),
                title: if zh { rule.title_zh } else { rule.title_en }.to_owned(),
                summary: if zh { rule.summary_zh } else { rule.summary_en }.to_owned(),
                category: rule.category.to_owned(),
                basis: rule.basis.to_owned(),
                source_title: rule.source.title.to_owned(),
                source_url: rule.source.url.to_owned(),
                score,
                matched,
            })
        })
        .collect();
    hits.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });
    hits.truncate(k);
    hits
}

// ---------------------------------------------------------------------------
// Deterministic checks
// ---------------------------------------------------------------------------

struct Facts<'a> {
    workspace: &'a Workspace,
    venture: &'a Venture,
    now_unix: i64,
    document: Option<&'a Document>,
}

fn finding(
    rule: &Rule,
    status: FindingStatus,
    detail: String,
    facts_used: &[&str],
    zh: bool,
) -> ComplianceFinding {
    ComplianceFinding {
        rule_id: rule.id.to_owned(),
        category: rule.category.to_owned(),
        basis: rule.basis.to_owned(),
        title: if zh { rule.title_zh } else { rule.title_en }.to_owned(),
        status,
        detail,
        facts_used: facts_used.iter().map(|fact| (*fact).to_owned()).collect(),
        source_authority: rule.source.authority.to_owned(),
        source_title: rule.source.title.to_owned(),
        source_url: rule.source.url.to_owned(),
        escalation: if zh {
            rule.escalation_zh
        } else {
            rule.escalation_en
        }
        .to_owned(),
    }
}

fn last_day_of_month(year: i32, month: u32) -> NaiveDate {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|first| first.pred_opt())
        .expect("valid month")
}

/// The most recent financial year end on or before `today`.
fn last_financial_year_end(today: NaiveDate, fye_month: u8) -> NaiveDate {
    let month = u32::from(fye_month.clamp(1, 12));
    let this_year = last_day_of_month(today.year(), month);
    if this_year <= today {
        this_year
    } else {
        last_day_of_month(today.year() - 1, month)
    }
}

fn add_months(date: NaiveDate, months: u32) -> NaiveDate {
    let total = date.month0() + months;
    let year = date.year() + (total / 12) as i32;
    let month = total % 12 + 1;
    let last = last_day_of_month(year, month);
    NaiveDate::from_ymd_opt(year, month, date.day().min(last.day())).expect("valid date")
}

fn money(cents: u64, currency: &str) -> String {
    format!("{currency} {}.{:02}", cents / 100, cents % 100)
}

fn trailing_receipts_cents(workspace: &Workspace, now_unix: i64) -> u64 {
    workspace
        .payments
        .iter()
        .filter(|payment| payment.received_at >= now_unix - 365 * DAY_SECONDS)
        .map(|payment| payment.amount_cents)
        .sum()
}

fn check_rule(rule: &Rule, facts: &Facts<'_>, zh: bool) -> Option<ComplianceFinding> {
    let venture = facts.venture;
    let workspace = facts.workspace;
    let today = chrono::DateTime::from_timestamp(facts.now_unix, 0)
        .map(|stamp| stamp.date_naive())
        .unwrap_or_default();
    let t = |en: String, zh_text: String| if zh { zh_text } else { en };
    match rule.check {
        CheckKind::JurisdictionKnown => Some(if venture.jurisdiction.is_empty() {
            finding(
                rule,
                FindingStatus::Unknown,
                t(
                    "No jurisdiction is recorded for the company.".into(),
                    "公司资料没有填写辖区。".into(),
                ),
                &["venture.jurisdiction"],
                zh,
            )
        } else {
            finding(
                rule,
                FindingStatus::Pass,
                t(
                    format!("Jurisdiction recorded as {}.", venture.jurisdiction),
                    format!("辖区记录为 {}。", venture.jurisdiction),
                ),
                &["venture.jurisdiction"],
                zh,
            )
        }),
        CheckKind::GstRegistrationThreshold => {
            if facts.document.is_some() {
                return None;
            }
            let receipts = trailing_receipts_cents(workspace, facts.now_unix);
            let basis = venture
                .revenue_estimate_cents
                .map(|estimate| estimate.max(receipts))
                .unwrap_or(receipts);
            let used = [
                "venture.gst_registered",
                "venture.revenue_estimate_cents",
                "payments.trailing_12_months",
            ];
            Some(if venture.gst_registered {
                finding(
                    rule,
                    FindingStatus::Pass,
                    t(
                        "The company is recorded as GST-registered.".into(),
                        "公司资料记录为已注册 GST。".into(),
                    ),
                    &used,
                    zh,
                )
            } else if basis >= SG_GST_THRESHOLD_CENTS {
                finding(rule, FindingStatus::Attention, t(format!("Recorded revenue ({}) is at or above the pack's S$1,000,000 threshold and the company is not recorded as GST-registered.", money(basis, &venture.currency)), format!("记录的营收({})已达到或超过规则包中的 100 万新元门槛,而公司未记录为 GST 注册。", money(basis, &venture.currency))), &used, zh)
            } else if venture.revenue_estimate_cents.is_none() && receipts == 0 {
                finding(rule, FindingStatus::NeedsReview, t("No revenue estimate and no recorded receipts: the threshold cannot be checked.".into(), "没有营收估计也没有收款记录,无法检查门槛。".into()), &used, zh)
            } else {
                finding(
                    rule,
                    FindingStatus::Pass,
                    t(
                        format!(
                            "Recorded revenue ({}) is below the pack's S$1,000,000 threshold.",
                            money(basis, &venture.currency)
                        ),
                        format!(
                            "记录的营收({})低于规则包中的 100 万新元门槛。",
                            money(basis, &venture.currency)
                        ),
                    ),
                    &used,
                    zh,
                )
            })
        }
        CheckKind::GstTaxInvoiceFields => {
            let document = facts.document?;
            if document.kind != DocumentKind::Invoice {
                return None;
            }
            if !venture.gst_registered {
                return Some(finding(
                    rule,
                    FindingStatus::NotApplicable,
                    t(
                        "The company is not recorded as GST-registered.".into(),
                        "公司未记录为 GST 注册。".into(),
                    ),
                    &["venture.gst_registered"],
                    zh,
                ));
            }
            let body = document.body.to_lowercase();
            let mentions_gst = body.contains("gst") || body.contains("消费税");
            let has_uen = !venture.uen.trim().is_empty();
            let used = ["venture.gst_registered", "venture.uen", "document.body"];
            Some(if mentions_gst && has_uen {
                finding(rule, FindingStatus::NeedsReview, t("The invoice mentions GST and a registration number is on file; check the remaining tax-invoice particulars against the IRAS list.".into(), "发票提到了 GST,且资料中有注册号;请对照 IRAS 清单核对其余项目。".into()), &used, zh)
            } else {
                let mut missing = Vec::new();
                if !mentions_gst {
                    missing.push(if zh {
                        "GST 金额/说明"
                    } else {
                        "a GST amount or statement"
                    });
                }
                if !has_uen {
                    missing.push(if zh {
                        "公司 UEN/GST 注册号"
                    } else {
                        "the company's UEN / GST registration number"
                    });
                }
                finding(
                    rule,
                    FindingStatus::Attention,
                    t(
                        format!(
                            "A GST-registered company's invoice is missing: {}.",
                            missing.join(", ")
                        ),
                        format!("GST 注册公司的发票缺少:{}。", missing.join("、")),
                    ),
                    &used,
                    zh,
                )
            })
        }
        CheckKind::EInvoicing => {
            if facts.document.is_some() || !venture.gst_registered {
                return None;
            }
            let recent = venture
                .incorporated_at
                .is_some_and(|at| at >= 1_746_057_600); // 2025-05-01
            Some(if venture.incorporated_at.is_none() {
                finding(rule, FindingStatus::Unknown, t("Incorporation date not recorded; the InvoiceNow phase cannot be determined.".into(), "未记录成立日期,无法判断 InvoiceNow 阶段。".into()), &["venture.incorporated_at", "venture.gst_registered"], zh)
            } else if recent {
                finding(rule, FindingStatus::NeedsReview, t("A recently incorporated, GST-registered company may fall in an InvoiceNow phase; check the IRAS timeline.".into(), "新近成立且已注册 GST 的公司可能属于 InvoiceNow 的某个阶段;请核对 IRAS 时间表。".into()), &["venture.incorporated_at", "venture.gst_registered"], zh)
            } else {
                finding(rule, FindingStatus::NeedsReview, t("InvoiceNow is being phased in; confirm whether a later phase reaches existing GST-registered companies.".into(), "InvoiceNow 正分阶段推行;确认后续阶段是否覆盖既有 GST 注册公司。".into()), &["venture.incorporated_at", "venture.gst_registered"], zh)
            })
        }
        CheckKind::InvoiceEssentials => {
            let document = facts.document?;
            if document.kind != DocumentKind::Invoice {
                return None;
            }
            let customer_name = workspace
                .customer(document.customer_id)
                .map(|customer| customer.name.clone())
                .unwrap_or_default();
            let mut missing = Vec::new();
            if !document.body.contains(&venture.name) {
                missing.push(if zh {
                    "公司名称"
                } else {
                    "the company name"
                });
            }
            if customer_name.is_empty() || !document.body.contains(&customer_name) {
                missing.push(if zh {
                    "客户名称"
                } else {
                    "the customer name"
                });
            }
            if document.amount_cents.unwrap_or(0) == 0 {
                missing.push(if zh { "金额" } else { "an amount" });
            }
            if document.body.trim().lines().count() < 4 {
                missing.push(if zh {
                    "交付内容说明"
                } else {
                    "a description of what was delivered"
                });
            }
            let used = [
                "document.body",
                "document.amount_cents",
                "venture.name",
                "customer.name",
            ];
            Some(if missing.is_empty() {
                finding(rule, FindingStatus::Pass, t("The invoice names both parties, states an amount, and describes the delivery.".into(), "发票写明了双方、金额和交付内容。".into()), &used, zh)
            } else {
                finding(
                    rule,
                    FindingStatus::Attention,
                    t(
                        format!("The invoice is missing: {}.", missing.join(", ")),
                        format!("发票缺少:{}。", missing.join("、")),
                    ),
                    &used,
                    zh,
                )
            })
        }
        CheckKind::EstimatedChargeableIncome => {
            if facts.document.is_some() {
                return None;
            }
            let used = ["venture.incorporated_at", "venture.fiscal_year_end_month"];
            let Some(incorporated_at) = venture.incorporated_at else {
                return Some(finding(
                    rule,
                    FindingStatus::Unknown,
                    t(
                        "Incorporation date not recorded; the ECI window cannot be computed."
                            .into(),
                        "未记录成立日期,无法计算 ECI 期限。".into(),
                    ),
                    &used,
                    zh,
                ));
            };
            let fye = last_financial_year_end(today, venture.fiscal_year_end_month);
            let incorporated = chrono::DateTime::from_timestamp(incorporated_at, 0)
                .map(|stamp| stamp.date_naive())
                .unwrap_or(today);
            if fye < incorporated {
                return Some(finding(
                    rule,
                    FindingStatus::NotApplicable,
                    t(
                        "No financial year has ended since incorporation.".into(),
                        "成立以来尚未结束一个财政年度。".into(),
                    ),
                    &used,
                    zh,
                ));
            }
            let due = add_months(fye, 3);
            Some(if today <= due {
                finding(rule, FindingStatus::Attention, t(format!("The financial year ended {fye}; ECI is due by {due} unless the waiver applies."), format!("财政年度于 {fye} 结束;除非符合豁免,ECI 须在 {due} 前申报。")), &used, zh)
            } else {
                finding(rule, FindingStatus::NeedsReview, t(format!("The ECI deadline for the year ended {fye} was {due}; confirm it was filed or that the waiver applied."), format!("截至 {fye} 的财政年度的 ECI 期限是 {due};请确认已申报或符合豁免。")), &used, zh)
            })
        }
        CheckKind::CorporateTaxReturn => {
            if facts.document.is_some() {
                return None;
            }
            let used = ["venture.incorporated_at", "today"];
            Some(if venture.incorporated_at.is_none() {
                finding(
                    rule,
                    FindingStatus::Unknown,
                    t(
                        "Incorporation date not recorded.".into(),
                        "未记录成立日期。".into(),
                    ),
                    &used,
                    zh,
                )
            } else if (9..=11).contains(&today.month()) {
                finding(
                    rule,
                    FindingStatus::Attention,
                    t(
                        format!(
                            "The corporate income tax return is due by 30 November {}.",
                            today.year()
                        ),
                        format!(
                            "企业所得税申报表须在 {} 年 11 月 30 日前提交。",
                            today.year()
                        ),
                    ),
                    &used,
                    zh,
                )
            } else {
                finding(
                    rule,
                    FindingStatus::Pass,
                    t(
                        "Outside the filing window; the next return is due by 30 November.".into(),
                        "目前不在申报期内;下一次申报截止 11 月 30 日。".into(),
                    ),
                    &used,
                    zh,
                )
            })
        }
        CheckKind::AnnualGeneralMeetingAndReturn => {
            if facts.document.is_some() {
                return None;
            }
            let used = ["venture.incorporated_at", "venture.fiscal_year_end_month"];
            let Some(incorporated_at) = venture.incorporated_at else {
                return Some(finding(rule, FindingStatus::Unknown, t("Incorporation date not recorded; the AGM and annual-return windows cannot be computed.".into(), "未记录成立日期,无法计算股东大会和年度申报期限。".into()), &used, zh));
            };
            let fye = last_financial_year_end(today, venture.fiscal_year_end_month);
            let incorporated = chrono::DateTime::from_timestamp(incorporated_at, 0)
                .map(|stamp| stamp.date_naive())
                .unwrap_or(today);
            if fye < incorporated {
                return Some(finding(
                    rule,
                    FindingStatus::NotApplicable,
                    t(
                        "No financial year has ended since incorporation.".into(),
                        "成立以来尚未结束一个财政年度。".into(),
                    ),
                    &used,
                    zh,
                ));
            }
            let agm_due = add_months(fye, 6);
            let ar_due = add_months(fye, 7);
            Some(if today <= ar_due {
                finding(rule, FindingStatus::Attention, t(format!("For the year ended {fye}: AGM by {agm_due} (unless exempt), annual return by {ar_due}."), format!("截至 {fye} 的年度:股东大会 {agm_due} 前(可豁免),年度申报 {ar_due} 前。")), &used, zh)
            } else {
                finding(rule, FindingStatus::NeedsReview, t(format!("The annual-return deadline for the year ended {fye} was {ar_due}; confirm it was filed."), format!("截至 {fye} 年度的年度申报期限是 {ar_due};请确认已提交。")), &used, zh)
            })
        }
        CheckKind::PersonalDataConsent => {
            if facts.document.is_some() {
                return None;
            }
            let with_data: Vec<&Customer> = workspace
                .customers
                .iter()
                .filter(|customer| !customer.email.trim().is_empty())
                .collect();
            let used = ["customers.email", "customers.personal_data_consent"];
            if with_data.is_empty() {
                return Some(finding(
                    rule,
                    FindingStatus::NotApplicable,
                    t(
                        "No contact details are stored yet.".into(),
                        "尚未存储联系方式。".into(),
                    ),
                    &used,
                    zh,
                ));
            }
            let without = with_data
                .iter()
                .filter(|customer| !customer.personal_data_consent)
                .count();
            Some(if without == 0 {
                finding(
                    rule,
                    FindingStatus::Pass,
                    t(
                        format!(
                            "Consent is recorded for all {} contact(s) with stored details.",
                            with_data.len()
                        ),
                        format!(
                            "已存储联系方式的 {} 位联系人都记录了同意。",
                            with_data.len()
                        ),
                    ),
                    &used,
                    zh,
                )
            } else {
                finding(rule, FindingStatus::Attention, t(format!("{without} of {} contact(s) with stored details have no recorded consent.", with_data.len()), format!("{} 位已存储联系方式的联系人中,有 {without} 位没有记录同意。", with_data.len())), &used, zh)
            })
        }
        CheckKind::DataProtectionOfficer => {
            if facts.document.is_some() {
                return None;
            }
            Some(finding(rule, FindingStatus::NeedsReview, t("This product does not record a DPO designation; confirm one is designated and its contact published.".into(), "本产品未记录数据保护官的指定;请确认已指定并公布联系方式。".into()), &["(not recorded)"], zh))
        }
        CheckKind::RecordKeeping => {
            if facts.document.is_some() {
                return None;
            }
            Some(finding(rule, FindingStatus::NeedsReview, t("Record keeping cannot be verified here; the signed audit chain and regular exports support it but do not prove it.".into(), "此处无法核实记录保存;签名审计链和定期导出有助于此,但不能证明。".into()), &["ledger", "exports"], zh))
        }
        CheckKind::ContractEssentials => {
            let offers: Vec<&Document> = match facts.document {
                Some(document) if document.kind == DocumentKind::Offer => vec![document],
                Some(_) => return None,
                None => workspace
                    .documents
                    .iter()
                    .filter(|document| {
                        document.kind == DocumentKind::Offer
                            && matches!(
                                document.status,
                                DocumentStatus::ApprovedPendingDelivery | DocumentStatus::Delivered
                            )
                    })
                    .collect(),
            };
            let used = ["document.body"];
            if offers.is_empty() {
                return Some(finding(
                    rule,
                    FindingStatus::NotApplicable,
                    t(
                        "No sent offers to check.".into(),
                        "没有已发送的方案可检查。".into(),
                    ),
                    &used,
                    zh,
                ));
            }
            let sections: [(&str, &[&str]); 7] = [
                ("scope", &["scope", "范围"]),
                ("deliverables", &["deliverable", "交付物", "交付"]),
                ("timeline", &["timeline", "week", "period", "周期", "周"]),
                (
                    "fees",
                    &[
                        "fee",
                        "investment",
                        "price",
                        "amount",
                        "费用",
                        "金额",
                        "报价",
                    ],
                ),
                ("confidentiality", &["confidential", "保密"]),
                (
                    "intellectual property",
                    &[
                        "intellectual property",
                        "ip ",
                        "ownership",
                        "知识产权",
                        "归属",
                    ],
                ),
                ("liability", &["liabilit", "责任"]),
            ];
            let mut problems = Vec::new();
            for offer in offers {
                let lower = offer.body.to_lowercase();
                let missing: Vec<&str> = sections
                    .iter()
                    .filter(|(_, markers)| !markers.iter().any(|marker| lower.contains(marker)))
                    .map(|(name, _)| *name)
                    .collect();
                if !missing.is_empty() {
                    problems.push(format!("\"{}\": {}", offer.title, missing.join(", ")));
                }
            }
            Some(if problems.is_empty() {
                finding(rule, FindingStatus::Pass, t("Every checked offer states scope, deliverables, timeline, fees, confidentiality, IP, and liability.".into(), "检查的方案都写明了范围、交付物、周期、费用、保密、知识产权和责任。".into()), &used, zh)
            } else {
                finding(
                    rule,
                    FindingStatus::Attention,
                    t(
                        format!("Offers missing sections — {}.", problems.join("; ")),
                        format!("方案缺少章节 — {}。", problems.join(";")),
                    ),
                    &used,
                    zh,
                )
            })
        }
        CheckKind::CrossBorderCustomers => {
            if facts.document.is_some() {
                return None;
            }
            let foreign: Vec<&Customer> = workspace
                .customers
                .iter()
                .filter(|customer| {
                    !customer.jurisdiction.is_empty()
                        && customer.jurisdiction != venture.jurisdiction
                })
                .collect();
            let used = ["customers.jurisdiction", "venture.jurisdiction"];
            Some(if foreign.is_empty() {
                finding(
                    rule,
                    FindingStatus::Pass,
                    t(
                        "No customer is recorded outside the company's jurisdiction.".into(),
                        "没有客户记录在公司辖区之外。".into(),
                    ),
                    &used,
                    zh,
                )
            } else {
                let names: Vec<String> = foreign
                    .iter()
                    .map(|customer| format!("{} ({})", customer.name, customer.jurisdiction))
                    .collect();
                finding(
                    rule,
                    FindingStatus::NeedsReview,
                    t(
                        format!(
                            "Customers outside the pack's coverage: {}.",
                            names.join(", ")
                        ),
                        format!("不在规则包覆盖范围内的客户:{}。", names.join("、")),
                    ),
                    &used,
                    zh,
                )
            })
        }
    }
}

fn overall(findings: &[ComplianceFinding]) -> &'static str {
    if findings
        .iter()
        .any(|f| f.status == FindingStatus::Attention)
    {
        "attention"
    } else if findings
        .iter()
        .any(|f| f.status == FindingStatus::NeedsReview)
    {
        "needs_review"
    } else if findings.iter().any(|f| f.status == FindingStatus::Unknown) {
        "unknown"
    } else {
        "clear_within_pack"
    }
}

#[derive(Deserialize)]
struct SummaryOut {
    summary: String,
}

/// Keep a model summary only when every rule id it names was retrieved or
/// checked; a summary citing a rule the founder cannot open is dropped.
fn validate_summary(text: &str, allowed_ids: &[String]) -> Result<String, Rejection> {
    let start = text.find('{').ok_or(Rejection::NotJson)?;
    let end = text.rfind('}').ok_or(Rejection::NotJson)?;
    if end <= start {
        return Err(Rejection::NotJson);
    }
    let out: SummaryOut =
        serde_json::from_str(&text[start..=end]).map_err(|error| match error.classify() {
            serde_json::error::Category::Eof => Rejection::Truncated,
            serde_json::error::Category::Data => Rejection::WrongShape,
            _ => Rejection::NotJson,
        })?;
    let summary = out.summary.trim();
    if summary.is_empty()
        || summary.chars().count() > 2_000
        || summary.chars().any(|ch| ch.is_control() && ch != '\n')
    {
        return Err(Rejection::Field("summary"));
    }
    // The sharp one: a summary may only cite rules the retrieval actually
    // returned. A model that invents a rule id is not making a formatting
    // mistake — it is fabricating a citation in a compliance document, and
    // the founder should be told that is what happened.
    let cited = summary
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-'))
        .filter(|token| token.starts_with("SG-"))
        .map(str::to_owned);
    for id in cited {
        if !allowed_ids.contains(&id) {
            return Err(Rejection::Field("citation"));
        }
    }
    Ok(summary.to_owned())
}

impl Store {
    /// Run the pack against the company (or one document), store the report,
    /// and record the check on the audit chain. Founder-initiated; the
    /// compliance-checker employee wraps the same call in a decision.
    pub fn run_compliance_check(
        &self,
        subject: ComplianceSubject,
        lang: &str,
    ) -> Result<ComplianceReport, WorkspaceError> {
        let zh = lang.starts_with("zh");
        let workspace = self.load()?;
        let venture = workspace
            .venture
            .as_ref()
            .ok_or_else(|| invalid("set up the company profile first"))?;
        let document = match subject.document_id {
            Some(id) => Some(workspace.document(id)?),
            None => None,
        };
        let resource = document
            .map(|document| format!("document:{}", document.id))
            .unwrap_or_else(|| "venture:profile".to_owned());
        let (allowed, _, reason) = self.check_policy(
            "compliance",
            "check",
            &resource,
            DataClass::Amber,
            AutomationLevel::L1Draft,
        );
        if !allowed {
            return Err(WorkspaceError::PolicyDenied(reason));
        }
        let now_unix = now();
        let subject_label = document
            .map(|document| format!("document:{}", document.id))
            .unwrap_or_else(|| "venture".to_owned());

        let facts_digest = {
            let snapshot = serde_json::json!({
                "venture": venture,
                "customers": workspace.customers.iter().map(|c| (c.id, &c.jurisdiction, c.personal_data_consent, !c.email.is_empty())).collect::<Vec<_>>(),
                "document": document.map(|d| (d.id, d.revision, &d.body, d.amount_cents)),
                "receipts": trailing_receipts_cents(&workspace, now_unix),
            });
            sovereign_audit_ledger::hash_bytes(&serde_json::to_vec(&snapshot).unwrap_or_default())
        };

        let pack = if venture.jurisdiction.is_empty() {
            None
        } else {
            pack_for(&venture.jurisdiction)
        };
        let (coverage, findings, pack_id, pack_version, review_status) = match &pack {
            Some(pack) => {
                let facts = Facts {
                    workspace: &workspace,
                    venture,
                    now_unix,
                    document,
                };
                let findings: Vec<ComplianceFinding> = pack
                    .rules
                    .iter()
                    .filter_map(|rule| check_rule(rule, &facts, zh))
                    .collect();
                (
                    "covered",
                    findings,
                    pack.id.to_owned(),
                    pack.version.to_owned(),
                    pack.review_status.to_owned(),
                )
            }
            None if venture.jurisdiction.is_empty() => {
                let rule = &packs()[0].rules[0];
                let facts = Facts {
                    workspace: &workspace,
                    venture,
                    now_unix,
                    document,
                };
                let findings = check_rule(rule, &facts, zh).into_iter().collect();
                (
                    "unknown_jurisdiction",
                    findings,
                    String::new(),
                    String::new(),
                    String::new(),
                )
            }
            None => (
                "not_covered",
                vec![ComplianceFinding {
                    rule_id: "COVERAGE".into(),
                    category: "coverage".into(),
                    basis: "demo_rule".into(),
                    title: if zh {
                        "该辖区没有规则包".into()
                    } else {
                        "No rule pack for this jurisdiction".into()
                    },
                    status: FindingStatus::Unknown,
                    detail: if zh {
                        format!(
                            "本产品目前只有新加坡的演示规则包;{} 未覆盖,不能得出任何结论。",
                            venture.jurisdiction
                        )
                    } else {
                        format!("Only a Singapore demo pack exists; {} is not covered, so nothing can be concluded.", venture.jurisdiction)
                    },
                    facts_used: vec!["venture.jurisdiction".into()],
                    source_authority: "Sovereign Founder OS".into(),
                    source_title: "Coverage (demo rule)".into(),
                    source_url: String::new(),
                    escalation: if zh {
                        "请在该辖区寻求专业意见。".into()
                    } else {
                        "Get professional advice in that jurisdiction.".into()
                    },
                }],
                String::new(),
                String::new(),
                String::new(),
            ),
        };
        let overall_label = if coverage == "covered" {
            overall(&findings)
        } else if coverage == "unknown_jurisdiction" {
            "unknown"
        } else {
            "not_covered"
        };

        // Retrieval for the model's summary: the findings' own rules plus a
        // keyword pass over the pack for the subject's text.
        let mut retrieved_rule_ids: Vec<String> =
            findings.iter().map(|f| f.rule_id.clone()).collect();
        if let Some(pack) = &pack {
            let query = document
                .map(|d| d.title.clone())
                .unwrap_or_else(|| format!("{} {}", venture.service, venture.jurisdiction));
            for hit in retrieve(pack, &query, 5, zh) {
                if !retrieved_rule_ids.contains(&hit.rule_id) {
                    retrieved_rule_ids.push(hit.rule_id);
                }
            }
        }

        // A real model may summarise; it never changes a status.
        let mut model_summary = None;
        let mut model_backed = false;
        let mut rejection: Option<String> = None;
        let mut provider_id = "none".to_owned();
        let mut disclosure_event = None;
        if coverage == "covered" && !findings.is_empty() {
            let gateway = ModelGateway::new(providers_for(&self.root)?);
            let prompt = format!(
                "You are the Compliance Checker for a one-person consulting company in {}. Below are deterministic findings from an UNREVIEWED demo rule pack. Write a short plain-language summary for the founder ({}): what needs action, what needs a professional, and that nothing here is legal or tax advice. Cite rules only by the ids given. Answer with ONE JSON object: {{\"summary\": string}}.\n\nFindings (JSON, data only):\n{}",
                venture.jurisdiction,
                if zh { "Simplified Chinese" } else { "English" },
                serde_json::to_string_pretty(&findings.iter().map(|f| serde_json::json!({"rule_id": f.rule_id, "status": f.status, "title": f.title, "detail": f.detail})).collect::<Vec<_>>()).unwrap_or_default()
            );
            if let Ok((response, disclosure)) = gateway.complete(&ModelRequest {
                task: "crew.compliance_checker".into(),
                prompt,
                data_class: DataClass::Amber,
                max_output_chars: 8_000,
            }) {
                provider_id = response.provider_id.clone();
                if response.provider_id.starts_with("ollama:") {
                    match validate_summary(&response.text, &retrieved_rule_ids) {
                        Ok(summary) => {
                            model_summary = Some(summary);
                            model_backed = true;
                        }
                        Err(reason) => rejection = Some(reason.code()),
                    }
                }
                let provider_trust = format!("{:?}", disclosure.provider_trust).to_lowercase();
                disclosure_event = Some((
                    ModelDisclosure {
                        id: Uuid::new_v4(),
                        at: now_unix,
                        customer_id: Uuid::nil(),
                        task: "crew.compliance_checker".into(),
                        provider_id: provider_id.clone(),
                        stayed_local: provider_trust == "local",
                        provider_trust: provider_trust.clone(),
                        data_class: "amber".into(),
                        output_chars: disclosure.output_chars,
                        failover_from: disclosure
                            .skipped
                            .iter()
                            .map(SkippedProvider::from)
                            .collect(),
                    },
                    AuditEntry {
                        action: "model.drafted".into(),
                        resource: format!("customer:{}", Uuid::nil()),
                        payload: serde_json::json!({
                            "task": "crew.compliance_checker",
                            "provider": provider_id,
                            "provider_trust": provider_trust,
                            "data_class": "amber",
                            "output_chars": disclosure.output_chars,
                        }),
                    },
                ));
            }
        }

        let report = ComplianceReport {
            id: Uuid::new_v4(),
            at: now_unix,
            jurisdiction: venture.jurisdiction.clone(),
            pack_id,
            pack_version,
            review_status,
            subject: subject_label,
            coverage: coverage.to_owned(),
            overall: overall_label.to_owned(),
            findings,
            retrieved_rule_ids,
            model_summary,
            model_backed,
            rejection,
            provider_id,
            facts_digest,
        };

        let mut persisted = self.load()?;
        if persisted.compliance_reports.len() >= MAX_COMPLIANCE_REPORTS {
            persisted.compliance_reports.remove(0);
        }
        let mut events = Vec::new();
        if let Some((disclosure, event)) = disclosure_event {
            persisted.disclosures.push(disclosure);
            events.push(event);
        }
        events.push(AuditEntry {
            action: "compliance.checked".into(),
            resource,
            payload: serde_json::json!({
                "report_id": report.id,
                "pack": report.pack_id,
                "pack_version": report.pack_version,
                "coverage": report.coverage,
                "overall": report.overall,
                "attention": report.findings.iter().filter(|f| f.status == FindingStatus::Attention).count(),
                "needs_review": report.findings.iter().filter(|f| f.status == FindingStatus::NeedsReview).count(),
                "facts_digest": report.facts_digest,
            }),
        });
        persisted.compliance_reports.push(report.clone());
        self.commit(&persisted, events)?;
        Ok(report)
    }

    /// Keyword search over the packs, for the Compliance page.
    pub fn search_rules(&self, query: &str, lang: &str) -> Vec<RuleHit> {
        let zh = lang.starts_with("zh");
        packs()
            .iter()
            .flat_map(|pack| retrieve(pack, query, 8, zh))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_split_ascii_words_and_cjk_bigrams() {
        assert_eq!(
            tokens("GST registration, threshold!"),
            vec!["gst", "registration", "threshold"]
        );
        assert_eq!(tokens("消费税注册"), vec!["消费", "费税", "税注", "注册"]);
        assert!(tokens("a").is_empty());
    }

    #[test]
    fn retrieval_ranks_rarer_terms_higher_and_returns_nothing_for_empty_queries() {
        let pack = singapore_pack();
        let hits = retrieve(&pack, "GST registration threshold", 3, false);
        assert_eq!(hits[0].rule_id, "SG-GST-01");
        assert!(hits[0].matched.contains(&"threshold".to_owned()));
        assert!(retrieve(&pack, "", 3, false).is_empty());
        let zh_hits = retrieve(&pack, "个人资料 同意", 3, true);
        assert_eq!(zh_hits[0].rule_id, "SG-PDPA-01");
        assert!(retrieve(&pack, "quantum blockchain", 3, false).is_empty());
    }

    #[test]
    fn financial_year_math_is_calendar_correct() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 10).unwrap();
        assert_eq!(
            last_financial_year_end(today, 12),
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()
        );
        assert_eq!(
            last_financial_year_end(today, 6),
            NaiveDate::from_ymd_opt(2026, 6, 30).unwrap()
        );
        assert_eq!(
            add_months(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(), 3),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap()
        );
        assert_eq!(
            add_months(NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(), 1),
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
        );
    }

    #[test]
    fn model_summaries_may_only_cite_retrieved_rules() {
        let allowed = vec!["SG-GST-01".to_owned()];
        assert!(
            validate_summary(r#"{"summary":"Register for GST (SG-GST-01)."}"#, &allowed).is_ok()
        );
        // A fabricated citation is reported as one. In a compliance document
        // that is not a formatting slip, and the founder is told which it was.
        assert_eq!(
            validate_summary(r#"{"summary":"See SG-FAKE-99."}"#, &allowed).unwrap_err(),
            Rejection::Field("citation")
        );
        assert_eq!(
            validate_summary("no json", &allowed).unwrap_err(),
            Rejection::NotJson
        );
        assert_eq!(
            validate_summary(r#"{"summary":""}"#, &allowed).unwrap_err(),
            Rejection::Field("summary")
        );
    }

    fn singapore_pack() -> RulePack {
        super::super::compliance_pack::singapore_demo_pack()
    }
}
