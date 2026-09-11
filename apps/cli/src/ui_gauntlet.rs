//! The security gauntlet: ten real attacks on the running kernel, plus the
//! baseline that proves the harness itself works.
//!
//! Everything here happens in memory against the live crates — no fixtures,
//! no stubs — because a page that claims "these attacks are refused" is worth
//! nothing if the attacks were never run. `POST /api/gauntlet` is its only
//! caller.
//!
//! Split out of `ui.rs` when that file came within forty lines of the
//! god-file limit. The cut is here because the dependency runs one way: this
//! module uses nothing from the HTTP surface, and the HTTP surface uses one
//! function from it.

use crate::demo;
use chrono::Duration;
use sovereign_artifact::{
    AdmittedArtifact, ArtifactError, ArtifactStore, ArtifactVerificationIntent, ArtifactVerifier,
    Digest, OperationSelector, PreparedInvocation, RawResourceGrant, SystemClock as ArtifactClock,
    VerifiedArtifact,
};
use sovereign_audit_ledger::{AppendInput, AuditLedger};
use sovereign_capability::v2::{
    CapabilityIssuerV2, CapabilityTokenV2, CapabilityV2Error, CapabilityV2IssueOptions,
    CapabilityV2IssueRequest, CapabilityValidatorV2, SystemClock as CapabilityClock,
};
use sovereign_contracts::{ActionRequest, AutomationLevel, DataClass};
use sovereign_identity::{
    AdmissionRole, AuthorityRole, CompiledCacheRole, DeviceIdentity, KeyValidity, PublisherRole,
    RoleTrustStore, TypedSigner,
};
use sovereign_policy::{AuthenticatedPolicyContextV2, PolicyAuthorizationV2, PolicyEngine};
use sovereign_sandbox::{
    CompileWorker, CompiledCache, SandboxError, VerifiedExecutionRequest, VerifiedSandboxExecutor,
};
use uuid::Uuid;

struct Gauntlet {
    publisher: TypedSigner<PublisherRole>,
    publishers: RoleTrustStore<PublisherRole>,
    issuer: CapabilityIssuerV2<CapabilityClock>,
    policy: PolicyEngine,
    session_id: Uuid,
}

pub(crate) fn gauntlet_json() -> serde_json::Value {
    match run_gauntlet() {
        Ok(results) => serde_json::json!({ "ok": true, "results": results }),
        Err(error) => serde_json::json!({ "ok": false, "error": error.to_string() }),
    }
}

fn check(results: &mut Vec<serde_json::Value>, key: &str, name: &str, pass: bool, detail: &str) {
    results.push(serde_json::json!({ "key": key, "name": name, "pass": pass, "detail": detail }));
}

/// A compilation worker that re-executes this binary with the hidden compile
/// subcommand, so untrusted Wasmtime compilation runs in a killable child
/// process — under an address-space ceiling where the platform can enforce
/// one (`CompileWorker::address_space_enforcement`).
fn compile_worker() -> CompileWorker {
    let program = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("sovereign"));
    CompileWorker::new(program, vec![crate::COMPILE_WORKER_SUBCOMMAND.to_string()])
}

/// A trusted compiled cache signed and verified under a demo cache key.
fn compiled_cache(
    dir: &std::path::Path,
    now_unix: i64,
) -> Result<CompiledCache, Box<dyn std::error::Error>> {
    let signer = TypedSigner::<CompiledCacheRole>::from_secret_bytes(
        demo::CACHE_ISSUER,
        demo::DEMO_CACHE_SECRET,
    )?;
    let mut trust = RoleTrustStore::<CompiledCacheRole>::new();
    trust.trust_signer(&signer, KeyValidity::new(now_unix - 60, now_unix + 3_600)?)?;
    Ok(CompiledCache::open(
        dir,
        signer,
        trust,
        demo::CACHE_ISSUER,
        now_unix,
    )?)
}

