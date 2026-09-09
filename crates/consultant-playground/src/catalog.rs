#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub(crate) struct CatalogEntry {
    key: &'static str,
    en: &'static str,
    zh: &'static str,
}

pub(crate) const CATALOG: [CatalogEntry; 27] = [
    CatalogEntry { key: "page_title", en: "Consultant Playground", zh: "顾问练习场" },
    CatalogEntry { key: "boundary_notice", en: "Practice with this example only. You cannot enter or save your own business or customer data here. Real-data setup is unavailable in this preview.", zh: "请仅使用此示例练习。你无法在此输入或保存自己的业务或客户数据。此预览版尚不支持真实数据设置。" },
    CatalogEntry { key: "company_label", en: "Company", zh: "公司" },
    CatalogEntry { key: "offer_label", en: "Offer", zh: "服务方案" },
    CatalogEntry { key: "relationship_label", en: "Relationship", zh: "客户关系" },
    CatalogEntry { key: "discovery_label", en: "Discovery", zh: "需求分析" },
    CatalogEntry { key: "price_label", en: "Price", zh: "价格" },
    CatalogEntry { key: "organization_label", en: "Organization", zh: "客户公司" },
    CatalogEntry { key: "contact_name_label", en: "Contact", zh: "联系人" },
    CatalogEntry { key: "contact_email_label", en: "Email", zh: "邮箱" },
    CatalogEntry { key: "stage_label", en: "Stage", zh: "阶段" },
    CatalogEntry { key: "problem_label", en: "Problem", zh: "问题" },
    CatalogEntry { key: "budget_label", en: "Budget", zh: "预算" },
    CatalogEntry { key: "constraint_label", en: "Constraint", zh: "限制条件" },
    CatalogEntry { key: "next_step_label", en: "Next step", zh: "下一步" },
    CatalogEntry { key: "reporting_clarity_sprint", en: "Reporting clarity sprint", zh: "报告梳理短期项目" },
    CatalogEntry { key: "weekly_reporting_takes_six_hours", en: "Weekly reporting takes six hours", zh: "每周编制报告需要六小时" },
    CatalogEntry { key: "finance_must_approve", en: "Finance must approve", zh: "需要财务批准" },
    CatalogEntry { key: "thirty_minute_scoping_call", en: "30-minute scoping call", zh: "30分钟需求范围沟通" },
    CatalogEntry { key: "lead", en: "Lead", zh: "潜在客户" },
    CatalogEntry { key: "customer", en: "Customer", zh: "客户" },
    CatalogEntry { key: "action_correct_price", en: "Correct offer price", zh: "修正服务价格" },
    CatalogEntry { key: "action_promote_customer", en: "Mark as customer", zh: "标记为客户" },
    CatalogEntry { key: "action_show_reporting_search", en: "Show reporting search", zh: "查看报告相关检索" },
    CatalogEntry { key: "action_reset", en: "Reset example", zh: "重置示例" },
    CatalogEntry { key: "language_en", en: "English", zh: "英语" },
    CatalogEntry { key: "language_zh", en: "Simplified Chinese", zh: "简体中文" },
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use serde_json::Value;

    use super::CATALOG;

    const EXPECTED: [(&str, &str, &str); 27] = [
        ("page_title", "Consultant Playground", "顾问练习场"),
        ("boundary_notice", "Practice with this example only. You cannot enter or save your own business or customer data here. Real-data setup is unavailable in this preview.", "请仅使用此示例练习。你无法在此输入或保存自己的业务或客户数据。此预览版尚不支持真实数据设置。"),
        ("company_label", "Company", "公司"),
        ("offer_label", "Offer", "服务方案"),
        ("relationship_label", "Relationship", "客户关系"),
        ("discovery_label", "Discovery", "需求分析"),
        ("price_label", "Price", "价格"),
        ("organization_label", "Organization", "客户公司"),
        ("contact_name_label", "Contact", "联系人"),
        ("contact_email_label", "Email", "邮箱"),
        ("stage_label", "Stage", "阶段"),
        ("problem_label", "Problem", "问题"),
        ("budget_label", "Budget", "预算"),
        ("constraint_label", "Constraint", "限制条件"),
        ("next_step_label", "Next step", "下一步"),
        ("reporting_clarity_sprint", "Reporting clarity sprint", "报告梳理短期项目"),
        ("weekly_reporting_takes_six_hours", "Weekly reporting takes six hours", "每周编制报告需要六小时"),
        ("finance_must_approve", "Finance must approve", "需要财务批准"),
        ("thirty_minute_scoping_call", "30-minute scoping call", "30分钟需求范围沟通"),
        ("lead", "Lead", "潜在客户"),
        ("customer", "Customer", "客户"),
        ("action_correct_price", "Correct offer price", "修正服务价格"),
        ("action_promote_customer", "Mark as customer", "标记为客户"),
        ("action_show_reporting_search", "Show reporting search", "查看报告相关检索"),
        ("action_reset", "Reset example", "重置示例"),
        ("language_en", "English", "英语"),
        ("language_zh", "Simplified Chinese", "简体中文"),
    ];

    #[test]
    fn catalog_contains_exact_complete_bilingual_table() {
        assert_eq!(CATALOG.len(), EXPECTED.len());
        let keys: Vec<_> = CATALOG.iter().map(|entry| entry.key).collect();
        assert_eq!(
            keys.iter().copied().collect::<HashSet<_>>().len(),
            keys.len()
        );
        assert!(CATALOG
            .iter()
            .all(|entry| !entry.en.is_empty() && !entry.zh.is_empty()));
        for (entry, expected) in CATALOG.iter().zip(EXPECTED) {
            assert_eq!((entry.key, entry.en, entry.zh), expected);
        }
        assert_eq!(
            CATALOG.iter().filter(|entry| entry.key == "lead").count(),
            1
        );
        assert_eq!(
            CATALOG
                .iter()
                .filter(|entry| entry.key == "customer")
                .count(),
            1
        );
    }

    #[test]
    fn catalog_serializes_exact_browser_shape() {
        let value = serde_json::to_value(CATALOG).expect("catalog serializes");
        let Value::Array(entries) = value else {
            panic!("catalog must serialize as an array")
        };
        assert_eq!(entries.len(), EXPECTED.len());
        for (entry, expected) in entries.iter().zip(EXPECTED) {
            let Value::Object(object) = entry else {
                panic!("catalog entry must be an object")
            };
            assert_eq!(object.len(), 3);
            assert_eq!(
                object.keys().map(String::as_str).collect::<HashSet<_>>(),
                ["key", "en", "zh"].into_iter().collect()
            );
            assert_eq!(
                object.get("key"),
                Some(&Value::String(expected.0.to_owned()))
            );
            assert_eq!(
                object.get("en"),
                Some(&Value::String(expected.1.to_owned()))
            );
            assert_eq!(
                object.get("zh"),
                Some(&Value::String(expected.2.to_owned()))
            );
        }
    }

    #[test]
    fn catalog_has_matching_empty_placeholder_sets() {
        for entry in CATALOG {
            assert!(!entry.en.contains('{'));
            assert!(!entry.en.contains('}'));
            assert!(!entry.zh.contains('{'));
            assert!(!entry.zh.contains('}'));
        }
    }
}
