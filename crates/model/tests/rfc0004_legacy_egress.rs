//! RFC 0004 v0.2 privacy slice 1: the legacy Amber/Green public-egress
//! bypass is closed on the supported Rust API.
//!
//! These compile as a separate crate, so they cannot name
//! `LocalVouch`'s constructor. A public adapter implementing
//! [`ModelProvider`] through the supported API cannot self-authorize
//! raw Protected input.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use sovereign_contracts::DataClass;
use sovereign_model::{
    DeterministicProvider, Health, ModelError, ModelGateway, ModelProvider, ModelRequest,
    ProviderError, ProviderTrust, SkipCause,
};

/// A public adapter: it implements the trait, can claim any `trust()`, and
/// records whether the gateway ever handed it a prompt. It cannot produce a
/// `LocalVouch`.
struct CountingPublicAdapter {
    id: &'static str,
    trust: ProviderTrust,
    observations: Arc<AtomicUsize>,
}

impl ModelProvider for CountingPublicAdapter {
    fn id(&self) -> &str {
        self.id
    }

    fn trust(&self) -> ProviderTrust {
        self.trust
    }

    fn health(&self) -> Health {
        Health::Healthy
    }

    fn complete(&self, request: &ModelRequest) -> Result<String, ProviderError> {
        self.observations.fetch_add(1, Ordering::SeqCst);
        Ok(request.prompt.clone())
    }
}

fn prompt(data_class: DataClass) -> ModelRequest {
    ModelRequest {
        task: "draft_outreach".into(),
        prompt: "SSN 123-45-6789 and full patient record for Dr. Tan".into(),
        data_class,
        max_output_chars: 4096,
    }
}

fn every_caller_class() -> [DataClass; 3] {
    [DataClass::Green, DataClass::Amber, DataClass::Red]
}

#[test]
fn mislabeled_protected_content_cannot_reach_a_public_adapter() {
    // A caller can put protected content in the prompt and label it Green or
    // Amber. That label must not authorize a public adapter.
    let observations = Arc::new(AtomicUsize::new(0));
    let gateway = ModelGateway::new(vec![Box::new(CountingPublicAdapter {
        id: "public-cloud",
        trust: ProviderTrust::Cloud,
        observations: observations.clone(),
    })]);

    for class in every_caller_class() {
        assert_eq!(
            gateway.complete(&prompt(class)),
            Err(ModelError::AllProvidersFailed),
            "{class:?} must not grant public egress"
        );
    }
    assert_eq!(
        observations.load(Ordering::SeqCst),
        0,
        "a public adapter must never observe a raw prompt"
    );
}

#[test]
fn self_reported_local_flag_alone_cannot_authorize_raw_protected_input() {
    // RFC 0004 rejected alternative: "Trust a provider's local flag". An
    // adapter that reports Local without a crate-constructed vouch is still
    // an untrusted consumer of raw Protected input.
    let observations = Arc::new(AtomicUsize::new(0));
    let adapter = CountingPublicAdapter {
        id: "i-swear-i-am-local",
        trust: ProviderTrust::Local,
        observations: observations.clone(),
    };
    assert!(
        adapter.local_vouch().is_none(),
        "the supported API cannot mint a LocalVouch"
    );
    let gateway = ModelGateway::new(vec![Box::new(adapter)]);

    for class in every_caller_class() {
        assert_eq!(
            gateway.complete(&prompt(class)),
            Err(ModelError::AllProvidersFailed)
        );
    }
    assert_eq!(observations.load(Ordering::SeqCst), 0);
}

#[test]
fn local_only_workflow_produces_zero_public_adapter_observations() {
    // A fully local workflow: one vouched deterministic stand-in, plus two
    // public adapters that would exfiltrate if consulted. The local path
    // must serve the request; the public adapters must observe nothing.
    let cloud_hits = Arc::new(AtomicUsize::new(0));
    let fake_local_hits = Arc::new(AtomicUsize::new(0));
    let gateway = ModelGateway::new(vec![
        Box::new(CountingPublicAdapter {
            id: "public-cloud",
            trust: ProviderTrust::Cloud,
            observations: cloud_hits.clone(),
        }),
        Box::new(CountingPublicAdapter {
            id: "self-reported-local",
            trust: ProviderTrust::Local,
            observations: fake_local_hits.clone(),
        }),
        Box::new(DeterministicProvider::local(
            "local-drafter",
            Health::Healthy,
        )),
    ]);

    for class in every_caller_class() {
        let (response, disclosure) = gateway.complete(&prompt(class)).unwrap();
        assert_eq!(response.provider_id, "local-drafter");
        assert_eq!(response.provider_trust, ProviderTrust::Local);
        assert_eq!(
            disclosure.skipped[0].reason,
            SkipCause::RawRequestIsLocalOnly
        );
        assert_eq!(
            disclosure.skipped[1].reason,
            SkipCause::RawRequestIsLocalOnly
        );
    }
    assert_eq!(cloud_hits.load(Ordering::SeqCst), 0);
    assert_eq!(fake_local_hits.load(Ordering::SeqCst), 0);
}
