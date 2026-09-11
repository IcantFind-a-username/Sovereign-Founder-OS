//! Unit tests for `compliance`'s private helpers.
//!
//! A child module rather than a sibling: these reach `tokens`, `retrieval`
//! and `validate_summary`, which are private to the parent and should stay
//! that way. Module privacy follows the module tree, not the file layout.

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
    assert!(validate_summary(r#"{"summary":"Register for GST (SG-GST-01)."}"#, &allowed).is_ok());
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
