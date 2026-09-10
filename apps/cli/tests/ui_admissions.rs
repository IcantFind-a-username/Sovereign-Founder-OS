//! What the Security page says about the admission records the product
//! itself wrote.

#[path = "support/ui_server.rs"]
mod ui_server;

use serde_json::json;
use ui_server::{pending_approval, UiServer};

/// The send path admits the built-in delivery tool under the owner's
/// admission key. The Security page verified every record against the demo
/// command's key instead, so the one record every installation has showed
/// as "failed verification" — on a fresh install, with nothing wrong.
#[test]
fn a_record_the_send_path_wrote_verifies_on_the_security_page() {
    let server = UiServer::start();
    let approval_id = pending_approval(&server);
    let decided = server
        .post(
            "/api/workspace/decide",
            &json!({ "approval_id": approval_id, "approve": true }),
        )
        .json();
    assert_eq!(decided["ok"], true, "{decided}");

    let state = server.get("/api/state").json();
    let plugins = state["plugins"].as_array().expect("plugins array");
    assert_eq!(
        plugins.len(),
        1,
        "one admitted tool after one send: {state}"
    );
    let plugin = &plugins[0];
    assert_eq!(plugin["verified"], true, "{plugin}");
    assert_eq!(plugin["issuer"], "founder-device.workspace", "{plugin}");
}
