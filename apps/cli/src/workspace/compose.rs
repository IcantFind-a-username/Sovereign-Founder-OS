use super::util::{has_chinese, local_date, now};
use super::*;

use uuid::Uuid;

pub(super) fn draft_outreach_note(venture: &Venture, customer: &Customer, zh: bool) -> String {
    // Deterministic local drafting logic — this is the "assistant" content.
    // It composes from known facts only; it invents nothing and cites no
    // numbers, so an unreviewed copy is safe.
    if zh {
        format!(
            "你好 {customer},\n\n我是 {venture} 的负责人。{service}——如果这正是你们现在需要的,我很乐意约个简短的通话,聊聊你们的目标和时间安排。\n\n期待回音。\n\n(本地起草助手草拟 · 未保存 · 请审阅后再使用)",
            customer = customer.name,
            venture = venture.name,
            service = venture.service,
        )
    } else {
        format!(
            "Hi {customer},\n\nI'm the founder of {venture}. {service} — if that's useful to you right now, I'd be glad to set up a short call to understand your goals and timeline.\n\nLooking forward to hearing from you.\n\n(drafted by the local assistant · not saved · review before use)",
            customer = customer.name,
            venture = venture.name,
            service = venture.service,
        )
    }
}

pub(super) fn render_document(
    kind: DocumentKind,
    venture: &Venture,
    customer: &Customer,
    amount_cents: Option<u64>,
    zh: bool,
) -> Document {
    // Deterministic templates by design: no model output enters authoritative
    // business state in this stage.
    let (title, body) = match (kind, zh) {
        (DocumentKind::Offer, false) => (
            format!("Offer — {} for {}", venture.name, customer.name),
            format!(
                "OFFER (DRAFT)\n\nFrom: {}\nTo: {}\n\nProposed service:\n{}\n\nScope, timeline, and pricing to be confirmed together.\nThis draft was generated locally by Sovereign Founder OS; no model was involved and nothing has been sent.",
                venture.name, customer.name, venture.service
            ),
        ),
        (DocumentKind::Offer, true) => (
            format!("报价单 — {} 致 {}", venture.name, customer.name),
            format!(
                "报价单(草稿)\n\n发件方:{}\n客户:{}\n\n拟提供的服务:\n{}\n\n范围、周期与价格待双方确认。\n本草稿由 Sovereign Founder OS 在本地生成;未使用任何模型,也未发送给任何人。",
                venture.name, customer.name, venture.service
            ),
        ),
        (DocumentKind::Invoice, false) => (
            format!("Invoice — {} to {}", venture.name, customer.name),
            format!(
                "INVOICE (DRAFT)\n\nFrom: {}\nBill to: {}\nAmount: {}\n\nPayment terms to be confirmed.\nThis draft was generated locally by Sovereign Founder OS and has not been issued.",
                venture.name,
                customer.name,
                format_money(&venture.currency, amount_cents.unwrap_or(0)),
            ),
        ),
        (DocumentKind::Invoice, true) => (
            format!("发票草稿 — {} 致 {}", venture.name, customer.name),
            format!(
                "发票(草稿)\n\n开票方:{}\n客户:{}\n金额:{}\n\n付款条款待确认。\n本草稿由 Sovereign Founder OS 在本地生成,尚未开具。",
                venture.name,
                customer.name,
                format_money(&venture.currency, amount_cents.unwrap_or(0)),
            ),
        ),
    };
    Document {
        id: Uuid::new_v4(),
        kind,
        customer_id: customer.id,
        title,
        body,
        amount_cents,
        status: DocumentStatus::Draft,
        created_at: now(),
        updated_at: now(),
        revision: 1,
        due_at: None,
        project_id: None,
        accepted_at: None,
    }
}

