# lazydbg-dap

Typed Rust models and serde support for the Debug Adapter Protocol.

The protocol specification is provided by the Microsoft Debug Adapter Protocol
repository in the `debug-adapter-protocol` git submodule. The checked-in Rust
types in `src/generated.rs` are generated from
`debug-adapter-protocol/debugAdapterProtocol.json`.

- Specification version: 1.71.0
- Upstream: https://github.com/microsoft/debug-adapter-protocol
- Upstream licenses: Creative Commons Attribution 3.0 and MIT

Initialize the submodule after cloning:

```text
git submodule update --init --recursive
```

Regenerate and verify the checked-in Rust types with:

```text
cargo run -p lazydbg-dap-codegen
cargo run -p lazydbg-dap-codegen -- --check
```
