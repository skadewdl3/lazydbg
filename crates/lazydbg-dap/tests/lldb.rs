#![cfg(feature = "lldb-21")]

use lazydbg_dap::lldb::{AttachArguments, Configuration, Console, LaunchArguments};
use serde_json::json;

#[test]
fn serializes_lldb_21_field_names_and_extensions() {
    let mut launch = LaunchArguments {
        configuration: Configuration {
            program: Some("/tmp/a.out".into()),
            source_path: Some("/workspace".into()),
            ..Configuration::default()
        },
        console: Some(Console::IntegratedTerminal),
        disable_stdio: Some(true),
        ..LaunchArguments::default()
    };
    launch.extensions.insert("vendorSetting".into(), json!(7));
    let value = serde_json::to_value(launch).unwrap();
    assert_eq!(value["program"], "/tmp/a.out");
    assert_eq!(value["sourcePath"], "/workspace");
    assert_eq!(value["console"], "integratedTerminal");
    assert_eq!(value["disableSTDIO"], true);
    assert_eq!(value["vendorSetting"], 7);

    let attach = AttachArguments {
        gdb_remote_port: Some(1234),
        gdb_remote_hostname: Some("debug-host".into()),
        ..AttachArguments::default()
    };
    let value = serde_json::to_value(attach).unwrap();
    assert_eq!(value["gdb-remote-port"], 1234);
    assert_eq!(value["gdb-remote-hostname"], "debug-host");
}

#[cfg(feature = "lldb-22")]
#[test]
fn serializes_lldb_22_stdio() {
    let launch = LaunchArguments {
        stdio: Some(vec![None, Some("output.log".into())]),
        ..LaunchArguments::default()
    };
    assert_eq!(
        serde_json::to_value(launch).unwrap()["stdio"],
        json!([null, "output.log"])
    );
}
