//! What the customer receives. The composed `.eml` is the one artefact of
//! this product that leaves the founder's hands, so these tests read it the
//! way a mail client and a customer would: the header block must decode, and
//! the body must state what to pay and by when from the approved record.

use super::compose::{compose_email, format_money};
use super::util::local_date;
use super::*;
use chrono::TimeZone;
use uuid::Uuid;

fn venture(name: &str, currency: &str, uen: &str) -> Venture {
    Venture {
        name: name.into(),
        service: "Order-flow consulting".into(),
        currency: currency.into(),
        uen: uen.into(),
        ..Venture::blank()
    }
}

fn customer(name: &str, email: &str) -> Customer {
    Customer {
        id: Uuid::new_v4(),
        name: name.into(),
        email: email.into(),
        notes: String::new(),
        created_at: 0,
        stage: CustomerStage::Customer,
        discovery_notes: String::new(),
        jurisdiction: String::new(),
        personal_data_consent: true,
        updated_at: 0,
    }
}

fn document(kind: DocumentKind, title: &str, body: &str, amount: Option<u64>) -> Document {
    Document {
        id: Uuid::new_v4(),
        kind,
        customer_id: Uuid::new_v4(),
        title: title.into(),
        body: body.into(),
        amount_cents: amount,
        status: DocumentStatus::PendingApproval,
        created_at: 0,
        updated_at: 0,
        revision: 1,
        due_at: None,
        project_id: None,
        accepted_at: None,
    }
}

fn split(message: &str) -> (&str, &str) {
    message
        .split_once("\r\n\r\n")
        .expect("a header block and a body")
}

/// The unfolded value of one header.
fn header(message: &str, name: &str) -> String {
    let (headers, _) = split(message);
    let unfolded = headers.replace("\r\n ", " ");
    unfolded
        .split("\r\n")
        .find_map(|line| line.strip_prefix(&format!("{name}: ")))
        .unwrap_or_else(|| panic!("no {name} header"))
        .to_owned()
}

/// Decode RFC 2047 `B` words the way a mail client does — rather than
/// comparing against a second encoder, which would share any mistake.
fn decode_words(value: &str) -> String {
    use base64::Engine as _;
    let mut bytes = Vec::new();
    for word in value.split(' ') {
        let payload = word
            .strip_prefix("=?UTF-8?B?")
            .and_then(|rest| rest.strip_suffix("?="))
            .unwrap_or_else(|| panic!("not an encoded-word: {word}"));
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .expect("valid base64");
        // Each word must hold whole characters on its own.
        assert!(
            std::str::from_utf8(&decoded).is_ok(),
            "a split character in {word}"
        );
        bytes.extend(decoded);
    }
    String::from_utf8(bytes).expect("words decode to whole UTF-8 characters")
}

fn local_noon(year: i32, month: u32, day: u32) -> i64 {
    chrono::Local
        .with_ymd_and_hms(year, month, day, 12, 0, 0)
        .single()
        .unwrap()
        .timestamp()
}

/// The defect found by using the product: an invoice drafted by an AI
/// employee has a one-line description for a body, and the amount and due
/// date lived only in fields — so the customer was asked to pay nothing, by
/// no date. The system now states both, from the record, in the document's
/// language.
#[test]
fn an_invoice_states_the_approved_amount_due_date_and_issuer() {
    let company = venture("晓岸咨询", "SGD", "202412345K");
    let buyer = customer("翠林饮品有限公司", "ops@cuilin.example");
    let mut invoice = document(
        DocumentKind::Invoice,
        "订单系统迁移项目",
        "为客户提供数字化转型咨询和流程梳理",
        Some(800_000),
    );
    invoice.due_at = Some(local_noon(2026, 10, 12));

    let message = compose_email(Some(&company), Some(&buyer), &invoice);
    let (_, body) = split(&message);
    let reference = invoice.id.simple().to_string()[..8].to_ascii_uppercase();
    assert!(body.starts_with("为客户提供数字化转型咨询和流程梳理\r\n"));
    for line in [
        "应付金额:SGD 8,000.00".to_owned(),
        "付款期限:2026-10-12".to_owned(),
        "开票方:晓岸咨询(UEN 202412345K)".to_owned(),
        format!("参考编号:{reference}"),
    ] {
        assert!(body.contains(&format!("{line}\r\n")), "missing {line:?}");
    }
    // The subject says what it is: the title names only the project.
    assert_eq!(
        decode_words(&header(&message, "Subject")),
        "发票:订单系统迁移项目"
    );
}

