//! The jurisdiction rule pack. **Demo pack, unreviewed.** Every rule cites
//! the official source it was written from and carries the date it was
//! read; none has been reviewed by a licensed professional, and the pack
//! never claims to. A rule that is a product convention rather than law is
//! labelled `demo_rule` so it can never be mistaken for a statute.
//!
//! Rule packs are data: adding a jurisdiction means adding a pack with its
//! own sources and review status, not changing the checker.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RuleSource {
    pub authority: &'static str,
    pub title: &'static str,
    pub url: &'static str,
    /// When the pack author last read the source.
    pub retrieved_on: &'static str,
}

/// Which deterministic check the rule drives; the check reads stored
/// founder facts only and never infers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    JurisdictionKnown,
    GstRegistrationThreshold,
    GstTaxInvoiceFields,
    EInvoicing,
    InvoiceEssentials,
    EstimatedChargeableIncome,
    CorporateTaxReturn,
    AnnualGeneralMeetingAndReturn,
    PersonalDataConsent,
    DataProtectionOfficer,
    RecordKeeping,
    ContractEssentials,
    CrossBorderCustomers,
}

#[derive(Debug, Clone, Serialize)]
pub struct Rule {
    pub id: &'static str,
    /// `tax`, `corporate`, `data_protection`, `invoicing`, `records`,
    /// `contract`, or `coverage`.
    pub category: &'static str,
    /// `law_or_guidance` when written from an official source; `demo_rule`
    /// when it is this product's own convention.
    pub basis: &'static str,
    pub title_en: &'static str,
    pub title_zh: &'static str,
    pub summary_en: &'static str,
    pub summary_zh: &'static str,
    pub applies_when: &'static str,
    pub check: CheckKind,
    pub escalation_en: &'static str,
    pub escalation_zh: &'static str,
    pub source: RuleSource,
    /// Extra retrieval terms beyond the title and summary.
    pub tags: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize)]
pub struct RulePack {
    pub id: &'static str,
    pub jurisdiction: &'static str,
    pub version: &'static str,
    pub review_status: &'static str,
    pub rules: Vec<Rule>,
}

pub const SG_PACK_ID: &str = "sg-consulting-demo";
pub const SG_PACK_VERSION: &str = "2026.09-demo";
pub const UNREVIEWED: &str = "Unreviewed demo pack: written from the cited official sources, not reviewed by a licensed professional, and not legal or tax advice. Verify every rule against its source before relying on it.";

/// The compulsory GST registration threshold recorded in this pack version
/// (taxable turnover, Singapore dollars, in cents).
pub const SG_GST_THRESHOLD_CENTS: u64 = 100_000_000;

pub fn packs() -> Vec<RulePack> {
    vec![singapore_demo_pack()]
}

pub fn pack_for(jurisdiction: &str) -> Option<RulePack> {
    packs()
        .into_iter()
        .find(|pack| pack.jurisdiction.eq_ignore_ascii_case(jurisdiction))
}

