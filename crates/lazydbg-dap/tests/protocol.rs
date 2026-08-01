use lazydbg_dap::{StoppedEvent, protocol::Extensible};
use serde_json::json;

#[test]
fn extensible_messages_retain_unknown_fields() {
    let raw = json!({
        "seq": 4,
        "type": "event",
        "event": "stopped",
        "body": { "reason": "vendor-stop", "vendorDetail": 42 },
        "vendorEnvelope": true
    });
    let event = Extensible::<StoppedEvent>::from_value(raw.clone()).unwrap();
    assert_eq!(event.body.reason, "vendor-stop");
    assert_eq!(event.raw(), &raw);
    assert_eq!(serde_json::to_value(&event).unwrap(), raw);
}