/// `SGD 8,000.00`: the company's own currency code and grouped digits. The
/// code comes from the profile rather than a symbol, because `$` alone is
/// five different currencies in the markets this product covers.
pub(super) fn format_money(currency: &str, cents: u64) -> String {
    let whole = (cents / 100).to_string();
    let mut grouped = String::with_capacity(whole.len() + whole.len() / 3);
    for (index, digit) in whole.chars().enumerate() {
        if index > 0 && (whole.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    let code = currency.trim();
    if code.is_empty() {
        format!("{grouped}.{:02}", cents % 100)
    } else {
        format!("{code} {grouped}.{:02}", cents % 100)
    }
}

/// Whether a document is written in Chinese. The composed message has no
/// language field to consult, and the right language for the lines the
/// system adds is the one the founder wrote (or approved) the document in.
fn written_in_chinese(document: &Document) -> bool {
    has_chinese(&document.title) || has_chinese(&document.body)
}

/// The facts a customer needs to act on, stated by the system from the
/// approved record rather than left to the body text.
///
/// A drafted body is prose — an AI employee's, or the founder's own edit —
/// and nothing guarantees it repeats the amount, still matches it after the
/// amount field was edited, or says when payment is due. These lines are
/// rendered from the fields the founder saw on the card and approved, so the
/// message cannot ask for a number the record does not hold.
fn outgoing_facts(venture: Option<&Venture>, document: &Document, zh: bool) -> Vec<String> {
    if document.amount_cents.is_none() && document.due_at.is_none() {
        return Vec::new();
    }
    let invoice = document.kind == DocumentKind::Invoice;
    let currency = venture
        .map(|venture| venture.currency.as_str())
        .unwrap_or("");
    let mut lines = Vec::new();
    if let Some(cents) = document.amount_cents {
        let label = match (invoice, zh) {
            (true, true) => "应付金额",
            (true, false) => "Amount due",
            (false, true) => "报价金额",
            (false, false) => "Quoted amount",
        };
        lines.push(format!(
            "{label}{}{}",
            if zh { ":" } else { ": " },
            format_money(currency, cents)
        ));
    }
    if let (true, Some(due_at)) = (invoice, document.due_at) {
        let date = local_date(due_at);
        lines.push(if zh {
            format!("付款期限:{date}")
        } else {
            format!("Payment due: {date}")
        });
    }
    if let Some(venture) = venture {
        let issuer = if venture.uen.trim().is_empty() {
            venture.name.clone()
        } else if zh {
            format!("{}(UEN {})", venture.name, venture.uen.trim())
        } else {
            format!("{} (UEN {})", venture.name, venture.uen.trim())
        };
        lines.push(match (invoice, zh) {
            (true, true) => format!("开票方:{issuer}"),
            (true, false) => format!("Issued by: {issuer}"),
            (false, true) => format!("报价方:{issuer}"),
            (false, false) => format!("From: {issuer}"),
        });
    }
    let reference = document.id.simple().to_string()[..8].to_ascii_uppercase();
    lines.push(if zh {
        format!("参考编号:{reference}")
    } else {
        format!("Reference: {reference}")
    });
    lines
}

/// The subject names the document's kind, so an invoice titled after its
/// project still reads as an invoice in the customer's inbox — unless the
/// title already says so.
fn subject_line(document: &Document, zh: bool) -> String {
    let (label, words): (&str, &[&str]) = match (document.kind, zh) {
        (DocumentKind::Invoice, true) => ("发票", &["发票", "invoice"]),
        (DocumentKind::Invoice, false) => ("Invoice", &["invoice", "发票"]),
        (DocumentKind::Offer, true) => ("报价", &["报价", "方案", "提案", "offer", "proposal"]),
        (DocumentKind::Offer, false) => ("Offer", &["offer", "proposal", "quote", "报价"]),
    };
    let lowered = document.title.to_lowercase();
    if words.iter().any(|word| lowered.contains(word)) {
        document.title.clone()
    } else if zh {
        format!("{label}:{}", document.title)
    } else {
        format!("{label}: {}", document.title)
    }
}

/// The founder's own address is not recorded anywhere yet, so the composed
/// message carries a reserved placeholder; the founder's mail client puts
/// their real address on it when they send.
const SENDER_PLACEHOLDER: &str = "founder@example.invalid";

/// Who a message is from and to, and what it is called, as a person reads
/// them. The composed headers encode exactly these values, and the preview
/// shows exactly these values, so the two cannot drift apart.
struct Envelope {
    sender_name: String,
    recipient_name: String,
    recipient_addr: String,
    subject: String,
    zh: bool,
}

impl Envelope {
    fn of(venture: Option<&Venture>, customer: Option<&Customer>, document: &Document) -> Self {
        let zh = written_in_chinese(document);
        Self {
            sender_name: venture
                .map(|venture| venture.name.clone())
                .unwrap_or_else(|| "Sovereign Founder".to_owned()),
            recipient_name: customer
                .map(|customer| customer.name.clone())
                .unwrap_or_else(|| "Customer".to_owned()),
            recipient_addr: header_safe(
                customer
                    .map(|customer| customer.email.trim())
                    .filter(|email| !email.is_empty())
                    .unwrap_or("recipient@example.invalid"),
            ),
            subject: header_safe(&subject_line(document, zh)),
            zh,
        }
    }
}

/// What the founder reads before approving a send: the message itself, and
/// its envelope decoded for a person.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MessagePreview {
    /// The message as it would be written, CRLF and encoded-words included.
    pub message: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    /// Everything after the headers, with the facts the system adds.
    pub body: String,
    /// The sender address is the reserved placeholder, not the founder's.
    pub from_is_placeholder: bool,
    /// No customer address is recorded; the recipient is a placeholder too.
    pub to_is_placeholder: bool,
}

pub(super) fn preview_email(
    venture: Option<&Venture>,
    customer: Option<&Customer>,
    document: &Document,
) -> MessagePreview {
    let envelope = Envelope::of(venture, customer, document);
    let message = compose_email(venture, customer, document);
    let body = message
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.replace("\r\n", "\n"))
        .unwrap_or_default();
    MessagePreview {
        from: format!("{} <{SENDER_PLACEHOLDER}>", envelope.sender_name),
        to: format!("{} <{}>", envelope.recipient_name, envelope.recipient_addr),
        subject: envelope.subject,
        body,
        from_is_placeholder: true,
        to_is_placeholder: envelope.recipient_addr.ends_with(".invalid"),
        message,
    }
}

