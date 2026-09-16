//! Model Gateway: a unified, model-neutral interface with health-aware
//! failover and honest data-disclosure records.
//!
//! Non-negotiables this crate enforces (architecture principles 1, 2, 7):
//!
//! - **Models are replaceable compute components.** The gateway routes one
//!   request through an ordered list of providers; removing or downing the
//!   primary provider does not stop a request as long as any eligible
//!   provider remains. This is the Stage 2 exit criterion.
//! - **Model output is untrusted.** A [`ModelResponse`] is a suggestion. It
//!   carries no authority, holds no keys, and callers must never write it into
//!   authoritative state without independent review. Nothing here touches the
//!   vault, policy, or capability layers.
//! - **Raw requests are local-only.** A prompt reaches only a provider this
//!   crate vouches for ([`LocalVouch`]) that also reports
//!   [`ProviderTrust::Local`]. Caller-supplied Amber/Green/`DataClass` MUST
//!   NOT authorize public or cloud-labelled egress: unknown and legacy Amber
//!   and Green values enter as Protected, and `DataClass` may only narrow a
//!   skip reason. A provider's self-reported `local` flag cannot mint a
//!   vouch. This closes RFC 0004's legacy Amber/Green public-egress bypass
//!   on the supported Rust API. v0.2 privacy slice 2 re-exports the closed
//!   [`PrivacyGateway`] from `sovereign-privacy`; attempt/adapter internals
//!   stay there. It is **not** the rest of RFC 0004: there is no real
//!   local-model sandbox, no ActiveV2, no product Exact Effect / 1C0, and
//!   no Secure Mesh / OwnedMesh executable.
//!
//! ## Honest limits
//!
//! The providers in this crate are **deterministic local stand-ins, not
//! LLMs** and not “cloud-assisted” inference. They exist to prove the
//! routing, health, failover, and disclosure contract without adding a
//! network dependency or a real model. A real public provider would
//! implement the same [`ModelProvider`] trait behind an egress broker that
//! does not exist yet; until it does, a cloud-labelled adapter is skipped
//! before it can observe a raw prompt. "Cost" and "latency" fields are
//! placeholders a real provider would populate.

use std::fmt;

use serde::Serialize;
use sovereign_contracts::DataClass;

mod ollama;

pub use ollama::{OllamaConfigError, OllamaProvider};

/// Closed RFC 0004 privacy workflow. Re-exported so this crate is a single
/// high-level door; broker, attempt, and adapter internals stay inside
/// `sovereign-privacy`. There is no public cross-crate manual attempt API.
pub use sovereign_privacy::{
    activate_owned_mesh, ActivationError, GatewayError, LocalCapability, PrivacyGateway,
    WorkflowOutcome,
};

/// Where a provider runs, for confidentiality routing. Local providers run on
/// the founder's device; cloud providers are untrusted for confidentiality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderTrust {
    Local,
    Cloud,
}

/// A provider's self-reported health. `Down` providers are skipped; `Degraded`
/// providers are used only if no `Healthy` provider is eligible first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Health {
    Healthy,
    Degraded,
    Down,
}

/// One request for model assistance. `data_class` is a compatibility/display
/// field: the caller classifies the prompt, but the label MUST NOT grant
/// public or cloud-labelled egress. The gateway treats unknown and legacy
/// Amber/Green values as Protected and may only use `DataClass` to narrow a
/// skip reason.
#[derive(Debug, Clone)]
pub struct ModelRequest {
    pub task: String,
    pub prompt: String,
    pub data_class: DataClass,
    pub max_output_chars: usize,
}

/// Untrusted model output. It is a suggestion, never authoritative state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelResponse {
    pub text: String,
    pub provider_id: String,
    pub provider_trust: ProviderTrust,
}