fn run_gauntlet() -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let now_unix = chrono::Utc::now().timestamp();
    let validity = KeyValidity::new(now_unix - 60, now_unix + 3_600)?;

    let publisher = TypedSigner::<PublisherRole>::from_secret_bytes(
        demo::PUBLISHER_ISSUER,
        demo::DEMO_PUBLISHER_SECRET,
    )?;
    let mut publishers = RoleTrustStore::<PublisherRole>::new();
    publishers.trust_signer(&publisher, validity)?;

    let authority = TypedSigner::<AuthorityRole>::from_secret_bytes(
        demo::AUTHORITY_ISSUER,
        demo::DEMO_AUTHORITY_SECRET,
    )?;
    let mut authority_trust = RoleTrustStore::<AuthorityRole>::new();
    authority_trust.trust_signer(&authority, validity)?;
    let issuer = CapabilityIssuerV2::new(authority, demo::AUDIENCE, CapabilityClock)?;
    let validator = CapabilityValidatorV2::new(
        authority_trust,
        demo::AUTHORITY_ISSUER,
        demo::AUDIENCE,
        CapabilityClock,
    )?;

    let gauntlet = Gauntlet {
        publisher,
        publishers,
        issuer,
        policy: PolicyEngine::new(),
        session_id: Uuid::new_v4(),
    };

    let invoice_selector = OperationSelector::new("invoice.tools", "1.0.0", "validate")?;
    let stress_selector = OperationSelector::new("demo.stress", "1.0.0", "spin")?;
    // The gauntlet runs as the real `sovereign` binary, so every module below
    // is compiled out-of-process in a killable worker (this binary re-executed
    // with its hidden compile subcommand), and compiled results are cached
    // under a signed record verified before any reuse. Whether that worker
    // also runs under an address-space ceiling is platform-dependent — ask
    // `address_space_enforcement()` rather than assuming it.
    let cache_dir = std::env::temp_dir().join(format!(
        "sovereign-gauntlet-cache-{}",
        Uuid::new_v4().simple()
    ));
    let worker = compile_worker();
    let mut executor = VerifiedSandboxExecutor::new(
        vec![invoice_selector.clone(), stress_selector.clone()],
        validator,
    )?
    .with_compile_worker(worker.clone())
    .with_compiled_cache(compiled_cache(&cache_dir, now_unix)?);

    let invoice_component =
        demo::compile_wat(r#"(module (func (export "sovereign_run") (result i32) i32.const 0))"#);
    let invoice_artifact = gauntlet.verify(
        &demo::invoice_manifest_json(&gauntlet.publisher, &invoice_component),
        &invoice_component,
    )?;
    let stress_component = demo::compile_wat(
        r#"(module
            (func (export "sovereign_run") (result i32)
                (loop $forever i32.const 1 drop br $forever)
                unreachable))"#,
    );
    let stress_artifact = gauntlet.verify(
        &demo::stress_manifest_json(&gauntlet.publisher, &stress_component),
        &stress_component,
    )?;

    // The executor requires the owner-admitted handle (RFC 0002 step 8), so
    // the gauntlet admits both plugins through a throwaway content-addressed
    // store; the directory is ephemeral and removed at the end of the run.
    let admission_dir =
        std::env::temp_dir().join(format!("sovereign-gauntlet-{}", Uuid::new_v4().simple()));
    let admission_signer = TypedSigner::<AdmissionRole>::from_secret_bytes(
        demo::ADMISSION_ISSUER,
        demo::DEMO_ADMISSION_SECRET,
    )?;
    let admission_store = ArtifactStore::open(&admission_dir)?;
    let invoice_admitted =
        admission_store.admit(&invoice_artifact, &admission_signer, &ArtifactClock)?;
    let stress_admitted =
        admission_store.admit(&stress_artifact, &admission_signer, &ArtifactClock)?;

    let invocation = prepare_invoice(&invoice_artifact, 250_000)?;
    let (decision, idempotency) = gauntlet.decide(&invocation, AutomationLevel::L1Draft)?;
    let token = gauntlet.issue(&invocation, &decision, idempotency)?;

    let mut results = Vec::new();

    let baseline =
        executor.execute(gauntlet.request(&token, &invocation, &invoice_admitted, &decision));
    check(
        &mut results,
        "baseline",
        "Baseline: authorized execution",
        baseline.is_ok(),
        "signed plugin ran in the import-free Wasmtime sandbox under an exact one-use capability",
    );

    let replay =
        executor.execute(gauntlet.request(&token, &invocation, &invoice_admitted, &decision));
    check(
        &mut results,
        "replay",
        "Token replay",
        matches!(
            replay,
            Err(SandboxError::CapabilityV2(CapabilityV2Error::Replay))
        ),
        "re-presenting the consumed capability was denied (process-local replay defense)",
    );

    let (fresh_decision, fresh_idempotency) =
        gauntlet.decide(&invocation, AutomationLevel::L1Draft)?;
    let fresh_token = gauntlet.issue(&invocation, &fresh_decision, fresh_idempotency)?;
    let swapped = prepare_invoice(&invoice_artifact, 9_900_000)?;
    let substitution = executor.execute(gauntlet.request(
        &fresh_token,
        &swapped,
        &invoice_admitted,
        &fresh_decision,
    ));
    let substitution_denied = matches!(
        substitution,
        Err(SandboxError::CapabilityV2(
            CapabilityV2Error::InvocationMismatch("canonical_input_digest")
        ))
    );
    // A publisher-verified but wrongly-admitted handle must be refused
    // before the one-use token is consumed — admission binds execution to
    // exactly the artifact the owner admitted.
    let admission_mismatch = executor.execute(gauntlet.request(
        &fresh_token,
        &invocation,
        &stress_admitted,
        &fresh_decision,
    ));
    let admission_denied = matches!(admission_mismatch, Err(SandboxError::ArtifactNotAdmitted));
    let honest_reuse = executor
        .execute(gauntlet.request(
            &fresh_token,
            &invocation,
            &invoice_admitted,
            &fresh_decision,
        ))
        .is_ok();
    check(
        &mut results,
        "substitution",
        "Input substitution after authorization",
        substitution_denied && honest_reuse,
        "swapped input was denied by digest mismatch; the untouched token still ran the authorized input",
    );
    check(
        &mut results,
        "admission_binding",
        "Executing without the owner's admission",
        admission_denied && honest_reuse,
        "an admitted handle for a different artifact was refused before any code ran or any token was consumed",
    );

    // Compilation isolation: a component whose bytes are well-formed enough to
    // be admitted but fail Wasmtime compilation is compiled in the killable
    // worker, so the failure is contained in the child and surfaces as a
    // fail-closed CompileWorkerFailed in the parent — never a host crash.
    let malformed_component: Vec<u8> = b"\0asm\x01\0\0\0\x7f\xff\xff\xff\xff\x0f".to_vec();
    let compile_isolation = (|| -> Result<bool, Box<dyn std::error::Error>> {
        let malformed_artifact = gauntlet.verify(
            &demo::invoice_manifest_json(&gauntlet.publisher, &malformed_component),
            &malformed_component,
        )?;
        let malformed_admitted =
            admission_store.admit(&malformed_artifact, &admission_signer, &ArtifactClock)?;
        let malformed_invocation = prepare_invoice(&malformed_artifact, 250_000)?;
        let (bad_decision, bad_idempotency) =
            gauntlet.decide(&malformed_invocation, AutomationLevel::L1Draft)?;
        let bad_token = gauntlet.issue(&malformed_invocation, &bad_decision, bad_idempotency)?;
        Ok(matches!(
            executor.execute(gauntlet.request(
                &bad_token,
                &malformed_invocation,
                &malformed_admitted,
                &bad_decision,
            )),
            Err(SandboxError::CompileWorkerFailed(_) | SandboxError::CompileWorkerTimeout)
        ))
    })()
    .unwrap_or(false);
    // The baseline above compiled a known-good component through this same
    // worker against a fresh cache directory, so its success is independent
    // proof that the worker process actually launches here. Without it a
    // failed spawn would be indistinguishable from a contained failure.
    let verdict = crate::gauntlet_report::compile_isolation(
        baseline.is_ok(),
        compile_isolation,
        worker.address_space_enforcement(),
    );
    check(
        &mut results,
        "compile_isolation",
        "Hostile compilation in the host process",
        verdict.pass,
        &verdict.detail,
    );

    // Cache poisoning: the baseline execution stored a compiled blob under a
    // signed record. Flip the compiled bytes on disk, then run the same
    // artifact again: the tampered entry must be refused before any unsafe
    // deserialize, quarantined, and transparently recompiled — never executed.
    let cache_poisoning = (|| -> Result<bool, Box<dyn std::error::Error>> {
        let mut poisoned = false;
        for entry in std::fs::read_dir(&cache_dir)?.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|x| x == "blob") {
                let mut bytes = std::fs::read(&path)?;
                if let Some(byte) = bytes.first_mut() {
                    *byte ^= 0xff;
                    std::fs::write(&path, &bytes)?;
                    poisoned = true;
                }
            }
        }
        if !poisoned {
            return Ok(false);
        }
        let (decision, idempotency) = gauntlet.decide(&invocation, AutomationLevel::L1Draft)?;
        let token = gauntlet.issue(&invocation, &decision, idempotency)?;
        let ran = executor
            .execute(gauntlet.request(&token, &invocation, &invoice_admitted, &decision))
            .is_ok();
        let quarantined = std::fs::read_dir(cache_dir.join("quarantine"))
            .map(|it| it.flatten().count())
            .unwrap_or(0)
            > 0;
        Ok(ran && quarantined)
    })()
    .unwrap_or(false);
    check(
        &mut results,
        "cache_poisoning",
        "Poisoning the compiled-artifact cache",
        cache_poisoning,
        "a tampered cached blob failed its signed-record check, was quarantined before any deserialize, and the artifact was transparently recompiled from source",
    );

    let mut greedy = demo::invoice_manifest_json(&gauntlet.publisher, &invoice_component);
    greedy["requested_host_capabilities"] = serde_json::json!(["filesystem.read"]);
    let greedy_result = gauntlet.verify(&greedy, &invoice_component);
    check(
        &mut results,
        "greedy_manifest",
        "Manifest demands host capabilities",
        matches!(greedy_result, Err(ArtifactError::HostCapabilitiesForbidden)),
        "a manifest requesting filesystem access was rejected before any code ran",
    );

    let stress_input = serde_json::json!({ "resource": "stress:demo" });
    let stress_invocation = PreparedInvocation::prepare(
        &stress_artifact,
        &stress_selector,
        &serde_json::to_vec(&stress_input)?,
        vec![RawResourceGrant::new("primary", "stress:demo")],
    )?;
    let (stress_decision, stress_idempotency) =
        gauntlet.decide(&stress_invocation, AutomationLevel::L1Draft)?;
    let stress_token = gauntlet.issue(&stress_invocation, &stress_decision, stress_idempotency)?;
    let loop_result = executor.execute(gauntlet.request(
        &stress_token,
        &stress_invocation,
        &stress_admitted,
        &stress_decision,
    ));
    check(
        &mut results,
        "infinite_loop",
        "Infinite-loop plugin",
        matches!(loop_result, Err(SandboxError::FuelExhausted)),
        "runaway guest was killed by deterministic fuel metering; the attempt still consumed its token",
    );

    let red = gauntlet.policy.evaluate(ActionRequest {
        actor_id: "compromised_agent".into(),
        venture_id: demo::VENTURE.into(),
        tool: "cloud.model".into(),
        operation: "infer".into(),
        resource: "customer_database".into(),
        data_class: DataClass::Red,
        automation_level: AutomationLevel::L3BoundedAuto,
    });
    check(
        &mut results,
        "red_cloud",
        "Red-zone data to a cloud model",
        !red.allowed,
        &format!("deterministic policy denial: {}", red.reason),
    );

    let (l3_decision, l3_idempotency) =
        gauntlet.decide(&invocation, AutomationLevel::L3BoundedAuto)?;
    let approval = gauntlet.issue(&invocation, &l3_decision, l3_idempotency);
    check(
        &mut results,
        "approval",
        "High-impact action without human approval",
        matches!(
            approval,
            Err(CapabilityV2Error::ApprovalEvidenceUnavailable)
        ),
        "no capability is minted without approval evidence — the runtime fails closed",
    );

    let device = DeviceIdentity::generate();
    let mut ledger = AuditLedger::new();
    ledger.append(
        AppendInput {
            venture_id: demo::VENTURE.into(),
            actor_id: demo::SUBJECT.into(),
            action: "execute".into(),
            resource: demo::INVOICE_RESOURCE.into(),
            capability_id: None,
            payload: serde_json::json!({ "gauntlet": true }),
            policy_decision_hash: None,
        },
        &device,
    )?;
    let mut tampered_events = ledger.events().to_vec();
    tampered_events[0].action = "nothing_happened".into();
    let tamper_detected =
        AuditLedger::from_events(tampered_events, device.public_key_b64()).is_err();
    check(
        &mut results,
        "audit_tamper",
        "Audit history tampering",
        tamper_detected,
        "rewriting one recorded action broke the signed hash chain and was detected",
    );

    // The admission store was a throwaway for this run; best-effort cleanup
    // (an early error above may leak one temp dir — harmless, OS-cleaned).
    let _ = std::fs::remove_dir_all(&admission_dir);
    let _ = std::fs::remove_dir_all(&cache_dir);

    Ok(results)
}