/// Compose a well-formed RFC 5322 message for an approved document. The result
/// is written to the local outbox and never transmitted — an `X-Sovereign`
/// header says so, and a missing recipient becomes an RFC 2606 `.invalid`
/// placeholder the founder must replace before sending. Header values come from
/// validated fields (names carry no control characters, emails no CR/LF or
/// separators), and are re-sanitized here, so no field can inject a header.
/// Non-ASCII header text is carried as RFC 2047 encoded-words.
pub(super) fn compose_email(
    venture: Option<&Venture>,
    customer: Option<&Customer>,
    document: &Document,
) -> String {
    let envelope = Envelope::of(venture, customer, document);
    let zh = envelope.zh;
    let placeholder = envelope.recipient_addr.ends_with(".invalid");

    let mut message = String::new();
    message.push_str(&format!(
        "From: {}\r\n",
        mailbox(&envelope.sender_name, SENDER_PLACEHOLDER)
    ));
    message.push_str(&format!(
        "To: {}\r\n",
        mailbox(&envelope.recipient_name, &envelope.recipient_addr)
    ));
    message.push_str(&format!(
        "Subject: {}\r\n",
        encode_unstructured(&envelope.subject)
    ));
    message.push_str(&format!("Date: {}\r\n", chrono::Utc::now().to_rfc2822()));
    message.push_str(&format!(
        "Message-ID: <{}@sovereign-founder-os.invalid>\r\n",
        document.id.simple()
    ));
    message.push_str("MIME-Version: 1.0\r\n");
    message.push_str("Content-Type: text/plain; charset=utf-8\r\n");
    // The body is raw UTF-8; without this line MIME reads it as 7-bit.
    message.push_str("Content-Transfer-Encoding: 8bit\r\n");
    message.push_str(
        "X-Sovereign-Composed: composed locally by Sovereign Founder OS; not transmitted\r\n",
    );
    if placeholder {
        message.push_str(
            "X-Sovereign-Note: recipient address is a placeholder - set the customer's email before sending\r\n",
        );
    }
    message.push_str("\r\n");
    for line in document.body.trim_end().split('\n') {
        message.push_str(line.trim_end_matches('\r'));
        message.push_str("\r\n");
    }
    let facts = outgoing_facts(venture, document, zh);
    if !facts.is_empty() {
        message.push_str("\r\n--\r\n");
        for line in facts {
            message.push_str(&line);
            message.push_str("\r\n");
        }
    }
    message
}