/// Non-repudiable record of what a request disclosed to which provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DisclosureRecord {
    pub task: String,
    pub provider_id: String,
    pub provider_trust: ProviderTrust,
    pub data_class: DataClass,
    pub provider_index: usize,
    pub output_chars: usize,
    /// Providers that were skipped before this one, and why — an auditable
    /// trail of the failover path taken.
    pub skipped: Vec<SkipReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkipReason {
    pub provider_id: String,
    pub reason: SkipCause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipCause {
    /// Red data may not be disclosed to a non-local provider. `DataClass` may
    /// only narrow a skip reason this way; it never grants egress.
    RedDataConfidentiality,
    /// The provider reported itself down.
    Unhealthy,
    /// The provider was tried and returned an error.
    Failed,
    /// A raw request may only reach a provider this crate vouches for as
    /// running on this device. Reaching anything else requires a compiled
    /// projection (RFC 0004).
    RawRequestIsLocalOnly,
    /// A cloud-labelled provider never receives a raw request. Caller Amber
    /// or Green cannot grant that egress (RFC 0004 Current gap).
    CloudLabelledDenied,
}

/// Proof that a provider is one this crate itself vouches for as local.
///
/// RFC 0004 forbids a trait method or a caller string from establishing local
/// trust: an adapter that self-reported `local` would be self-authorizing raw
/// access to protected data. So the proof is a value only this module can
/// construct. An implementor outside this crate cannot name the constructor,
/// cannot build one, and therefore cannot claim the vouch however it
/// implements the trait or whatever it calls itself.
///
/// Two kinds qualify today: the core-reviewed deterministic stand-ins built
/// into this crate, and the Ollama adapter, which refuses any non-loopback
/// base URL at construction. An ordinary process does not become trusted by
/// listening on localhost.
#[derive(Debug, Clone, Copy)]
pub struct LocalVouch(PhantomNotConstructible);

#[derive(Debug, Clone, Copy)]
struct PhantomNotConstructible;

impl LocalVouch {
    /// Callable only from inside this crate.
    pub(crate) fn core_reviewed() -> Self {
        LocalVouch(PhantomNotConstructible)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ModelError {
    #[error("no eligible provider could serve the request")]
    AllProvidersFailed,
    #[error("no providers are configured")]
    NoProviders,
    #[error("provider produced output over the requested ceiling")]
    OutputTooLarge,
}

/// A single reason a provider call failed, distinct from routing decisions.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("provider failed: {0}")]
pub struct ProviderError(pub String);

/// A model provider. Real cloud providers implement this behind an egress
/// broker; the deterministic providers here implement it locally.
pub trait ModelProvider: Send + Sync {
    fn id(&self) -> &str;
    /// Where the provider says it runs. A display fact for the founder's
    /// disclosure log — never an authorization. See [`LocalVouch`].
    fn trust(&self) -> ProviderTrust;
    fn health(&self) -> Health;
    fn complete(&self, request: &ModelRequest) -> Result<String, ProviderError>;

    /// Return the vouch when this crate itself established the provider runs
    /// on this device. The default is `None`, and an outside implementor
    /// cannot do better: [`LocalVouch`] has no constructor it can reach.
    fn local_vouch(&self) -> Option<LocalVouch> {
        None
    }
}

impl fmt::Debug for dyn ModelProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ModelProvider")
            .field("id", &self.id())
            .field("trust", &self.trust())
            .field("health", &self.health())
            .finish()
    }
}

/// Ordered, health-aware, confidentiality-respecting model router.
#[derive(Debug, Default)]
pub struct ModelGateway {
    providers: Vec<Box<dyn ModelProvider>>,
}

impl ModelGateway {
    pub fn new(providers: Vec<Box<dyn ModelProvider>>) -> Self {
        Self { providers }
    }

    /// Route one request. Tries providers in order, skipping any that are
    /// confidentiality-ineligible or down, then any healthy provider, then
    /// (only if no healthy one served it) degraded providers. Returns the
    /// first successful response with a disclosure record of the path taken.
    pub fn complete(
        &self,
        request: &ModelRequest,
    ) -> Result<(ModelResponse, DisclosureRecord), ModelError> {
        if self.providers.is_empty() {
            return Err(ModelError::NoProviders);
        }
        // Two passes: prefer Healthy providers, then fall back to Degraded.
        // Down and confidentiality-ineligible providers are never used.
        let mut skipped = Vec::new();
        for allow_degraded in [false, true] {
            for (index, provider) in self.providers.iter().enumerate() {
                let eligible = self.classify(provider.as_ref(), request, allow_degraded);
                match eligible {
                    Eligibility::Skip(cause) => {
                        // Record each distinct skip once (on the pass that
                        // first rejects it) to keep the trail readable.
                        if !allow_degraded || cause != SkipCause::Unhealthy {
                            record_skip(&mut skipped, provider.id(), cause);
                        }
                        continue;
                    }
                    Eligibility::Try => match provider.complete(request) {
                        Ok(text) => {
                            if text.chars().count() > request.max_output_chars {
                                return Err(ModelError::OutputTooLarge);
                            }
                            let disclosure = DisclosureRecord {
                                task: request.task.clone(),
                                provider_id: provider.id().to_owned(),
                                provider_trust: provider.trust(),
                                data_class: request.data_class,
                                provider_index: index,
                                output_chars: text.chars().count(),
                                skipped: skipped.clone(),
                            };
                            return Ok((
                                ModelResponse {
                                    text,
                                    provider_id: provider.id().to_owned(),
                                    provider_trust: provider.trust(),
                                },
                                disclosure,
                            ));
                        }
                        Err(_) => {
                            record_skip(&mut skipped, provider.id(), SkipCause::Failed);
                            continue;
                        }
                    },
                }
            }
        }
        Err(ModelError::AllProvidersFailed)
    }

    fn classify(
        &self,
        provider: &dyn ModelProvider,
        request: &ModelRequest,
        allow_degraded: bool,
    ) -> Eligibility {
        // A raw request carries whatever the caller put in it, so it may only
        // reach a provider this crate vouches for. A trait method or a caller
        // string MUST NOT establish that trust (RFC 0004).
        if provider.local_vouch().is_none() {
            return Eligibility::Skip(SkipCause::RawRequestIsLocalOnly);
        }
        // Caller DataClass never grants egress. Unknown and legacy Amber/Green
        // enter this boundary as Protected. A cloud-labelled provider — even
        // one this crate vouched as a local-enough stand-in — does not receive
        // a raw request. DataClass may only narrow the recorded skip reason.
        if provider.trust() != ProviderTrust::Local {
            return Eligibility::Skip(match request.data_class {
                DataClass::Red => SkipCause::RedDataConfidentiality,
                DataClass::Amber | DataClass::Green => SkipCause::CloudLabelledDenied,
            });
        }
        match provider.health() {
            Health::Healthy => Eligibility::Try,
            Health::Degraded if allow_degraded => Eligibility::Try,
            Health::Degraded | Health::Down => Eligibility::Skip(SkipCause::Unhealthy),
        }
    }

    pub fn provider_ids(&self) -> Vec<&str> {
        self.providers
            .iter()
            .map(|provider| provider.id())
            .collect()
    }
}

enum Eligibility {
    Try,
    Skip(SkipCause),
}

fn record_skip(skipped: &mut Vec<SkipReason>, provider_id: &str, reason: SkipCause) {
    if !skipped.iter().any(|entry| entry.provider_id == provider_id) {
        skipped.push(SkipReason {
            provider_id: provider_id.to_owned(),
            reason,
        });
    }
}

/// A deterministic local provider for tests and demos. It is **not an LLM**:
/// it produces fixed, inspectable text so the gateway contract is provable
/// without a network or a model. Health is settable to exercise failover.
pub struct DeterministicProvider {
    id: String,
    trust: ProviderTrust,
    health: Health,
    /// If true, `complete` returns an error, to exercise the failed-then-fail-
    /// over path distinctly from a Down provider.
    fail_on_call: bool,
    template: fn(&ModelRequest, &str) -> String,
}

impl fmt::Debug for DeterministicProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeterministicProvider")
            .field("id", &self.id)
            .field("trust", &self.trust)
            .field("health", &self.health)
            .field("fail_on_call", &self.fail_on_call)
            .finish()
    }
}