pub fn singapore_demo_pack() -> RulePack {
    let read = "2026-09-10";
    RulePack {
        id: SG_PACK_ID,
        jurisdiction: "SG",
        version: SG_PACK_VERSION,
        review_status: UNREVIEWED,
        rules: vec![
            Rule {
                id: "SG-COV-01",
                category: "coverage",
                basis: "demo_rule",
                title_en: "Jurisdiction must be recorded before anything is checked",
                title_zh: "先记录公司所在辖区,才能检查",
                summary_en: "Checks only run for a jurisdiction the founder entered. Nothing is inferred from language, IP address, or currency.",
                summary_zh: "只对创始人自己填写的辖区运行检查;不会从语言、IP 或币种推断适用法。",
                applies_when: "always",
                check: CheckKind::JurisdictionKnown,
                escalation_en: "Enter the jurisdiction in the company profile.",
                escalation_zh: "在公司资料中填写辖区。",
                source: RuleSource { authority: "Sovereign Founder OS", title: "Product convention (demo rule)", url: "https://github.com/IcantFind-a-username/Sovereign-Founder-OS/blob/main/docs/product/founder-os-execution-blueprint.zh-CN.md", retrieved_on: read },
                tags: &["jurisdiction", "coverage", "辖区"],
            },
            Rule {
                id: "SG-GST-01",
                category: "tax",
                basis: "law_or_guidance",
                title_en: "GST registration when taxable turnover exceeds S$1 million",
                title_zh: "应税营业额超过 100 万新元须注册 GST",
                summary_en: "A business must register for GST when its taxable turnover exceeds S$1 million over the past 12 months, or is expected to in the next 12 months. Voluntary registration is possible below that.",
                summary_zh: "过去 12 个月应税营业额超过 100 万新元,或预计未来 12 个月将超过,必须注册 GST;低于门槛可自愿注册。",
                applies_when: "the company sells taxable goods or services in Singapore",
                check: CheckKind::GstRegistrationThreshold,
                escalation_en: "Confirm the threshold basis with IRAS or a tax agent before registering or claiming exemption.",
                escalation_zh: "注册或主张豁免前,向 IRAS 或税务代理确认门槛的计算口径。",
                source: RuleSource { authority: "IRAS", title: "Do I need to register for GST", url: "https://www.iras.gov.sg/taxes/goods-services-tax-(gst)/gst-registration-deregistration/do-i-need-to-register-for-gst", retrieved_on: read },
                tags: &["gst", "registration", "threshold", "turnover", "消费税", "注册"],
            },
            Rule {
                id: "SG-GST-02",
                category: "invoicing",
                basis: "law_or_guidance",
                title_en: "GST-registered businesses issue tax invoices with the required particulars",
                title_zh: "GST 注册企业须开具载明规定项目的税务发票",
                summary_en: "A GST-registered business must issue a tax invoice for standard-rated supplies to GST-registered customers, showing its GST registration number, the GST amount, and the other required particulars.",
                summary_zh: "GST 注册企业向 GST 注册客户提供标准税率供应时须开具税务发票,载明 GST 注册号、GST 金额及其他规定项目。",
                applies_when: "the company is GST-registered",
                check: CheckKind::GstTaxInvoiceFields,
                escalation_en: "Check the full list of tax-invoice particulars on the IRAS page before issuing.",
                escalation_zh: "开票前对照 IRAS 页面上税务发票的完整项目清单。",
                source: RuleSource { authority: "IRAS", title: "Invoicing, price display and record keeping", url: "https://www.iras.gov.sg/taxes/goods-services-tax-(gst)/basics-of-gst/invoicing-price-display-and-record-keeping", retrieved_on: read },
                tags: &["gst", "tax invoice", "invoice", "发票", "税务发票"],
            },
            Rule {
                id: "SG-GST-03",
                category: "invoicing",
                basis: "law_or_guidance",
                title_en: "InvoiceNow e-invoicing is being phased in for GST-registered businesses",
                title_zh: "InvoiceNow 电子发票正分阶段对 GST 注册企业推行",
                summary_en: "IRAS is phasing in mandatory transmission of invoice data through InvoiceNow for GST-registered businesses, starting with newly incorporated companies that register for GST. Whether and when it applies depends on incorporation and registration dates.",
                summary_zh: "IRAS 正分阶段要求 GST 注册企业通过 InvoiceNow 传输发票数据,先从新注册 GST 的新成立公司开始;是否适用取决于成立和注册日期。",
                applies_when: "the company is GST-registered and was incorporated recently",
                check: CheckKind::EInvoicing,
                escalation_en: "Check the current InvoiceNow timeline on the IRAS page against your incorporation and GST registration dates.",
                escalation_zh: "对照 IRAS 页面上的 InvoiceNow 最新时间表和你的成立、GST 注册日期。",
                source: RuleSource { authority: "IRAS", title: "GST InvoiceNow Requirement", url: "https://www.iras.gov.sg/digital-services/gst-invoicenow-requirement", retrieved_on: read },
                tags: &["invoicenow", "e-invoice", "electronic invoice", "电子发票"],
            },
            Rule {
                id: "SG-INV-01",
                category: "invoicing",
                basis: "demo_rule",
                title_en: "Every invoice names both parties, the amount, and what was delivered",
                title_zh: "每张发票写明双方、金额和交付内容",
                summary_en: "A product convention derived from record-keeping practice: an invoice this product issues should carry the company name, the customer name, an amount, and a description of what was delivered.",
                summary_zh: "来自记账实践的产品约定:本产品开出的发票应包含公司名称、客户名称、金额和交付内容说明。",
                applies_when: "an invoice is drafted or issued",
                check: CheckKind::InvoiceEssentials,
                escalation_en: "Fill in the missing parts before sending the invoice.",
                escalation_zh: "发送前补全缺失部分。",
                source: RuleSource { authority: "Sovereign Founder OS", title: "Product convention (demo rule), informed by IRAS record keeping guidance", url: "https://www.iras.gov.sg/taxes/corporate-income-tax/basics-of-corporate-income-tax/record-keeping-requirements", retrieved_on: read },
                tags: &["invoice", "fields", "发票", "金额"],
            },
            Rule {
                id: "SG-CIT-01",
                category: "tax",
                basis: "law_or_guidance",
                title_en: "Estimated Chargeable Income within 3 months after the financial year end",
                title_zh: "财政年度结束后 3 个月内申报预估应税收入(ECI)",
                summary_en: "Companies file ECI within three months after their financial year end unless an ECI filing waiver applies (for example, small companies with nil ECI).",
                summary_zh: "公司须在财政年度结束后 3 个月内申报 ECI,除非符合豁免条件(例如 ECI 为零的小公司)。",
                applies_when: "the company is incorporated in Singapore and a financial year has ended",
                check: CheckKind::EstimatedChargeableIncome,
                escalation_en: "Confirm whether the waiver applies to you; if not, file ECI on myTax Portal.",
                escalation_zh: "确认是否符合豁免;不符合则在 myTax Portal 申报 ECI。",
                source: RuleSource { authority: "IRAS", title: "Filing Estimated Chargeable Income (ECI)", url: "https://www.iras.gov.sg/taxes/corporate-income-tax/filing-estimated-chargeable-income-(eci)", retrieved_on: read },
                tags: &["eci", "estimated chargeable income", "corporate tax", "企业所得税", "预估"],
            },
            Rule {
                id: "SG-CIT-02",
                category: "tax",
                basis: "law_or_guidance",
                title_en: "Corporate income tax return (Form C-S/C) by 30 November",
                title_zh: "企业所得税申报表(Form C-S/C)须于 11 月 30 日前提交",
                summary_en: "Companies file their corporate income tax return for the preceding year of assessment by 30 November each year.",
                summary_zh: "公司须在每年 11 月 30 日前提交上一课税年的企业所得税申报表。",
                applies_when: "the company is incorporated in Singapore",
                check: CheckKind::CorporateTaxReturn,
                escalation_en: "Engage a tax agent if the company has trade income or complex items.",
                escalation_zh: "有经营收入或复杂事项时,委托税务代理。",
                source: RuleSource { authority: "IRAS", title: "Filing Form C-S/ Form C-S (Lite)/ Form C", url: "https://www.iras.gov.sg/taxes/corporate-income-tax/filing-form-c-s-c", retrieved_on: read },
                tags: &["form c-s", "form c", "tax return", "30 november", "所得税申报"],
            },
            Rule {
                id: "SG-ACRA-01",
                category: "corporate",
                basis: "law_or_guidance",
                title_en: "AGM within 6 months and annual return within 7 months after the financial year end",
                title_zh: "财政年度结束后 6 个月内召开股东大会,7 个月内提交年度申报",
                summary_en: "A private company holds its AGM within six months after its financial year end (unless it qualifies for AGM exemption) and files its annual return with ACRA within seven months after the financial year end.",
                summary_zh: "私人公司须在财政年度结束后 6 个月内召开股东大会(符合条件可豁免),并在 7 个月内向 ACRA 提交年度申报。",
                applies_when: "the company is a Singapore-incorporated private company",
                check: CheckKind::AnnualGeneralMeetingAndReturn,
                escalation_en: "Ask your corporate secretary to confirm the AGM exemption and file the annual return.",
                escalation_zh: "请公司秘书确认股东大会豁免并提交年度申报。",
                source: RuleSource { authority: "ACRA", title: "Annual General Meeting and Annual Return", url: "https://www.acra.gov.sg/how-to-guides/holding-agms-and-filing-annual-returns", retrieved_on: read },
                tags: &["agm", "annual return", "acra", "年度申报", "股东大会"],
            },
            Rule {
                id: "SG-PDPA-01",
                category: "data_protection",
                basis: "law_or_guidance",
                title_en: "Collect personal data with consent and a stated purpose",
                title_zh: "收集个人资料须取得同意并说明目的",
                summary_en: "Under the PDPA, an organisation collecting personal data must obtain consent, notify the purposes, and use the data only for those purposes, subject to the Act's exceptions.",
                summary_zh: "根据 PDPA,机构收集个人资料须取得同意、告知目的,并仅为该目的使用,法定例外除外。",
                applies_when: "the company holds personal data about contacts",
                check: CheckKind::PersonalDataConsent,
                escalation_en: "Record how consent was obtained for each contact and keep a purpose statement.",
                escalation_zh: "记录每位联系人的同意方式并保留目的说明。",
                source: RuleSource { authority: "PDPC", title: "Personal Data Protection Act overview", url: "https://www.pdpc.gov.sg/overview-of-pdpa/the-legislation/personal-data-protection-act", retrieved_on: read },
                tags: &["pdpa", "consent", "personal data", "个人资料", "同意"],
            },
            Rule {
                id: "SG-PDPA-02",
                category: "data_protection",
                basis: "law_or_guidance",
                title_en: "Appoint a data protection officer",
                title_zh: "指定数据保护官",
                summary_en: "Every organisation must designate at least one person as its data protection officer and make their business contact information available.",
                summary_zh: "每个机构须至少指定一人为数据保护官,并公开其业务联系方式。",
                applies_when: "always, for organisations subject to the PDPA",
                check: CheckKind::DataProtectionOfficer,
                escalation_en: "As a one-person company, record yourself as the DPO and register the contact with PDPC where required.",
                escalation_zh: "一人公司可将自己登记为数据保护官,并按要求向 PDPC 登记联系方式。",
                source: RuleSource { authority: "PDPC", title: "Data Protection Officers", url: "https://www.pdpc.gov.sg/overview-of-pdpa/data-protection/business-owner/data-protection-officers", retrieved_on: read },
                tags: &["dpo", "data protection officer", "数据保护官"],
            },
            Rule {
                id: "SG-REC-01",
                category: "records",
                basis: "law_or_guidance",
                title_en: "Keep business records for at least five years",
                title_zh: "商业记录至少保存五年",
                summary_en: "Companies must keep proper records and accounts, including source documents, for at least five years from the relevant year of assessment.",
                summary_zh: "公司须妥善保存账目和记录,包括原始凭证,自相关课税年起至少五年。",
                applies_when: "always",
                check: CheckKind::RecordKeeping,
                escalation_en: "Export your workspace regularly and keep the exports with your accounting records.",
                escalation_zh: "定期导出工作区,并与会计记录一并保存。",
                source: RuleSource { authority: "IRAS", title: "Record keeping requirements", url: "https://www.iras.gov.sg/taxes/corporate-income-tax/basics-of-corporate-income-tax/record-keeping-requirements", retrieved_on: read },
                tags: &["records", "five years", "record keeping", "记录", "保存"],
            },
            Rule {
                id: "SG-CON-01",
                category: "contract",
                basis: "demo_rule",
                title_en: "A consulting proposal states scope, deliverables, timeline, fees, and the confidentiality, IP, and liability positions",
                title_zh: "咨询方案应写明范围、交付物、周期、费用,以及保密、知识产权和责任安排",
                summary_en: "A product convention for the first consulting scenario: before a proposal is sent, it should state what is delivered, by when, for how much, and who owns the work, keeps what confidential, and bears which liability.",
                summary_zh: "首个咨询场景的产品约定:方案发送前应写明交付什么、何时、多少钱,以及成果归属、保密和责任分配。",
                applies_when: "an offer is sent or accepted",
                check: CheckKind::ContractEssentials,
                escalation_en: "Have a lawyer in the customer's jurisdiction review the contract for anything material.",
                escalation_zh: "重大事项请客户所在辖区的律师复核合同。",
                source: RuleSource { authority: "Sovereign Founder OS", title: "Product convention (demo rule)", url: "https://github.com/IcantFind-a-username/Sovereign-Founder-OS/blob/main/docs/product/founder-os-execution-blueprint.zh-CN.md", retrieved_on: read },
                tags: &["contract", "proposal", "scope", "confidentiality", "ip", "liability", "合同", "范围", "保密"],
            },
            Rule {
                id: "SG-COV-02",
                category: "coverage",
                basis: "demo_rule",
                title_en: "Customers outside Singapore are not covered by this pack",
                title_zh: "新加坡以外的客户不在本规则包覆盖范围内",
                summary_en: "This pack covers Singapore only. A customer recorded in another jurisdiction may bring that jurisdiction's tax, data, and contract rules into play; the pack cannot check them.",
                summary_zh: "本包只覆盖新加坡。记录在其他辖区的客户可能引入该辖区的税务、数据和合同规则,本包无法检查。",
                applies_when: "a customer's jurisdiction is set to something other than SG",
                check: CheckKind::CrossBorderCustomers,
                escalation_en: "Get advice in the customer's jurisdiction before signing or invoicing across borders.",
                escalation_zh: "跨境签约或开票前,先取得客户所在辖区的专业意见。",
                source: RuleSource { authority: "Sovereign Founder OS", title: "Product convention (demo rule)", url: "https://github.com/IcantFind-a-username/Sovereign-Founder-OS/blob/main/docs/product/founder-os-execution-blueprint.zh-CN.md", retrieved_on: read },
                tags: &["cross-border", "eu", "us", "foreign customer", "跨境", "海外客户"],
            },
        ],
    }
}