/// Longest encoded-word written here, delimiters included. RFC 2047 caps a
/// word at 75 and any line holding one at 76; 60 leaves room on the first
/// line for the longest header name this module writes (`Subject: `).
const MAX_ENCODED_WORD: usize = 60;

/// Carry non-ASCII header text as RFC 2047 `B` (base64) encoded-words,
/// folded one word per line. Plain ASCII passes through untouched. Each word
/// holds whole characters — the RFC forbids splitting one across words.
///
/// `B` rather than `Q`: a Chinese character costs 4 characters here and 9 in
/// `Q`, so a company or customer name fits in one word. That matters beyond
/// length. The RFC says the space between two adjacent words is ignored, and
/// not every reader honours it — Python's parser, measured, renders a name
/// split across two words with a space in the middle.
fn encode_unstructured(text: &str) -> String {
    use base64::Engine as _;
    if text.is_ascii() {
        return text.to_owned();
    }
    const PREFIX: &str = "=?UTF-8?B?";
    const SUFFIX: &str = "?=";
    // Base64 turns every 3 bytes into 4 characters.
    let byte_budget = (MAX_ENCODED_WORD - PREFIX.len() - SUFFIX.len()) / 4 * 3;
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if current.len() + ch.len_utf8() > byte_budget {
            chunks.push(std::mem::take(&mut current));
        }
        current.push(ch);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
        .iter()
        .map(|chunk| {
            let encoded = base64::engine::general_purpose::STANDARD.encode(chunk.as_bytes());
            format!("{PREFIX}{encoded}{SUFFIX}")
        })
        .collect::<Vec<_>>()
        .join("\r\n ")
}

/// `name <address>`. An encoded name folds before the address, so the line
/// holding the encoded-words stays within RFC 2047's limit however long the
/// address is.
fn mailbox(name: &str, address: &str) -> String {
    let display = encode_display_name(name);
    if !name.is_ascii() {
        format!("{display}\r\n <{address}>")
    } else {
        format!("{display} <{address}>")
    }
}

/// Render an RFC 5322 display-name: kept bare when it is a safe atom, a
/// quoted-string with `\\` and `"` escaped when it is other ASCII, and
/// encoded-words when it is not ASCII (an encoded-word may not sit inside a
/// quoted-string). CR/LF are stripped defensively.
fn encode_display_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .filter(|ch| *ch != '\r' && *ch != '\n')
        .collect();
    if !sanitized.is_ascii() {
        return encode_unstructured(&sanitized);
    }
    let needs_quoting = sanitized.is_empty()
        || sanitized
            .chars()
            .any(|ch| !(ch.is_ascii_alphanumeric() || " !#$%&'*+-/=?^_`{|}~".contains(ch)));
    if needs_quoting {
        let escaped = sanitized.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        sanitized
    }
}

/// Collapse any CR/LF in a single-line header value to spaces — defense in
/// depth against header injection on top of upstream field validation.
fn header_safe(value: &str) -> String {
    value.replace(['\r', '\n'], " ")
}