impl DeterministicProvider {
    pub fn local(id: impl Into<String>, health: Health) -> Self {
        Self {
            id: id.into(),
            trust: ProviderTrust::Local,
            health,
            fail_on_call: false,
            template: default_template,
        }
    }

    /// A local provider that returns the request prompt verbatim. Used when
    /// the caller has already composed the draft deterministically and wants
    /// the gateway only for resilient routing and disclosure recording.
    pub fn local_echo(id: impl Into<String>, health: Health) -> Self {
        Self {
            id: id.into(),
            trust: ProviderTrust::Local,
            health,
            fail_on_call: false,
            template: echo_template,
        }
    }

    pub fn cloud(id: impl Into<String>, health: Health) -> Self {
        Self {
            id: id.into(),
            trust: ProviderTrust::Cloud,
            health,
            fail_on_call: false,
            template: default_template,
        }
    }

    pub fn failing(mut self) -> Self {
        self.fail_on_call = true;
        self
    }
}

fn echo_template(request: &ModelRequest, _provider_id: &str) -> String {
    request.prompt.clone()
}

fn default_template(request: &ModelRequest, provider_id: &str) -> String {
    format!(
        "[draft suggestion · {task} · via {provider_id}]\n{prompt}",
        task = request.task,
        provider_id = provider_id,
        prompt = request.prompt,
    )
}

