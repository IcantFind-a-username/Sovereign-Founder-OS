//! The deterministic drafts the product writes when no real model answers or
//! a model's answer fails validation.
//!
//! Not a fake and not a stand-in for a model: a bounded draft assembled from
//! the same stored facts the employee was allowed to read, labelled as a
//! template everywhere it is shown. Insufficient facts produce an honest
//! "needs input" proposal, never invented content.
//!
//! Split from `crew_roles` when that file reached the god-file limit. The
//! dependency runs one way — this module reads the role's input, the role
//! module knows nothing about these templates — which is why the cut is here.

use super::crew_roles::{RoleInput, MAX_LIST_ITEMS};
use super::crew_types::*;
use super::*;

fn invalid(message: &str) -> WorkspaceError {
    WorkspaceError::Invalid(message.to_owned())
}

// ---------------------------------------------------------------------------
// Deterministic templates: bounded drafts from the same facts, used when no
// real model answers or its output fails validation. Labelled as such.
// ---------------------------------------------------------------------------

/// The deterministic proposal for this input, plus the founder-facing
/// summary. Insufficient facts produce an honest "needs input" proposal,
/// never invented content.
pub(super) fn deterministic_change(
    input: &RoleInput,
) -> Result<(ProposedChange, String), WorkspaceError> {
    let zh = input.lang == "zh";
    match input.role {
        RoleId::Analyst => {
            let customer = input
                .customer
                .as_ref()
                .ok_or_else(|| invalid("choose a customer"))?;
            let text = format!("{}\n{}", customer.discovery_notes, customer.notes);
            let sentences: Vec<String> = split_sentences(&text);
            let mut problems = Vec::new();
            let mut constraints = Vec::new();
            let mut budget_lines = Vec::new();
            for sentence in sentences.iter().take(MAX_LIST_ITEMS * 2) {
                let lower = sentence.to_lowercase();
                if lower.contains("budget")
                    || lower.contains("预算")
                    || lower.contains("sgd")
                    || lower.contains("usd")
                    || lower.contains('$')
                {
                    budget_lines.push(sentence.clone());
                } else if [
                    "must",
                    "need",
                    "approv",
                    "deadline",
                    "by ",
                    "constraint",
                    "cannot",
                    "only",
                    "必须",
                    "需要",
                    "审批",
                    "截止",
                    "限制",
                    "不能",
                ]
                .iter()
                .any(|marker| lower.contains(marker))
                {
                    constraints.push(sentence.clone());
                } else {
                    problems.push(sentence.clone());
                }
            }
            problems.truncate(MAX_LIST_ITEMS);
            constraints.truncate(MAX_LIST_ITEMS);
            let budget = if budget_lines.is_empty() {
                if zh {
                    "未说明".to_owned()
                } else {
                    "not stated".to_owned()
                }
            } else {
                budget_lines.join(" ")
            };
            let mut open_questions = if zh {
                vec![
                    "谁最终批准预算和签约?".to_owned(),
                    "90 天内怎样算成功?".to_owned(),
                    "涉及哪些系统、数据和相关人员?".to_owned(),
                ]
            } else {
                vec![
                    "Who signs off the budget and the engagement?".to_owned(),
                    "What does success look like in 90 days?".to_owned(),
                    "Which systems, data, and people are involved?".to_owned(),
                ]
            };
            if problems.is_empty() {
                open_questions.insert(
                    0,
                    if zh {
                        "先记录客户的需求笔记,再让分析员整理。".to_owned()
                    } else {
                        "Capture discovery notes first; there is nothing to analyse yet.".to_owned()
                    },
                );
            }
            let assumptions = vec![if zh {
                "以上问题和约束来自笔记原文的机械拆分,尚未经客户确认。".to_owned()
            } else {
                "Problems and constraints are split mechanically from your notes and are not customer-confirmed.".to_owned()
            }];
            let summary = if zh {
                format!(
                    "从 {} 的笔记里整理出 {} 个问题、{} 条约束,预算:{}。这是模板整理,不是模型分析。",
                    customer.name, problems.len(), constraints.len(), budget
                )
            } else {
                format!(
                    "Sorted {}'s notes into {} problem(s) and {} constraint(s); budget: {}. Template analysis, not a model's.",
                    customer.name, problems.len(), constraints.len(), budget
                )
            };
            Ok((
                ProposedChange::DiscoverySummary {
                    customer_id: customer.id,
                    problems,
                    constraints,
                    budget,
                    open_questions,
                    assumptions,
                },
                summary,
            ))
        }
        RoleId::ProposalWriter => {
            let customer = input
                .customer
                .as_ref()
                .ok_or_else(|| invalid("choose a customer"))?;
            let context = if customer.discovery_notes.trim().is_empty() {
                customer.notes.clone()
            } else {
                customer.discovery_notes.clone()
            };
            let amount_cents = budget_midpoint_cents(&context);
            let (title, body) = if zh {
                (
                    format!("方案 — {} 为 {} 提供{}", input.venture_name, customer.name, input.service),
                    format!(
                        "方案(草稿,由 AI 员工“报价助理”起草,发送前请审阅)\n\n背景\n{}\n\n范围\n- 评估现状并明确目标\n- 交付:{}\n- 交付物与验收标准将在启动会确认\n\n交付物\n- 现状评估与改进建议\n- 实施计划与交接材料\n\n周期\n约 4 周,自启动会起算。\n\n费用\n{}\n\n假设\n- 客户在 5 个工作日内提供所需资料\n- 范围以本方案为准,变更另议\n\n下一步\n30 分钟的范围确认会。",
                        if context.trim().is_empty() { "(尚无需求记录)" } else { context.trim() },
                        input.service,
                        amount_line(amount_cents, &input.currency, zh)
                    ),
                )
            } else {
                (
                    format!("Proposal — {} for {}", input.service, customer.name),
                    format!(
                        "PROPOSAL (DRAFT — prepared by the AI Proposal Writer; review before sending)\n\nContext\n{}\n\nScope\n- Assess the current situation and agree the goal\n- Deliver: {}\n- Deliverables and acceptance criteria confirmed at kickoff\n\nDeliverables\n- Assessment and improvement recommendations\n- Implementation plan and handover material\n\nTimeline\nAbout 4 weeks from the kickoff call.\n\nInvestment\n{}\n\nAssumptions\n- The customer provides the needed material within 5 working days\n- Scope is as written here; changes are agreed separately\n\nNext step\nA 30-minute scoping call.",
                        if context.trim().is_empty() { "(no discovery notes recorded yet)" } else { context.trim() },
                        input.service,
                        amount_line(amount_cents, &input.currency, zh)
                    ),
                )
            };
            let assumptions = vec![if zh {
                "金额取自笔记中的预算区间中点;没有预算时留空待确认。".to_owned()
            } else {
                "The amount is the midpoint of the budget range found in the notes; left open when no budget was stated.".to_owned()
            }];
            let summary = if zh {
                format!(
                    "为 {} 起草了一份方案草稿({})。模板起草,不是模型输出。",
                    customer.name,
                    amount_line(amount_cents, &input.currency, zh)
                )
            } else {
                format!(
                    "Drafted a proposal for {} ({}). Template draft, not a model's.",
                    customer.name,
                    amount_line(amount_cents, &input.currency, zh)
                )
            };
            Ok((
                ProposedChange::OfferDraft {
                    customer_id: customer.id,
                    title,
                    body,
                    amount_cents,
                    assumptions,
                },
                summary,
            ))
        }
        RoleId::DeliveryPlanner => {
            let customer = input
                .customer
                .as_ref()
                .ok_or_else(|| invalid("choose a customer"))?;
            let offer = input.document.as_ref();
            let project_name = match offer {
                Some(offer) => offer
                    .title
                    .replace("Proposal — ", "")
                    .replace("方案 — ", ""),
                None => format!("{} — {}", input.service, customer.name),
            };
            let plan: Vec<(&str, &str, u32)> = vec![
                ("Kickoff and scoping call", "启动与范围确认会", 3),
                ("Discovery interviews", "访谈与资料收集", 7),
                ("Draft findings and recommendations", "起草评估与建议", 14),
                ("Review draft with the customer", "与客户评审草稿", 18),
                ("Final delivery and handover", "最终交付与交接", 25),
                ("Acceptance sign-off", "验收签字", 28),
            ];
            let tasks = plan
                .iter()
                .map(|(en, zh_title, days)| PlannedTask {
                    title: if zh {
                        (*zh_title).to_owned()
                    } else {
                        (*en).to_owned()
                    },
                    due_in_days: *days,
                })
                .collect();
            let acceptance_criteria = if zh {
                vec![
                    "客户书面确认交付物已收到".to_owned(),
                    "建议清单中的每一项都有负责人和日期".to_owned(),
                    "启动会确认的目标有可核对的结果".to_owned(),
                ]
            } else {
                vec![
                    "The customer confirms receipt of the deliverables in writing".to_owned(),
                    "Every recommendation has an owner and a date".to_owned(),
                    "The goal agreed at kickoff has a checkable result".to_owned(),
                ]
            };
            let summary = if zh {
                format!(
                    "为 {} 规划了 {} 个带日期的任务和 {} 条验收标准。模板计划,不是模型输出。",
                    project_name,
                    plan.len(),
                    acceptance_criteria.len()
                )
            } else {
                format!("Planned {} dated tasks and {} acceptance criteria for {}. Template plan, not a model's.", plan.len(), acceptance_criteria.len(), project_name)
            };
            Ok((
                ProposedChange::DeliveryPlan {
                    customer_id: customer.id,
                    offer_id: offer.map(|offer| offer.id),
                    project_id: input.project.as_ref().map(|project| project.id),
                    project_name,
                    tasks,
                    acceptance_criteria,
                },
                summary,
            ))
        }
        RoleId::InvoiceClerk => {
            let customer = input
                .customer
                .as_ref()
                .ok_or_else(|| invalid("choose a customer"))?;
            let project = input
                .project
                .as_ref()
                .ok_or_else(|| invalid("choose a finished project"))?;
            let amount_cents = project
                .offer_amount_cents
                .or(project.budget_cents)
                .filter(|amount| *amount > 0)
                .ok_or_else(|| {
                    invalid("the project has no offer amount or budget; set one before invoicing")
                })?;
            let gst_line = if input.gst_registered {
                if zh {
                    "本公司已注册 GST;GST 按 IRAS 规则适用,金额未含税计算。"
                } else {
                    "GST-registered: GST applies per IRAS rules; tax is not computed in this draft."
                }
            } else if zh {
                "本公司未注册 GST。"
            } else {
                "Not GST-registered."
            };
            let (title, body) = if zh {
                (
                    format!("发票 — {}", project.name),
                    format!(
                        "发票(草稿,由 AI 员工“账务助理”起草,开具前请审阅)\n\n开票方:{}\n客户:{}\n\n项目:{}\n交付内容:按已接受的方案范围完成交付\n\n金额:{}\n付款期限:30 天\n{}\n",
                        input.venture_name, customer.name, project.name, amount_line(Some(amount_cents), &input.currency, zh), gst_line
                    ),
                )
            } else {
                (
                    format!("Invoice — {}", project.name),
                    format!(
                        "INVOICE (DRAFT — prepared by the AI Invoice Clerk; review before issuing)\n\nFrom: {}\nBill to: {}\n\nProject: {}\nDelivered: the scope of the accepted proposal\n\nAmount: {}\nPayment terms: 30 days\n{}\n",
                        input.venture_name, customer.name, project.name, amount_line(Some(amount_cents), &input.currency, zh), gst_line
                    ),
                )
            };
            let summary = if zh {
                format!(
                    "为 {} 起草发票 {}。模板起草,不是模型输出。",
                    project.name,
                    amount_line(Some(amount_cents), &input.currency, zh)
                )
            } else {
                format!(
                    "Drafted an invoice for {} ({}). Template draft, not a model's.",
                    project.name,
                    amount_line(Some(amount_cents), &input.currency, zh)
                )
            };
            Ok((
                ProposedChange::InvoiceDraft {
                    customer_id: customer.id,
                    project_id: Some(project.id),
                    title,
                    body,
                    amount_cents,
                    due_in_days: 30,
                },
                summary,
            ))
        }
        RoleId::QualityChecker => {
            let document = input
                .document
                .as_ref()
                .ok_or_else(|| invalid("choose a draft"))?;
            let customer = input.customer.as_ref();
            let mut findings = Vec::new();
            let lower = document.body.to_lowercase();
            let finding = |kind: &str, en: &str, zh_text: &str| ReviewFinding {
                kind: kind.to_owned(),
                detail: if zh {
                    zh_text.to_owned()
                } else {
                    en.to_owned()
                },
            };
            if document.kind == DocumentKind::Invoice && document.amount_cents.unwrap_or(0) == 0 {
                findings.push(finding(
                    "gap",
                    "The invoice has no amount.",
                    "发票没有金额。",
                ));
            }
            if document.kind == DocumentKind::Offer
                && !(lower.contains("scope") || lower.contains("范围"))
            {
                findings.push(finding(
                    "gap",
                    "The proposal does not state a scope.",
                    "方案没有写明范围。",
                ));
            }
            if document.kind == DocumentKind::Offer
                && !(lower.contains("timeline")
                    || lower.contains("week")
                    || lower.contains("周期")
                    || lower.contains("周"))
            {
                findings.push(finding(
                    "gap",
                    "The proposal does not state a timeline.",
                    "方案没有写明周期。",
                ));
            }
            if lower.contains("to be confirmed")
                || lower.contains("待确认")
                || lower.contains("tbd")
                || lower.contains("[")
            {
                findings.push(finding("placeholder", "The text still contains placeholders (\"to be confirmed\", \"TBD\", or brackets).", "文本仍有占位符(“待确认”、“TBD”或方括号)。"));
            }
            if document.body.trim().chars().count() < 120 {
                findings.push(finding(
                    "gap",
                    "The body is very short for a document a customer will read.",
                    "正文太短,客户可能看不明白。",
                ));
            }
            if customer.is_some_and(|customer| {
                customer.notes.is_empty() && customer.discovery_notes.is_empty()
            }) {
                findings.push(finding("risk", "There are no discovery notes for this customer, so the draft cannot be checked against what they said.", "这位客户没有需求记录,无法核对草稿是否符合客户所说。"));
            }
            let summary = if zh {
                format!(
                    "检查了“{}”(第 {} 版):{} 条发现。模板检查,不是模型输出。",
                    document.title,
                    document.revision,
                    findings.len()
                )
            } else {
                format!(
                    "Checked \"{}\" (revision {}): {} finding(s). Template check, not a model's.",
                    document.title,
                    document.revision,
                    findings.len()
                )
            };
            Ok((
                ProposedChange::ReviewFindings {
                    document_id: document.id,
                    findings,
                },
                summary,
            ))
        }
        RoleId::ComplianceChecker => Err(invalid(
            "the compliance checker runs from the Compliance page",
        )),
    }
}

