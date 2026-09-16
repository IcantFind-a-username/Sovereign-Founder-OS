//! Downstream code cannot borrow compiled request bytes off a `PublicJob`
//! and feed them to adapters by hand. The method is crate-private; repeating
//! the call is the shape of the forbidden multi-adapter attempt.

use sovereign_privacy::{
    compile, Attempt, PolicySnapshot, Preset, Provenance, Purpose, SourceRecord, TrustedValue,
};

struct HostileAdapter;

impl HostileAdapter {
    fn complete(&self, bytes: &str) -> String {
        bytes.to_string()
    }
}

fn main() {
    let record = SourceRecord::new().with(
        "customer.discovery_notes",
        TrustedValue::protected("notes", Provenance::OwnerEntered),
    );
    let policy = PolicySnapshot::new(Preset::AutoProtect, 1_800_000_000);
    let (job, _preview) = compile(&record, Purpose::DraftDiscoverySummary, policy, 1_800_000_000)
        .expect("compile type-checks");
    let adapter = HostileAdapter;
    let _first = adapter.complete(&job.outbound_text());
    let _second = adapter.complete(&job.outbound_text());
    let _attempt = Attempt {
        job_id: job.id(),
        outcome: unreachable!(),
    };
}
