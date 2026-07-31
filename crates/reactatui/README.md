# Reactatui Runtime

This branch moves Reactatui from process-global hook state to an owned `Runtime`
while keeping components and hooks familiar. Application code owns one runtime;
components remain zero-argument functions and continue to call free hooks.

```rust,ignore
#[component]
fn App<'a>() -> TuiNode<'a> {
    let count = state(|| 0);
    let increment = count.clone();

    tui! {
        <Button
            label="Increment"
            borders={Borders::ALL}
            disabled={false}
            focused={true}
            on:click={move || increment.update(|value| *value += 1)}
        />
    }
}

let runtime = Runtime::new();
loop {
    if runtime.needs_render() {
        terminal.draw(|frame| runtime.render(frame, frame.area(), App))?;
    }
    if event::poll(Duration::from_millis(16))? {
        runtime.handle_event(event::read()?);
    }
}
```

The scoped current-runtime mechanism is internal. It exists only during
`Runtime::render`, `Runtime::render_to_buffer`, and `Runtime::handle_event`, so
independent runtimes do not share state and users do not pass a context through
the component tree.

## Performance Model

- `State<T>` reads and writes through its own `Rc<RefCell<T>>`; repeated access
  does not query the erased hook map. Mutations request a redraw automatically.
- `Stored<T>` (`use_ref`) is persistent mutable storage that does not redraw.
- Hooks use source call sites for identity. Conditional hooks are allowed, and
  skipped hook state is cleaned up after the frame.
- Hook/component maps and frame queues retain their allocations between frames.
- Component ancestry stores one parent ID per component. Full paths are built
  only while dispatching an event.
- Literal `keybindings!` patterns are parsed by the proc macro and promoted to
  static key specifications instead of being parsed every frame or key press.
- Keyboard events route through the focused component and its ancestors. Mouse
  events target the last painted region and bubble through registered parents.
- `Size::Fr(1)` is the default. Content probing is reserved for explicit
  `size: auto`, avoiding scratch-buffer measurement for ordinary fill layouts.
- `view(widget)` preserves a concrete Ratatui widget type and avoids `TuiNode`
  boxing. `AnyView` is the explicit escape hatch for heterogeneous views.
- Rendering is demand-driven through `needs_render()`. `redraw_handle()` is a
  thread-safe signal for background work.

Measure the three main rendering paths with:

```sh
cargo run --release -p reactatui --example runtime_bench
```

The benchmark reports time, allocations, and allocated bytes per frame for a
direct Ratatui widget, `Runtime` plus a concrete typed view, and a component with
state and memo hooks.

## Hooks And Data

`state` and `use_state` are aliases. The remaining built-in hooks are
`use_ref`, `use_memo`, `use_effect`, `use_key`, and `use_focus`.

Install application services once and read them by type inside components:

```rust,ignore
runtime.insert_resource(AppState::new(&runtime));

#[component]
fn Status<'a>() -> TuiNode<'a> {
    let app = resource::<AppState>();
    // ...
}
```

Use `Action` for callbacks without a payload and `Callback<T>` for payloads.
Custom event syntax maps directly to typed props:

```rust,ignore
#[component]
fn SaveButton(#[prop] on_save: Action) -> TuiNode<'static> { /* ... */ }

tui! { <SaveButton on:save={move || save()} /> }
```

There is no string-based emitter/listener bus in the runtime.

## Identity And Layout

Function component calls receive a stable source-call-site identity. Components
created repeatedly at one call site must provide a key:

```rust,ignore
tui! {
    <Flex::vertical>
        for item in items {
            <Row key={item.id} item={item} />
        }
    </Flex>
}
```

Use fixed lengths, percentages, or fractional sizes for predictable one-pass
layout. Use `auto` only when intrinsic content measurement is actually needed.

## Migration

| Previous API | Runtime API |
| --- | --- |
| Process-global hooks | Own a `Runtime`; hooks remain free functions in components |
| `global`, `global_or`, `try_use_global` | `Runtime::insert_resource` and typed `resource::<T>()` |
| String `emitter`, `listen`, `bind` | Typed `Action` / `Callback<T>` component props |
| Source-based `computed` / `memo` | Dependency-based `use_memo(deps, compute)` |
| Manual frame/event dispatch | `Runtime::render` and `Runtime::handle_event` |
| Always redraw | Gate `terminal.draw` with `Runtime::needs_render()` |
| Positional hook slots | Source-call-site hook identity |

Pure components can still be called as ordinary Rust functions outside a
runtime. Hooks and event registration require a runtime render scope.
