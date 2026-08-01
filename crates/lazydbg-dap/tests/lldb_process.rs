use std::{
    process::{Command, Stdio},
    time::Duration,
};

use lazydbg_dap::{DisconnectArguments, InitializeRequestArguments, client::Client, error::Error};

#[test]
#[ignore = "requires lldb-dap on PATH; run explicitly as an adapter smoke test"]
fn initializes_and_disconnects_from_lldb_dap() {
    let executable = std::env::var_os("LLDB_DAP").unwrap_or_else(|| "lldb-dap".into());
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start lldb-dap");
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (client, _incoming) = Client::spawn(stdout, stdin);

    let _capabilities = client
        .send_timeout(
            &InitializeRequestArguments {
                adapter_id: "lldb-dap".into(),
                client_id: Some("lazydbg-dap-test".into()),
                client_name: Some("LazyDbg DAP smoke test".into()),
                columns_start_at1: Some(true),
                lines_start_at1: Some(true),
                locale: Some("en-US".into()),
                path_format: Some("path".into()),
                supports_ansi_styling: Some(true),
                supports_args_can_be_interpreted_by_shell: Some(false),
                supports_invalidated_event: Some(true),
                supports_memory_event: Some(true),
                supports_memory_references: Some(true),
                supports_progress_reporting: Some(true),
                supports_run_in_terminal_request: Some(false),
                supports_start_debugging_request: Some(false),
                supports_variable_paging: Some(true),
                supports_variable_type: Some(true),
            },
            Duration::from_secs(5),
        )
        .expect("initialize failed");

    let disconnect = client.send_timeout(&DisconnectArguments::default(), Duration::from_secs(5));
    assert!(disconnect.is_ok() || matches!(disconnect, Err(Error::Disconnected)));
    child.wait().expect("failed waiting for lldb-dap");
}