impl Gauntlet {
    fn verify(
        &self,
        manifest_json: &serde_json::Value,
        component: &[u8],
    ) -> Result<VerifiedArtifact, ArtifactError> {
        let canonical = serde_json_canonicalizer::to_vec(manifest_json)
            .map_err(|_| ArtifactError::InputCanonicalizationFailed)?;
        let signed = self
            .publisher
            .sign_cose(&canonical)
            .map_err(|_| ArtifactError::PublisherVerificationFailed)?;
        let intent = ArtifactVerificationIntent::new(
            demo::PUBLISHER_ISSUER,
            Digest::of_bytes(&signed),
            Digest::of_bytes(component),
        )?;
        ArtifactVerifier::new(&self.publishers).verify(&intent, &signed, component)
    }

    fn decide(
        &self,
        invocation: &PreparedInvocation,
        automation_level: AutomationLevel,
    ) -> Result<(PolicyAuthorizationV2, Uuid), Box<dyn std::error::Error>> {
        let idempotency = Uuid::new_v4();
        let decision = self.policy.evaluate_prepared(
            invocation,
            AuthenticatedPolicyContextV2::new(
                demo::AUDIENCE,
                demo::VENTURE,
                demo::SUBJECT,
                self.session_id,
                DataClass::Green,
                automation_level,
                idempotency,
            )?,
        )?;
        Ok((decision, idempotency))
    }