/// The founder-facing summary of a model-produced change.
pub(super) fn summarize_change(change: &ProposedChange, zh: bool) -> String {
    match change {
        ProposedChange::DiscoverySummary {
            problems,
            constraints,
            budget,
            ..
        } => {
            if zh {
                format!(
                    "模型整理出 {} 个问题、{} 条约束,预算:{}。",
                    problems.len(),
                    constraints.len(),
                    budget
                )
            } else {
                format!(
                    "The model found {} problem(s) and {} constraint(s); budget: {}.",
                    problems.len(),
                    constraints.len(),
                    budget
                )
            }
        }
        ProposedChange::OfferDraft {
            title,
            amount_cents,
            ..
        } => {
            let amount = amount_cents
                .map(|cents| format!("{}.{:02}", cents / 100, cents % 100))
                .unwrap_or_else(|| {
                    if zh {
                        "金额待定".into()
                    } else {
                        "amount open".into()
                    }
                });
            if zh {
                format!("模型起草了方案“{}”({})。", title, amount)
            } else {
                format!("The model drafted \"{}\" ({}).", title, amount)
            }
        }
        ProposedChange::DeliveryPlan {
            project_name,
            tasks,
            acceptance_criteria,
            ..
        } => {
            if zh {
                format!(
                    "模型为 {} 规划了 {} 个任务和 {} 条验收标准。",
                    project_name,
                    tasks.len(),
                    acceptance_criteria.len()
                )
            } else {
                format!(
                    "The model planned {} tasks and {} acceptance criteria for {}.",
                    tasks.len(),
                    acceptance_criteria.len(),
                    project_name
                )
            }
        }
        ProposedChange::InvoiceDraft {
            title,
            amount_cents,
            due_in_days,
            ..
        } => {
            if zh {
                format!(
                    "模型起草了发票“{}”,金额 {}.{:02},{} 天内到期。",
                    title,
                    amount_cents / 100,
                    amount_cents % 100,
                    due_in_days
                )
            } else {
                format!(
                    "The model drafted \"{}\" for {}.{:02}, due in {} days.",
                    title,
                    amount_cents / 100,
                    amount_cents % 100,
                    due_in_days
                )
            }
        }
        ProposedChange::ReviewFindings { findings, .. } => {
            if zh {
                format!("模型给出 {} 条检查发现。", findings.len())
            } else {
                format!("The model reported {} finding(s).", findings.len())
            }
        }
        ProposedChange::ComplianceReport { .. } => {
            if zh {
                "合规检查报告。".to_owned()
            } else {
                "Compliance report.".to_owned()
            }
        }
    }
}