/// The amount line is rendered from the approved field, never copied from
/// prose: a body that still quotes an old figure cannot override it.
#[test]
fn the_stated_amount_follows_the_record_not_the_body() {
    let company = venture("Acme", "USD", "");
    let offer = document(
        DocumentKind::Offer,
        "Website refresh",
        "Scope as discussed. Earlier we said 2,000.",
        Some(250_000),
    );
    let message = compose_email(
        Some(&company),
        Some(&customer("Dr. Tan", "t@x.example")),
        &offer,
    );
    let (_, body) = split(&message);
    assert!(body.contains("\r\n--\r\nQuoted amount: USD 2,500.00\r\nFrom: Acme\r\n"));
    // Offers carry no payment due date; no UEN means none is claimed.
    assert!(!body.contains("Payment due"));
    assert!(!body.contains("UEN"));
    assert_eq!(header(&message, "Subject"), "Offer: Website refresh");
}

#[test]
fn a_document_with_no_amount_or_due_date_gets_no_facts_block() {
    let offer = document(DocumentKind::Offer, "Proposal — Acme", "Hello.\n", None);
    let message = compose_email(Some(&venture("Acme", "SGD", "")), None, &offer);
    let (_, body) = split(&message);
    assert_eq!(body, "Hello.\r\n");
    // "Proposal" already names an offer; no prefix is added.
    assert_eq!(
        decode_words(&header(&message, "Subject")),
        "Proposal — Acme"
    );
}

/// Raw UTF-8 in a header is not RFC 5322. Every header line must be ASCII,
/// every encoded-word within the RFC's length, every line foldable, and the
/// words must decode to exactly the names and title the founder wrote.
#[test]
fn non_ascii_headers_are_encoded_words_that_round_trip() {
    let long_title =
        "晓岸咨询为翠林饮品有限公司提供的订单系统迁移、流程梳理与上线培训服务".repeat(2);
    let company = venture("晓岸咨询", "SGD", "");
    let buyer = customer("翠林饮品有限公司", "ops@cuilin.example");
    let invoice = document(DocumentKind::Invoice, &long_title, "说明", Some(100));
    let message = compose_email(Some(&company), Some(&buyer), &invoice);
    let (headers, _) = split(&message);

    assert!(headers.is_ascii(), "a raw non-ASCII byte in the headers");
    // The body is raw UTF-8, so the message must say it is 8-bit.
    assert!(headers.contains("\r\nContent-Transfer-Encoding: 8bit\r\n"));
    let mut encoded_lines = 0;
    for line in headers.split("\r\n").filter(|line| line.contains("=?")) {
        encoded_lines += 1;
        // RFC 2047: a line holding an encoded-word is at most 76 chars.
        assert!(line.len() <= 76, "header line over 76 chars: {line}");
        for word in line.split(' ').filter(|w| w.starts_with("=?")) {
            assert!(word.len() <= 75, "encoded-word over 75 chars: {word}");
        }
    }
    // The long title really did fold: the check above inspected something.
    assert!(encoded_lines > 4, "only {encoded_lines} encoded lines");
    // A name is one word: a reader that keeps the space between adjacent
    // words would otherwise split the name in two.
    for name_header in ["From", "To"] {
        let value = header(&message, name_header);
        assert_eq!(value.matches("=?").count(), 1, "{name_header}: {value}");
    }
    let from = header(&message, "From");
    let (from_name, from_addr) = from.rsplit_once(' ').unwrap();
    assert_eq!(decode_words(from_name), "晓岸咨询");
    assert_eq!(from_addr, "<founder@example.invalid>");
    let to = header(&message, "To");
    let (to_name, to_addr) = to.rsplit_once(' ').unwrap();
    assert_eq!(decode_words(to_name), "翠林饮品有限公司");
    assert_eq!(to_addr, "<ops@cuilin.example>");
    assert_eq!(
        decode_words(&header(&message, "Subject")),
        format!("发票:{long_title}")
    );
}

#[test]
fn money_uses_the_company_currency_and_groups_digits() {
    assert_eq!(format_money("SGD", 0), "SGD 0.00");
    assert_eq!(format_money("SGD", 99), "SGD 0.99");
    assert_eq!(format_money("SGD", 100_000), "SGD 1,000.00");
    assert_eq!(format_money("MYR", 123_456_789), "MYR 1,234,567.89");
    assert_eq!(format_money("", 100_000), "1,000.00");
}

/// Dates the system writes into the founder's notes and messages are
/// calendar dates where the founder is, not in UTC.
#[test]
fn local_date_is_the_founders_calendar_date() {
    assert_eq!(local_date(local_noon(2026, 9, 12)), "2026-09-12");
    assert_eq!(local_date(local_noon(2026, 1, 1)), "2026-01-01");
}
