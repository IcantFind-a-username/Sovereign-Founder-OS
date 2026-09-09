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
pub(crate) struct PlaygroundSession {
    graph: ConsultantPlaygroundGraph,
}

impl PlaygroundSession {
    pub(crate) fn new() -> Self {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlaygroundAction {
    CorrectOfferPrice,
    PromoteAcmeToCustomer,
    ShowReportingSearch,
    Reset,
}

impl PlaygroundSession {
    pub(crate) fn apply(&mut self, action: PlaygroundAction) {
        match action {
            PlaygroundAction::CorrectOfferPrice => {
                self.graph.offer.price_usd_cents = 350_000;
            }
            PlaygroundAction::PromoteAcmeToCustomer => {
                self.graph.relationship.stage = RelationshipStage::Customer;
            }
            PlaygroundAction::ShowReportingSearch => {}
            PlaygroundAction::Reset => {
                *self = Self::new();
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub(crate) struct PlaygroundReadModel {
    profile: &'static str,
    real_data_enabled: bool,
    persistence: &'static str,
    company_name: &'static str,
    offer_name_key: &'static str,
    offer_price_usd_cents: u32,
    relationship_organization: &'static str,
    relationship_contact_name: &'static str,
    relationship_contact_email: &'static str,
    relationship_stage: &'static str,
    discovery_problem_key: &'static str,
    discovery_budget_min_usd_cents: u32,
    discovery_budget_max_usd_cents: u32,
    discovery_constraint_key: &'static str,
    discovery_next_step_key: &'static str,
}

impl SemanticKey {
    fn as_str(self) -> &'static str {
        match self {
            Self::ReportingClaritySprint => "reporting_clarity_sprint",
            Self::WeeklyReportingTakesSixHours => "weekly_reporting_takes_six_hours",
            Self::FinanceMustApprove => "finance_must_approve",
            Self::ThirtyMinuteScopingCall => "thirty_minute_scoping_call",
        }
    }
}

impl PlaygroundSession {
    pub(crate) fn read_model(&self) -> PlaygroundReadModel {
        PlaygroundReadModel {
            profile: "synthetic_playground",
            real_data_enabled: false,
            persistence: "none",
            company_name: self.graph.company.name,
            offer_name_key: self.graph.offer.name_key.as_str(),
            offer_price_usd_cents: self.graph.offer.price_usd_cents,
            relationship_organization: self.graph.relationship.organization,
            relationship_contact_name: self.graph.relationship.contact_name,
            relationship_contact_email: self.graph.relationship.contact_email,
            relationship_stage: match self.graph.relationship.stage {
                RelationshipStage::Lead => "lead",
                RelationshipStage::Customer => "customer",
            },
            discovery_problem_key: self.graph.discovery.problem_key.as_str(),
            discovery_budget_min_usd_cents: self.graph.discovery.budget_min_usd_cents,
            discovery_budget_max_usd_cents: self.graph.discovery.budget_max_usd_cents,
            discovery_constraint_key: self.graph.discovery.constraint_key.as_str(),
            discovery_next_step_key: self.graph.discovery.next_step_key.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PlaygroundAction, PlaygroundSession, RelationshipStage, SemanticKey};

    fn reachable_states() -> [PlaygroundSession; 4] {
        let fresh = PlaygroundSession::new();
        let mut promoted = fresh;
        promoted.apply(PlaygroundAction::PromoteAcmeToCustomer);
        let mut corrected = fresh;
        corrected.apply(PlaygroundAction::CorrectOfferPrice);
        let mut corrected_promoted = corrected;
        corrected_promoted.apply(PlaygroundAction::PromoteAcmeToCustomer);
        [fresh, promoted, corrected, corrected_promoted]
    }

    #[test]
    fn closed_actions_cover_every_reachable_transition() {
        let actions = [
            PlaygroundAction::CorrectOfferPrice,
            PlaygroundAction::PromoteAcmeToCustomer,
            PlaygroundAction::ShowReportingSearch,
            PlaygroundAction::Reset,
        ];
        for state in reachable_states() {
            for action in actions {
                let mut actual = state;
                actual.apply(action);
                let mut expected = state;
                match action {
                    PlaygroundAction::CorrectOfferPrice => {
                        expected.graph.offer.price_usd_cents = 350_000;
                    }
                    PlaygroundAction::PromoteAcmeToCustomer => {
                        expected.graph.relationship.stage = RelationshipStage::Customer;
                    }
                    PlaygroundAction::ShowReportingSearch => {}
                    PlaygroundAction::Reset => expected = PlaygroundSession::new(),
                }
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn correct_price_and_promotion_are_idempotent_and_commute() {
        let mut repeated = PlaygroundSession::new();
        repeated.apply(PlaygroundAction::CorrectOfferPrice);
        repeated.apply(PlaygroundAction::CorrectOfferPrice);
        repeated.apply(PlaygroundAction::PromoteAcmeToCustomer);
        repeated.apply(PlaygroundAction::PromoteAcmeToCustomer);

        let mut price_then_stage = PlaygroundSession::new();
        price_then_stage.apply(PlaygroundAction::CorrectOfferPrice);
        price_then_stage.apply(PlaygroundAction::PromoteAcmeToCustomer);
        let mut stage_then_price = PlaygroundSession::new();
        stage_then_price.apply(PlaygroundAction::PromoteAcmeToCustomer);
        stage_then_price.apply(PlaygroundAction::CorrectOfferPrice);

        assert_eq!(repeated, price_then_stage);
        assert_eq!(price_then_stage, stage_then_price);
        assert_eq!(repeated.graph.offer.price_usd_cents, 350_000);
        assert_eq!(
            repeated.graph.relationship.stage,
            RelationshipStage::Customer
        );
    }

    #[test]
    fn reporting_search_is_read_only_in_every_reachable_state() {
        for state in reachable_states() {
            let snapshot = state;
            let mut actual = state;
            actual.apply(PlaygroundAction::ShowReportingSearch);
            assert_eq!(actual, snapshot);
            actual.apply(PlaygroundAction::ShowReportingSearch);
            assert_eq!(actual, snapshot);
        }
    }

    #[test]
    fn reset_reconstructs_exact_fixture_from_every_reachable_state() {
        for state in reachable_states() {
            let mut actual = state;
            actual.apply(PlaygroundAction::Reset);
            assert_eq!(actual, PlaygroundSession::new());
            actual.apply(PlaygroundAction::Reset);
            assert_eq!(actual, PlaygroundSession::new());
        }
    }

    #[test]
    fn read_model_serializes_exact_fields_in_every_reachable_state() {
        let expected_states = [
            (250_000, "lead"),
            (250_000, "customer"),
            (350_000, "lead"),
            (350_000, "customer"),
        ];
        for (session, (price, stage)) in reachable_states().into_iter().zip(expected_states) {
            let actual = serde_json::to_value(session.read_model()).expect("DTO serializes");
            let expected = serde_json::json!({
                "profile": "synthetic_playground", "real_data_enabled": false, "persistence": "none",
                "company_name": "North Star Operations", "offer_name_key": "reporting_clarity_sprint",
                "offer_price_usd_cents": price, "relationship_organization": "Acme Ltd",
                "relationship_contact_name": "Alex Chen", "relationship_contact_email": "alex.chen@example.test",
                "relationship_stage": stage, "discovery_problem_key": "weekly_reporting_takes_six_hours",
                "discovery_budget_min_usd_cents": 300_000, "discovery_budget_max_usd_cents": 500_000,
                "discovery_constraint_key": "finance_must_approve", "discovery_next_step_key": "thirty_minute_scoping_call"
            });
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn read_model_reads_preserve_session_and_are_deterministic() {
        for session in reachable_states() {
            let before = session;
            let first = serde_json::to_string(&session.read_model()).expect("DTO serializes");
            let second = serde_json::to_string(&session.read_model()).expect("DTO serializes");
            assert_eq!(first, second);
            let mut searched = session;
            let before_value = serde_json::to_value(searched.read_model()).expect("DTO serializes");
            searched.apply(PlaygroundAction::ShowReportingSearch);
            let after_value = serde_json::to_value(searched.read_model()).expect("DTO serializes");
            let after_string =
                serde_json::to_string(&searched.read_model()).expect("DTO serializes");
            assert_eq!(before_value, after_value);
            assert_eq!(first, after_string);
            assert_eq!(searched, before);
        }
    }

    #[test]
    fn read_model_snapshot_is_detached_and_reset_restores_projection() {
        for session in reachable_states() {
            let before = session;
            let snapshot = before.read_model();
            let initial = serde_json::to_value(snapshot).expect("DTO serializes");
            let mut changed = snapshot;
            changed.offer_price_usd_cents = 1;
            let mut modified = session;
            modified.apply(PlaygroundAction::CorrectOfferPrice);
            modified.apply(PlaygroundAction::PromoteAcmeToCustomer);
            assert_eq!(
                serde_json::to_value(snapshot).expect("DTO serializes"),
                initial
            );
            assert_eq!(before, session);
            let mut reset = session;
            reset.apply(PlaygroundAction::Reset);
            assert_eq!(
                serde_json::to_value(reset.read_model()).expect("DTO serializes"),
                serde_json::to_value(PlaygroundSession::new().read_model())
                    .expect("DTO serializes")
            );
            assert_ne!(
                serde_json::to_value(changed).expect("DTO serializes"),
                initial
            );
        }
    }

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