fn split_sentences(text: &str) -> Vec<String> {
    text.split(['.', '\n', ';', '。', '；', '!', '！', '?', '？'])
        .map(str::trim)
        .filter(|sentence| sentence.chars().count() >= 4)
        .map(|sentence| sentence.to_owned())
        .collect()
}

/// "budget SGD 3,000–5,000" → the midpoint, "around $4,000" → the value, in
/// cents; None when no sentence with a money marker carries a number. Whole
/// currency units only, and year-like numbers are ignored.
fn budget_midpoint_cents(text: &str) -> Option<u64> {
    const MARKERS: [&str; 9] = [
        "$", "sgd", "usd", "eur", "budget", "price", "fee", "预算", "报价",
    ];
    let mut numbers: Vec<u64> = Vec::new();
    for sentence in split_sentences(text) {
        let lower = sentence.to_lowercase();
        if !MARKERS.iter().any(|marker| lower.contains(marker)) {
            continue;
        }
        let mut current = String::new();
        for ch in sentence.chars().chain(std::iter::once(' ')) {
            if ch.is_ascii_digit() {
                current.push(ch);
            } else if ch == ',' && !current.is_empty() {
                continue;
            } else if !current.is_empty() {
                if let Ok(value) = current.parse::<u64>() {
                    let year_like = (1900..=2100).contains(&value);
                    if (100..=100_000_000).contains(&value) && !year_like {
                        numbers.push(value);
                    }
                }
                current.clear();
            }
        }
    }
    match numbers.as_slice() {
        [] => None,
        [one] => Some(one * 100),
        [low, high, ..] => Some((low + high) / 2 * 100),
    }
}

fn amount_line(amount_cents: Option<u64>, currency: &str, zh: bool) -> String {
    match amount_cents {
        Some(cents) => format!("{currency} {}.{:02}", cents / 100, cents % 100),
        None => {
            if zh {
                "金额待确认".to_owned()
            } else {
                "Amount to be confirmed".to_owned()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_midpoints_and_single_values_are_found() {
        assert_eq!(
            budget_midpoint_cents("budget SGD 3,000–5,000"),
            Some(400_000)
        );
        assert_eq!(budget_midpoint_cents("around $4000"), Some(400_000));
        assert_eq!(budget_midpoint_cents("six hours a week"), None);
        assert_eq!(budget_midpoint_cents("in 2026 we need 12 things"), None);
        assert_eq!(
            budget_midpoint_cents("budget approved in 2026 for $4,500"),
            Some(450_000)
        );
        assert_eq!(budget_midpoint_cents("预算三千到五千"), None);
    }

    #[test]
    fn sentences_split_on_english_and_chinese_punctuation() {
        let out = split_sentences("Weekly reporting takes six hours. 预算三千到五千。ok");
        assert_eq!(
            out,
            vec!["Weekly reporting takes six hours", "预算三千到五千"]
        );
    }
}