    fn issue(
        &self,
        invocation: &PreparedInvocation,
        decision: &PolicyAuthorizationV2,
        idempotency: Uuid,
    ) -> Result<CapabilityTokenV2, CapabilityV2Error> {
        self.issuer.issue(CapabilityV2IssueRequest {
            venture_id: demo::VENTURE,
            subject_id: demo::SUBJECT,
            session_id: self.session_id,
            policy_decision: decision,
            prepared_invocation: invocation,
            options: CapabilityV2IssueOptions {
                ttl: Duration::seconds(60),
                idempotency_key: idempotency,
            },
        })
    }

    fn request<'a>(
        &self,
        token: &'a CapabilityTokenV2,
        invocation: &'a PreparedInvocation,
        admitted: &'a AdmittedArtifact,
        decision: &'a PolicyAuthorizationV2,
    ) -> VerifiedExecutionRequest<'a> {
        VerifiedExecutionRequest {
            token,
            invocation,
            admitted,
            venture_id: demo::VENTURE,
            subject_id: demo::SUBJECT,
            session_id: self.session_id,
            policy_decision: decision,
        }
    }
}

fn prepare_invoice(
    artifact: &VerifiedArtifact,
    total_cents: i64,
) -> Result<PreparedInvocation, ArtifactError> {
    let input = serde_json::json!({
        "invoice_id": demo::INVOICE_RESOURCE,
        "customer": "Acme Pte Ltd",
        "total_cents": total_cents
    });
    PreparedInvocation::prepare(
        artifact,
        &OperationSelector::new("invoice.tools", "1.0.0", "validate")?,
        &serde_json::to_vec(&input).expect("static demo input serializes"),
        vec![RawResourceGrant::new("primary", demo::INVOICE_RESOURCE)],
    )
}
