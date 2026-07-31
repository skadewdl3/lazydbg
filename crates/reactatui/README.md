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
- `Stored<T>` (`stored`) is persistent mutable storage that does not redraw.
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

The built-in hooks are `state`, `stored`, `memo`, `effect`, `keys`, and `focus`.

`memo(|| compute())` automatically records `State::get` and `State::with`
calls made while computing. It recomputes when any recorded state version
changes, without comparing or hashing the state values, so those values do not
need `Eq` or `PartialEq`. A memo that reads no state computes only once.

Install application services lazily and read them by key inside components:

```rust,ignore
#[component]
fn Status<'a>() -> TuiNode<'a> {
    let app = resource_or("app_state", || AppState::new(/* ... */));
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

## Two-Way Bindings

Components can share a `State<T>` handle with their parent through a binding.
Bindings are required unless their parameter is `Option<State<T>>`. Use
`bind:name={...}` for named bindings and mark exactly one default binding with
`#[bind(default)]` when the parent should use `bind={...}`.

```rust,ignore
#[component]
fn Child(#[bind] value: State<String>) -> TuiNode<'static> {
    let text = bind(value);
    // text.set(...) updates the parent's state and requests a redraw.
    TuiNode::empty()
}

#[component]
fn Parent() -> TuiNode<'static> {
    let input_text = state(String::new);
    tui! { <Child bind:value={input_text} /> }
}

#[component]
fn DefaultChild(#[bind(default)] value: State<String>) -> TuiNode<'static> {
    TuiNode::empty()
}

// tui! { <DefaultChild bind={input_text} /> }
```

## Identity And Layout

Function component calls receive a stable source-call-site identity. Repeated
unkeyed calls fall back to their occurrence index, which keeps simple loops
convenient. Provide a key when rows can reorder or own persistent state:

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
| `global`, `global_or`, `try_use_global` | Keyed `resource::<T>(key)` and `resource_or(key, init)` |
| String `emitter`, `listen`, `bind` | Typed `Action` / `Callback<T>` component props |
| Source-based `computed` / `memo` | State-tracking `memo(compute)` |
| Manual frame/event dispatch | `Runtime::render` and `Runtime::handle_event` |
| Always redraw | Gate `terminal.draw` with `Runtime::needs_render()` |
| Positional hook slots | Source-call-site hook identity |

Pure components can still be called as ordinary Rust functions outside a
runtime. Hooks and event registration require a runtime render scope.
