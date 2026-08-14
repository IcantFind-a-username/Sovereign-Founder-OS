#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SemanticKey {
    ReportingClaritySprint,
    WeeklyReportingTakesSixHours,
    FinanceMustApprove,
    ThirtyMinuteScopingCall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RelationshipStage {
    Lead,
    Customer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Company {
    name: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Offer {
    name_key: SemanticKey,
    price_usd_cents: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Relationship {
    organization: &'static str,
    contact_name: &'static str,
    contact_email: &'static str,
    stage: RelationshipStage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Discovery {
    problem_key: SemanticKey,
    budget_min_usd_cents: u32,
    budget_max_usd_cents: u32,
    constraint_key: SemanticKey,
    next_step_key: SemanticKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ConsultantPlaygroundGraph {
    company: Company,
    offer: Offer,
    relationship: Relationship,
    discovery: Discovery,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PlaygroundSession {
    graph: ConsultantPlaygroundGraph,
}

impl PlaygroundSession {
    fn new() -> Self {
        Self {
            graph: ConsultantPlaygroundGraph {
                company: Company {
                    name: "North Star Operations",
                },
                offer: Offer {
                    name_key: SemanticKey::ReportingClaritySprint,
                    price_usd_cents: 250_000,
                },
                relationship: Relationship {
                    organization: "Acme Ltd",
                    contact_name: "Alex Chen",
                    contact_email: "alex.chen@example.test",
                    stage: RelationshipStage::Lead,
                },
                discovery: Discovery {
                    problem_key: SemanticKey::WeeklyReportingTakesSixHours,
                    budget_min_usd_cents: 300_000,
                    budget_max_usd_cents: 500_000,
                    constraint_key: SemanticKey::FinanceMustApprove,
                    next_step_key: SemanticKey::ThirtyMinuteScopingCall,
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PlaygroundSession, RelationshipStage, SemanticKey};

    #[test]
    fn fixture_matches_exact_synthetic_consultant_thread() {
        let graph = PlaygroundSession::new().graph;

        assert_eq!(graph.company.name, "North Star Operations");
        assert_eq!(graph.offer.name_key, SemanticKey::ReportingClaritySprint);
        assert_eq!(graph.offer.price_usd_cents, 250_000);
        assert_eq!(graph.relationship.organization, "Acme Ltd");
        assert_eq!(graph.relationship.contact_name, "Alex Chen");
        assert_eq!(graph.relationship.contact_email, "alex.chen@example.test");
        assert_eq!(graph.relationship.stage, RelationshipStage::Lead);
        assert_eq!(
            graph.discovery.problem_key,
            SemanticKey::WeeklyReportingTakesSixHours
        );
        assert_eq!(graph.discovery.budget_min_usd_cents, 300_000);
        assert_eq!(graph.discovery.budget_max_usd_cents, 500_000);
        assert_eq!(
            graph.discovery.constraint_key,
            SemanticKey::FinanceMustApprove
        );
        assert_eq!(
            graph.discovery.next_step_key,
            SemanticKey::ThirtyMinuteScopingCall
        );
    }

    #[test]
    fn fresh_sessions_reconstruct_the_same_graph() {
        let first = PlaygroundSession::new();
        let second = PlaygroundSession::new();

        assert_eq!(first, second);
        assert_eq!(first.graph, second.graph);
    }

    #[test]
    fn fixture_invariants_are_closed_and_self_consistent() {
        let graph = PlaygroundSession::new().graph;

        assert!(!graph.company.name.is_empty());
        assert!(!graph.offer.name_key.eq(&graph.discovery.problem_key));
        assert!(graph.offer.price_usd_cents < graph.discovery.budget_min_usd_cents);
        assert!(graph.discovery.budget_min_usd_cents <= graph.discovery.budget_max_usd_cents);
        assert!(graph.relationship.contact_email.ends_with("@example.test"));
        assert_eq!(graph.relationship.stage, RelationshipStage::Lead);
        assert_ne!(graph.relationship.stage, RelationshipStage::Customer);
    }

    #[test]
    fn semantic_keys_are_closed_over_task_one_facts() {
        let keys = [
            SemanticKey::ReportingClaritySprint,
            SemanticKey::WeeklyReportingTakesSixHours,
            SemanticKey::FinanceMustApprove,
            SemanticKey::ThirtyMinuteScopingCall,
        ];

        let ordinals = keys.map(|key| match key {
            SemanticKey::ReportingClaritySprint => 0,
            SemanticKey::WeeklyReportingTakesSixHours => 1,
            SemanticKey::FinanceMustApprove => 2,
            SemanticKey::ThirtyMinuteScopingCall => 3,
        });
        assert_eq!(ordinals, [0, 1, 2, 3]);
    }
}