impl ModelProvider for DeterministicProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn trust(&self) -> ProviderTrust {
        self.trust
    }

    /// A deterministic stand-in is core-reviewed code in this crate, so the
    /// local ones carry the vouch. `cloud` exists to simulate a provider this
    /// product does not vouch for, and must not.
    fn local_vouch(&self) -> Option<LocalVouch> {
        (self.trust == ProviderTrust::Local).then(LocalVouch::core_reviewed)
    }

    fn health(&self) -> Health {
        self.health
    }

    fn complete(&self, request: &ModelRequest) -> Result<String, ProviderError> {
        if self.fail_on_call {
            return Err(ProviderError(format!("{} simulated failure", self.id)));
        }
        Ok((self.template)(request, &self.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(data_class: DataClass) -> ModelRequest {
        ModelRequest {
            task: "draft_outreach".into(),
            prompt: "Say hello to Dr. Tan".into(),
            data_class,
            max_output_chars: 4096,
        }
    }

    #[test]
    fn primary_serves_when_healthy() {
        let gateway = ModelGateway::new(vec![
            Box::new(DeterministicProvider::local(
                "local-primary",
                Health::Healthy,
            )),
            Box::new(DeterministicProvider::local(
                "local-backup",
                Health::Healthy,
            )),
        ]);
        let (response, disclosure) = gateway.complete(&request(DataClass::Amber)).unwrap();
        assert_eq!(response.provider_id, "local-primary");
        assert!(disclosure.skipped.is_empty());
    }

    #[test]
    fn a_raw_request_never_reaches_a_provider_this_product_does_not_vouch_for() {
        // This inverts the previous rule on purpose (RFC 0004, "Current
        // gap"): an Amber label used to be enough to route a raw request to a
        // cloud provider. A caller can mislabel data and an adapter can claim
        // local trust, so a raw prompt now stops at the closed local list.
        // Reaching anything else takes a compiled projection.
        let gateway = ModelGateway::new(vec![
            Box::new(DeterministicProvider::local("local-drafter", Health::Down)),
            Box::new(DeterministicProvider::cloud("cloud", Health::Healthy)),
        ]);
        assert_eq!(
            gateway.complete(&request(DataClass::Amber)),
            Err(ModelError::AllProvidersFailed)
        );

        // Nor does a provider get in by naming itself like a local one: the
        // vouch is a value only this crate can construct, so a cloud stand-in
        // wearing a local-sounding id is still refused.
        let disguised = ModelGateway::new(vec![Box::new(DeterministicProvider::cloud(
            "local-drafter",
            Health::Healthy,
        ))]);
        assert_eq!(
            disguised.complete(&request(DataClass::Red)),
            Err(ModelError::AllProvidersFailed)
        );
        assert!(DeterministicProvider::cloud("anything", Health::Healthy)
            .local_vouch()
            .is_none());
        assert!(DeterministicProvider::local("anything", Health::Healthy)
            .local_vouch()
            .is_some());

        // A vouched local provider serves it.
        let local = ModelGateway::new(vec![Box::new(DeterministicProvider::local(
            "local-drafter",
            Health::Healthy,
        ))]);
        let (response, disclosure) = local.complete(&request(DataClass::Amber)).unwrap();
        assert_eq!(response.provider_id, "local-drafter");
        assert_eq!(disclosure.data_class, DataClass::Amber);
    }

    #[test]
    fn disclosure_names_serving_provider_and_records_failover() {
        // A failing primary and a healthy backup: the disclosure must name the
        // provider that actually served, its index, and record the skip.
        let gateway = ModelGateway::new(vec![
            Box::new(DeterministicProvider::local("primary", Health::Healthy).failing()),
            Box::new(DeterministicProvider::local("backup", Health::Healthy)),
        ]);
        let (response, disclosure) = gateway.complete(&request(DataClass::Amber)).unwrap();
        assert_eq!(response.provider_id, "backup");
        assert_eq!(disclosure.provider_id, "backup");
        assert_eq!(disclosure.provider_index, 1);
        assert!(disclosure
            .skipped
            .iter()
            .any(|skip| skip.provider_id == "primary" && skip.reason == SkipCause::Failed));
    }

    #[test]
    fn output_exceeding_the_limit_fails_closed() {
        // An echo provider returns the prompt verbatim; a prompt longer than
        // the caller's ceiling must be refused, not silently truncated or
        // passed through.
        let gateway = ModelGateway::new(vec![Box::new(DeterministicProvider::local_echo(
            "echo",
            Health::Healthy,
        ))]);
        let mut request = request(DataClass::Amber);
        request.max_output_chars = 4;
        request.prompt = "this prompt is far longer than four characters".into();
        assert_eq!(gateway.complete(&request), Err(ModelError::OutputTooLarge));
    }

    #[test]
    fn removing_primary_does_not_stop_the_workflow() {
        // Stage 2 exit criterion: primary down → work continues. Since RFC
        // 0004 closed raw egress, "continues" means a vouched local provider
        // takes over. Losing the primary must not become a reason to widen
        // the confidentiality route, so the cloud stand-in in the middle is
        // passed over for what it is, not tried and rejected later.
        let gateway = ModelGateway::new(vec![
            Box::new(DeterministicProvider::local("primary", Health::Down)),
            Box::new(DeterministicProvider::cloud(
                "cloud-backup",
                Health::Healthy,
            )),
            Box::new(DeterministicProvider::local(
                "local-fallback",
                Health::Healthy,
            )),
        ]);
        let (response, disclosure) = gateway.complete(&request(DataClass::Green)).unwrap();
        assert_eq!(response.provider_id, "local-fallback");
        assert_eq!(disclosure.provider_index, 2);
        assert_eq!(disclosure.skipped[0].provider_id, "primary");
        assert_eq!(disclosure.skipped[0].reason, SkipCause::Unhealthy);
        assert_eq!(disclosure.skipped[1].provider_id, "cloud-backup");
        assert_eq!(
            disclosure.skipped[1].reason,
            SkipCause::RawRequestIsLocalOnly
        );
    }

    #[test]
    fn failed_call_fails_over_to_next_provider() {
        let gateway = ModelGateway::new(vec![
            Box::new(DeterministicProvider::local("flaky", Health::Healthy).failing()),
            Box::new(DeterministicProvider::local("stable", Health::Healthy)),
        ]);
        let (response, disclosure) = gateway.complete(&request(DataClass::Green)).unwrap();
        assert_eq!(response.provider_id, "stable");
        assert_eq!(disclosure.skipped[0].reason, SkipCause::Failed);
    }

    #[test]
    fn degraded_used_only_after_healthy_exhausted() {
        let gateway = ModelGateway::new(vec![
            Box::new(DeterministicProvider::local("degraded", Health::Degraded)),
            Box::new(DeterministicProvider::local("healthy", Health::Healthy)),
        ]);
        // Healthy wins on the first pass even though degraded is listed first.
        let (response, _) = gateway.complete(&request(DataClass::Green)).unwrap();
        assert_eq!(response.provider_id, "healthy");

        // With only a degraded provider, the second pass uses it.
        let degraded_only = ModelGateway::new(vec![Box::new(DeterministicProvider::local(
            "degraded",
            Health::Degraded,
        ))]);
        let (response, _) = degraded_only.complete(&request(DataClass::Green)).unwrap();
        assert_eq!(response.provider_id, "degraded");
    }

    #[test]
    fn red_data_never_reaches_a_cloud_provider() {
        let gateway = ModelGateway::new(vec![
            Box::new(DeterministicProvider::cloud("cloud", Health::Healthy)),
            Box::new(DeterministicProvider::local("local", Health::Healthy)),
        ]);
        let (response, disclosure) = gateway.complete(&request(DataClass::Red)).unwrap();
        assert_eq!(response.provider_id, "local");
        assert_eq!(response.provider_trust, ProviderTrust::Local);
        // The unvouched provider is now refused one step earlier — for being
        // unvouched at all, rather than for the data class of this particular
        // request. The Red guard remains behind it as defence in depth for a
        // vouched provider that nonetheless reports non-local trust.
        assert_eq!(
            disclosure.skipped[0].reason,
            SkipCause::RawRequestIsLocalOnly
        );
    }

    #[test]
    fn red_data_with_only_cloud_providers_fails_closed() {
        let gateway = ModelGateway::new(vec![
            Box::new(DeterministicProvider::cloud("cloud-a", Health::Healthy)),
            Box::new(DeterministicProvider::cloud("cloud-b", Health::Healthy)),
        ]);
        // Red data cannot be served by any cloud provider: fail rather than leak.
        assert_eq!(
            gateway.complete(&request(DataClass::Red)),
            Err(ModelError::AllProvidersFailed)
        );
    }

    #[test]
    fn all_down_fails_closed() {
        let gateway = ModelGateway::new(vec![
            Box::new(DeterministicProvider::local("a", Health::Down)),
            Box::new(DeterministicProvider::local("b", Health::Down)),
        ]);
        assert_eq!(
            gateway.complete(&request(DataClass::Green)),
            Err(ModelError::AllProvidersFailed)
        );
    }

    #[test]
    fn no_providers_is_an_explicit_error() {
        let gateway = ModelGateway::default();
        assert_eq!(
            gateway.complete(&request(DataClass::Green)),
            Err(ModelError::NoProviders)
        );
    }

    /// A vouched stand-in that nonetheless self-reports cloud trust — models
    /// the case where this crate accepted a provider as local enough to exist
    /// while the provider still labels itself [`ProviderTrust::Cloud`].
    struct VouchedCloudTrustStandIn {
        hits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl ModelProvider for VouchedCloudTrustStandIn {
        fn id(&self) -> &str {
            "vouched-cloud"
        }

        fn trust(&self) -> ProviderTrust {
            ProviderTrust::Cloud
        }

        fn health(&self) -> Health {
            Health::Healthy
        }

        fn complete(&self, request: &ModelRequest) -> Result<String, ProviderError> {
            self.hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(request.prompt.clone())
        }

        fn local_vouch(&self) -> Option<LocalVouch> {
            Some(LocalVouch::core_reviewed())
        }
    }

    #[test]
    fn the_gateway_trusts_caller_labels_a_mislabeled_prompt_routes_to_cloud() {
        // Inverted from the v01-01 honesty pin (RFC 0004 Current gap): a
        // caller can still mislabel protected content Green or Amber, but
        // that label MUST NOT authorize a cloud-labelled provider — even one
        // this crate vouched as a local-enough stand-in. DataClass may only
        // narrow the skip reason; it never grants egress. Do not delete this
        // test.
        let hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cloud_only = ModelGateway::new(vec![Box::new(VouchedCloudTrustStandIn {
            hits: hits.clone(),
        })]);
        for class in [DataClass::Green, DataClass::Amber, DataClass::Red] {
            assert_eq!(
                cloud_only.complete(&request(class)),
                Err(ModelError::AllProvidersFailed),
                "{class:?} must not grant egress to a cloud-labelled provider"
            );
        }
        assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 0);

        let with_local = ModelGateway::new(vec![
            Box::new(VouchedCloudTrustStandIn { hits: hits.clone() }),
            Box::new(DeterministicProvider::local(
                "local-drafter",
                Health::Healthy,
            )),
        ]);
        let (response, disclosure) = with_local.complete(&request(DataClass::Green)).unwrap();
        assert_eq!(response.provider_id, "local-drafter");
        assert_eq!(disclosure.skipped[0].provider_id, "vouched-cloud");
        assert_eq!(disclosure.skipped[0].reason, SkipCause::CloudLabelledDenied);
        let (_, amber) = with_local.complete(&request(DataClass::Amber)).unwrap();
        assert_eq!(amber.skipped[0].reason, SkipCause::CloudLabelledDenied);
        let (_, red) = with_local.complete(&request(DataClass::Red)).unwrap();
        // DataClass may only narrow: Red records the more specific deny.
        assert_eq!(red.skipped[0].reason, SkipCause::RedDataConfidentiality);
        assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 0);
    }
}
