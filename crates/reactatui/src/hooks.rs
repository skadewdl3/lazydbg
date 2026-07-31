use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;

use crate::keys::ParsedKeySpec;

/// Owns all component state and event registrations for one UI tree.
///
/// A runtime is installed only while it builds/renders a tree or dispatches an
/// event. Components can therefore keep using the familiar free-standing hook
/// functions without sharing process-global state.
#[derive(Clone)]
pub struct Runtime {
    inner: Rc<RuntimeInner>,
}

struct RuntimeInner {
    hooks: RefCell<HookRuntime>,
    redraw: RedrawHandle,
}

/// A thread-safe redraw flag that can be handed to background workers.
#[derive(Clone)]
pub struct RedrawHandle(Arc<AtomicBool>);

impl RedrawHandle {
    pub fn request(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_requested(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EventOutcome {
    pub handled: bool,
    pub redraw_requested: bool,
}

thread_local! {
    static CURRENT_RUNTIME: RefCell<Vec<Rc<RuntimeInner>>> = const { RefCell::new(Vec::new()) };
}

struct RuntimeAccess;

static RUNTIME: RuntimeAccess = RuntimeAccess;

impl RuntimeAccess {
    fn with<R>(&self, f: impl FnOnce(&RefCell<HookRuntime>) -> R) -> R {
        CURRENT_RUNTIME.with(|stack| {
            let inner = stack
                .borrow()
                .last()
                .cloned()
                .expect("reactatui hook used outside Runtime::render/handle_event");
            f(&inner.hooks)
        })
    }

    fn request_redraw(&self) {
        CURRENT_RUNTIME.with(|stack| {
            let inner = stack
                .borrow()
                .last()
                .cloned()
                .expect("reactatui state used outside its Runtime scope");
            inner.redraw.request();
        });
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RuntimeInner {
                hooks: RefCell::new(HookRuntime::default()),
                redraw: RedrawHandle(Arc::new(AtomicBool::new(true))),
            }),
        }
    }

    fn scoped<R>(&self, f: impl FnOnce() -> R) -> R {
        struct Scope;
        impl Drop for Scope {
            fn drop(&mut self) {
                CURRENT_RUNTIME.with(|stack| {
                    stack.borrow_mut().pop();
                });
            }
        }

        CURRENT_RUNTIME.with(|stack| stack.borrow_mut().push(self.inner.clone()));
        let _scope = Scope;
        f()
    }

    pub fn render<V>(&self, frame: &mut ratatui::Frame<'_>, area: Rect, view: impl FnOnce() -> V)
    where
        V: crate::view::View,
    {
        self.scoped(|| {
            self.inner.redraw.0.store(false, Ordering::Release);
            begin_frame();
            frame.render_widget(crate::view::ViewWidget(view()), area);
            finish_frame();
        });
    }

    /// Renders a frame directly into a buffer.
    ///
    /// This is the headless counterpart to [`Runtime::render`], useful for
    /// tests, snapshots, benchmarks, and renderers that already own a buffer.
    pub fn render_to_buffer<V>(
        &self,
        buffer: &mut ratatui::buffer::Buffer,
        area: Rect,
        view: impl FnOnce() -> V,
    ) where
        V: crate::view::View,
    {
        self.scoped(|| {
            self.inner.redraw.0.store(false, Ordering::Release);
            begin_frame();
            crate::view::View::render(view(), area, buffer);
            finish_frame();
        });
    }

    pub fn handle_event(&self, event: ratatui::crossterm::event::Event) -> EventOutcome {
        let handled = self.scoped(|| match event {
            ratatui::crossterm::event::Event::Key(key) => dispatch_key(key),
            ratatui::crossterm::event::Event::Mouse(mouse) => dispatch_mouse(mouse),
            ratatui::crossterm::event::Event::Resize(_, _) => {
                self.request_render();
                true
            }
            _ => false,
        });
        EventOutcome {
            handled,
            redraw_requested: self.needs_render(),
        }
    }

    pub fn needs_render(&self) -> bool {
        self.inner.redraw.is_requested()
    }

    pub fn request_render(&self) {
        self.inner.redraw.request();
    }

    pub fn redraw_handle(&self) -> RedrawHandle {
        self.inner.redraw.clone()
    }

    pub fn create_state<T: 'static>(&self, value: T) -> State<T> {
        State::new(value, self.redraw_handle())
    }

    pub fn insert_resource<T: 'static>(&self, value: T) -> Resource<T> {
        let resource = Resource(Rc::new(value));
        self.inner
            .hooks
            .borrow_mut()
            .resources
            .insert(TypeId::of::<T>(), Box::new(resource.clone()));
        resource
    }

    pub fn resource<T: 'static>(&self) -> Option<Resource<T>> {
        self.inner
            .hooks
            .borrow()
            .resources
            .get(&TypeId::of::<T>())
            .and_then(|value| value.downcast_ref::<Resource<T>>())
            .cloned()
    }
}

pub struct Resource<T>(Rc<T>);

impl<T> Clone for Resource<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Deref for Resource<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn resource<T: 'static>() -> Resource<T> {
    try_resource()
        .unwrap_or_else(|| panic!("resource `{}` is not installed", std::any::type_name::<T>()))
}

pub fn try_resource<T: 'static>() -> Option<Resource<T>> {
    RUNTIME.with(|rt| {
        rt.borrow()
            .resources
            .get(&TypeId::of::<T>())
            .and_then(|value| value.downcast_ref::<Resource<T>>())
            .cloned()
    })
}

pub struct Callback<T>(Rc<RefCell<dyn FnMut(T)>>);

impl<T> Clone for Callback<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T, F> From<F> for Callback<T>
where
    F: FnMut(T) + 'static,
{
    fn from(handler: F) -> Self {
        Self(Rc::new(RefCell::new(handler)))
    }
}

impl<T> Callback<T> {
    pub fn call(&self, value: T) {
        (self.0.borrow_mut())(value);
    }
}

/// A typed callback with no payload.
pub struct Action(Rc<RefCell<dyn FnMut()>>);

impl Clone for Action {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<F> From<F> for Action
where
    F: FnMut() + 'static,
{
    fn from(handler: F) -> Self {
        Self(Rc::new(RefCell::new(handler)))
    }
}

impl Action {
    pub fn call(&self) {
        (self.0.borrow_mut())();
    }
}

type HookSite = (&'static str, u32, u32);

struct KeyBinding {
    component_id: u64,
    matches: Box<dyn Fn(&KeyEvent) -> bool>,
    handler: Option<Box<dyn FnMut(KeyEvent) -> Propagation>>,
}

const CHORD_TIMEOUT: Duration = Duration::from_millis(1000);

struct ChordBinding {
    // Each inner Vec is one alternative full sequence, e.g. `"g-g" | "home"`
    // registers two alternatives, lengths 2 and 1.
    alternatives: &'static [&'static [ParsedKeySpec]],
    handler: Option<Box<dyn FnMut() -> Propagation>>,
    component_id: u64,
}

/// Controls whether a custom-event handler allows the event to continue
/// bubbling up the component tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Propagation {
    /// Let the event keep bubbling to ancestor listeners.
    Continue,
    /// Stop here; no further ancestors will receive this event.
    Stop,
}

/// A registered interactive screen region. Populated every render frame
/// by `register_mouse_region` (called from macro-generated code).
///
/// Click and scroll handlers return `Propagation` so the dispatch loop
/// can stop bubbling when a handler returns `Propagation::Stop`.
struct MouseRegion {
    rect: Rect,
    /// Stable id derived from the owning component + per-component registration
    /// order — survives layout shifts caused by window resize.
    id: u64,
    parent: Option<usize>,
    component_id: u64,
    click_handler: Option<Box<dyn FnMut(MouseButton) -> Propagation>>,
    mousein_handler: Option<Box<dyn FnMut()>>,
    mouseout_handler: Option<Box<dyn FnMut()>>,
    scrollx_handler: Option<Box<dyn FnMut(i16) -> Propagation>>,
    scrolly_handler: Option<Box<dyn FnMut(i16) -> Propagation>>,
}

#[derive(Clone, Copy)]
struct PendingComponentKey {
    hash: u64,
    explicit: bool,
}

#[derive(Default)]
struct HookRuntime {
    id_stack: Vec<u64>,
    sibling_counters: Vec<u32>,
    states: HashMap<(u64, HookSite), Box<dyn Any>>,
    resources: HashMap<TypeId, Box<dyn Any>>,
    seen_state_keys: HashSet<(u64, HookSite)>,
    seen_components: HashSet<u64>,
    pending_effects: Vec<Box<dyn FnOnce()>>,
    key_bindings: Vec<KeyBinding>,
    /// Per-frame mouse regions — rebuilt every render.
    mouse_regions: Vec<MouseRegion>,
    /// Which region ids were hovered last frame (for hover-enter tracking).
    prev_hovered: HashSet<u64>,
    hover_scratch: HashSet<u64>,
    /// Parent links for focus and event bubbling. Unlike storing full paths,
    /// this requires no per-component allocation during steady-state renders.
    component_parents: HashMap<u64, Option<u64>>,
    chord_bindings: Vec<ChordBinding>,
    chord_pending: Vec<KeyEvent>,
    chord_last_event_at: Option<Instant>,
    /// Per-component count of mouse regions registered this frame — used
    /// to generate stable region IDs that survive layout shifts.
    mouse_region_idx: HashMap<u64, u32>,
    mouse_region_stack: Vec<usize>,
    focused_component: Option<u64>,
    pending_component_keys: Vec<PendingComponentKey>,
    component_callsite_counts: HashMap<(u64, u64), u32>,
}

pub struct MouseRegionGuard;

impl Drop for MouseRegionGuard {
    fn drop(&mut self) {
        RUNTIME.with(|rt| {
            rt.borrow_mut().mouse_region_stack.pop();
        });
    }
}

/// Call once per frame, before building the component tree, to reset
/// per-frame bookkeeping. Hook state is preserved until its component or call
/// site disappears, or until the owning runtime is dropped.
fn begin_frame() {
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        debug_assert!(
            rt.id_stack.is_empty(),
            "begin_frame() called while a #[component] is still on the stack"
        );
        rt.key_bindings.clear();
        // mouse_regions are cleared here; prev_hovered is preserved across frames
        // so hover-enter detection works.
        rt.mouse_regions.clear();
        rt.chord_bindings.clear();
        // Reset per-component mouse region counters for the new frame.
        rt.mouse_region_idx.clear();
        rt.mouse_region_stack.clear();
        rt.component_callsite_counts.clear();
        rt.seen_state_keys.clear();
        rt.seen_components.clear();
        rt.pending_effects.clear();
    });
}

fn finish_frame() {
    let (removed, effects) = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let stale: Vec<_> = rt
            .states
            .keys()
            .filter(|key| !rt.seen_state_keys.contains(key))
            .copied()
            .collect();
        let removed = stale
            .into_iter()
            .filter_map(|key| rt.states.remove(&key))
            .collect::<Vec<_>>();
        if let Some(focused) = rt.focused_component
            && !rt.seen_components.contains(&focused)
        {
            let mut candidate = rt.component_parents.get(&focused).copied().flatten();
            while candidate.is_some_and(|component| !rt.seen_components.contains(&component)) {
                candidate = candidate
                    .and_then(|component| rt.component_parents.get(&component).copied().flatten());
            }
            rt.focused_component = candidate;
        }
        let HookRuntime {
            component_parents,
            seen_components,
            ..
        } = &mut *rt;
        component_parents.retain(|component, _| seen_components.contains(component));
        seen_components.clear();
        rt.seen_state_keys.clear();
        let effects = std::mem::take(&mut rt.pending_effects);
        (removed, effects)
    });
    drop(removed);
    let mut effects = effects;
    for effect in effects.drain(..) {
        effect();
    }
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        if rt.pending_effects.capacity() < effects.capacity() {
            rt.pending_effects = effects;
        }
    });
}

/// Called from macro-generated render closures to record that an
/// interactive element occupies `rect` this frame.
///
/// **Click and scroll handlers now return `Propagation`** so that bubbling
/// can be stopped at the appropriate layer.
#[doc(hidden)]
pub fn register_mouse_region(
    component_id: u64,
    rect: Rect,
    click_handler: Option<Box<dyn FnMut(MouseButton) -> Propagation>>,
    mousein_handler: Option<Box<dyn FnMut()>>,
    mouseout_handler: Option<Box<dyn FnMut()>>,
    scrollx_handler: Option<Box<dyn FnMut(i16) -> Propagation>>,
    scrolly_handler: Option<Box<dyn FnMut(i16) -> Propagation>>,
) -> MouseRegionGuard {
    // Derive a stable ID from (component_id, per-component registration count).
    // This survives layout shifts (window resize, scroll) because it doesn't
    // encode absolute screen coordinates.
    let id = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let local_idx = {
            let entry = rt.mouse_region_idx.entry(component_id).or_insert(0);
            let val = *entry;
            *entry += 1;
            val
        };
        let mut hasher = DefaultHasher::new();
        component_id.hash(&mut hasher);
        local_idx.hash(&mut hasher);
        hasher.finish()
    });

    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let parent = rt.mouse_region_stack.last().copied();
        let index = rt.mouse_regions.len();
        rt.mouse_regions.push(MouseRegion {
            rect,
            id,
            parent,
            component_id,
            click_handler,
            mousein_handler,
            mouseout_handler,
            scrollx_handler,
            scrolly_handler,
        });
        rt.mouse_region_stack.push(index);
    });
    MouseRegionGuard
}

/// Returns the current number of registered mouse regions. Used to track
/// regions registered during a temporary measurement pass.
pub fn mouse_region_count() -> usize {
    RUNTIME.with(|rt| rt.borrow().mouse_regions.len())
}

/// Transforms mouse regions that were registered into a temporary buffer
/// (e.g. during `measure_node`) so they match the final blitted screen coordinates.
/// `dx` and `dy` are the translations applied to the top-left of the original probe area.
/// `clip` is the final screen bounding box; any region outside it is truncated.
pub fn transform_mouse_regions(start: usize, end: usize, dx: i32, dy: i32, clip: Option<Rect>) {
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let end = end.min(rt.mouse_regions.len());
        for i in start..end {
            let r = &mut rt.mouse_regions[i];
            let x = (r.rect.x as i32 + dx).clamp(0, u16::MAX as i32) as u16;
            let y = (r.rect.y as i32 + dy).clamp(0, u16::MAX as i32) as u16;
            let shifted = Rect::new(x, y, r.rect.width, r.rect.height);
            if let Some(c) = clip {
                r.rect = shifted.intersection(c);
            } else {
                r.rect = shifted;
            }
        }
    });
}

/// Hides mouse regions that were registered during a measurement pass but
/// never ultimately blitted to the screen (e.g. discarded by layout).
pub fn cull_mouse_regions(start: usize, end: usize) {
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let end = end.min(rt.mouse_regions.len());
        for i in start..end {
            let r = &mut rt.mouse_regions[i];
            r.rect.width = 0;
            r.rect.height = 0;
        }
    });
}

// ---------------------------------------------------------------------------
// Mouse dispatch helpers
// ---------------------------------------------------------------------------

/// Collect mouse-region indices on the bubbling path for the point `(col, row)`.
///
/// The "target" is the smallest-area region containing the point (innermost
/// child in a correctly-laid-out tree). Only regions that fully contain the
/// target's rect are considered ancestors and included in the path.
/// Result is sorted innermost-first (ascending area) — the natural bubbling
/// order for DOM-style event propagation.
///
/// `has_handler` filters to only regions that have a relevant handler set.
fn mouse_target(col: u16, row: u16) -> Option<usize> {
    RUNTIME.with(|rt| {
        rt.borrow()
            .mouse_regions
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, region)| contains(region.rect, col, row).then_some(index))
    })
}

/// Process one mouse event. Call this in your event loop alongside
/// `dispatch_key`.
///
/// **Bubbling semantics**: click and scroll events are dispatched to the
/// innermost (smallest-area) region containing the cursor first, then bubble
/// outward through ancestor regions. A handler returning `Propagation::Stop`
/// halts the bubble. Hover (`mousein`/`mouseout`) events fire for every
/// region whose hover state changes (no propagation concept — they fire
/// independently per region).
pub fn dispatch_mouse(event: MouseEvent) -> bool {
    let col = event.column;
    let row = event.row;

    match event.kind {
        MouseEventKind::Down(button) => dispatch_mouse_click(col, row, button),
        MouseEventKind::ScrollUp => dispatch_mouse_scroll_y(col, row, -1),
        MouseEventKind::ScrollDown => dispatch_mouse_scroll_y(col, row, 1),
        MouseEventKind::ScrollLeft => dispatch_mouse_scroll_x(col, row, -1),
        MouseEventKind::ScrollRight => dispatch_mouse_scroll_x(col, row, 1),
        MouseEventKind::Moved => dispatch_mouse_move(col, row),
        _ => false,
    }
}

fn dispatch_mouse_click(col: u16, row: u16, button: MouseButton) -> bool {
    let mut current = mouse_target(col, row);
    let mut handled = current.is_some();
    let focus_target = current.and_then(|index| {
        RUNTIME.with(|rt| rt.borrow().mouse_regions.get(index).map(|r| r.component_id))
    });
    if let Some(component_id) = focus_target {
        let changed = RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            let changed = rt.focused_component != Some(component_id);
            rt.focused_component = Some(component_id);
            changed
        });
        if changed {
            RUNTIME.request_redraw();
        }
    }
    while let Some(i) = current {
        current = RUNTIME.with(|rt| rt.borrow().mouse_regions.get(i).and_then(|r| r.parent));
        let handler_opt = RUNTIME.with(|rt| {
            rt.borrow_mut()
                .mouse_regions
                .get_mut(i)
                .and_then(|r| r.click_handler.take())
        });
        if let Some(mut handler) = handler_opt {
            handled = true;
            let prop = handler(button);
            RUNTIME.with(|rt| {
                if let Some(r) = rt.borrow_mut().mouse_regions.get_mut(i) {
                    r.click_handler = Some(handler);
                }
            });
            if prop == Propagation::Stop {
                break;
            }
        }
    }
    handled
}

fn dispatch_mouse_scroll_y(col: u16, row: u16, delta: i16) -> bool {
    let mut current = mouse_target(col, row);
    let mut handled = false;
    while let Some(i) = current {
        current = RUNTIME.with(|rt| rt.borrow().mouse_regions.get(i).and_then(|r| r.parent));
        let handler_opt = RUNTIME.with(|rt| {
            rt.borrow_mut()
                .mouse_regions
                .get_mut(i)
                .and_then(|r| r.scrolly_handler.take())
        });
        if let Some(mut handler) = handler_opt {
            handled = true;
            let prop = handler(delta);
            RUNTIME.with(|rt| {
                if let Some(r) = rt.borrow_mut().mouse_regions.get_mut(i) {
                    r.scrolly_handler = Some(handler);
                }
            });
            if prop == Propagation::Stop {
                break;
            }
        }
    }
    handled
}

fn dispatch_mouse_scroll_x(col: u16, row: u16, delta: i16) -> bool {
    let mut current = mouse_target(col, row);
    let mut handled = false;
    while let Some(i) = current {
        current = RUNTIME.with(|rt| rt.borrow().mouse_regions.get(i).and_then(|r| r.parent));
        let handler_opt = RUNTIME.with(|rt| {
            rt.borrow_mut()
                .mouse_regions
                .get_mut(i)
                .and_then(|r| r.scrollx_handler.take())
        });
        if let Some(mut handler) = handler_opt {
            handled = true;
            let prop = handler(delta);
            RUNTIME.with(|rt| {
                if let Some(r) = rt.borrow_mut().mouse_regions.get_mut(i) {
                    r.scrollx_handler = Some(handler);
                }
            });
            if prop == Propagation::Stop {
                break;
            }
        }
    }
    handled
}

fn dispatch_mouse_move(col: u16, row: u16) -> bool {
    let mut mousein_handlers_to_run = Vec::new();
    let mut mouseout_handlers_to_run = Vec::new();

    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let prev = std::mem::take(&mut rt.prev_hovered);
        let mut now_hovered = std::mem::take(&mut rt.hover_scratch);
        now_hovered.clear();

        for (index, region) in rt.mouse_regions.iter_mut().enumerate() {
            if contains(region.rect, col, row) {
                now_hovered.insert(region.id);
                // Fire on:mousein only when the cursor first enters.
                if !prev.contains(&region.id)
                    && let Some(handler) = region.mousein_handler.take()
                {
                    mousein_handlers_to_run.push((index, handler));
                }
            } else if prev.contains(&region.id) {
                // Fire on:mouseout when the cursor leaves.
                if let Some(handler) = region.mouseout_handler.take() {
                    mouseout_handlers_to_run.push((index, handler));
                }
            }
        }
        rt.prev_hovered = now_hovered;
        rt.hover_scratch = prev;
        rt.hover_scratch.clear();
    });

    let handled = !mousein_handlers_to_run.is_empty() || !mouseout_handlers_to_run.is_empty();
    for (index, mut handler) in mousein_handlers_to_run {
        handler();
        RUNTIME.with(|rt| {
            if let Some(region) = rt.borrow_mut().mouse_regions.get_mut(index) {
                region.mousein_handler = Some(handler);
            }
        });
    }
    for (index, mut handler) in mouseout_handlers_to_run {
        handler();
        RUNTIME.with(|rt| {
            if let Some(region) = rt.borrow_mut().mouse_regions.get_mut(index) {
                region.mouseout_handler = Some(handler);
            }
        });
    }
    handled
}

#[inline]
fn contains(rect: Rect, col: u16, row: u16) -> bool {
    col >= rect.x
        && col < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}

/// Run every handler registered this frame whose binding matches.
/// Returns `true` if at least one handler fired.
pub fn dispatch_key(event: KeyEvent) -> bool {
    if event.kind == KeyEventKind::Release {
        return false;
    }
    // Figure out how deep the best `key_bindings` candidate is *before*
    // committing to chord handling. This is what was missing before:
    // chords used to run unconditionally and could swallow an event —
    // even just as an in-progress partial sequence — with no idea that a
    // deeper, more specific `on`/`on_when` handler existed and wanted to
    // fire (and possibly return `Propagation::Stop`).
    let key_depth = peek_key_binding_target_depth(event);

    if let Some(handled) = try_dispatch_chord(event, key_depth) {
        return handled;
    }

    dispatch_key_bindings(event)
}

fn is_on_focus_path(rt: &HookRuntime, component_id: u64) -> bool {
    let Some(focused) = rt.focused_component else {
        return true;
    };
    let mut current = Some(focused);
    while let Some(id) = current {
        if id == component_id {
            return true;
        }
        current = rt.component_parents.get(&id).copied().flatten();
    }
    false
}

fn component_depth(rt: &HookRuntime, component_id: u64) -> usize {
    let mut depth = 0;
    let mut current = Some(component_id);
    while let Some(id) = current {
        depth += 1;
        current = rt.component_parents.get(&id).copied().flatten();
    }
    depth
}

fn component_ancestry(rt: &HookRuntime, component_id: u64) -> Vec<u64> {
    let mut path = Vec::with_capacity(component_depth(rt, component_id));
    let mut current = Some(component_id);
    while let Some(id) = current {
        path.push(id);
        current = rt.component_parents.get(&id).copied().flatten();
    }
    path.reverse();
    path
}

/// Read-only lookup of the deepest `key_bindings` component whose matcher
/// fires for `event`, without taking or running any handler. Used only to
/// compare against chord candidates so the two systems can agree on who
/// gets the event before either commits to handling it.
fn peek_key_binding_target_depth(event: KeyEvent) -> Option<usize> {
    RUNTIME.with(|rt| {
        let rt = rt.borrow();
        rt.key_bindings
            .iter()
            .filter(|binding| {
                binding.handler.is_some()
                    && is_on_focus_path(&rt, binding.component_id)
                    && (binding.matches)(&event)
            })
            .map(|binding| component_depth(&rt, binding.component_id))
            .max()
    })
}

/// The regular (non-chord) dispatch path — this is the old body of
fn dispatch_key_bindings(event: KeyEvent) -> bool {
    let ancestry_path = RUNTIME.with(|rt| {
        let rt = rt.borrow();
        if let Some(focused) = rt.focused_component {
            return Some(component_ancestry(&rt, focused));
        }

        rt.key_bindings
            .iter()
            .filter(|binding| binding.handler.is_some() && (binding.matches)(&event))
            .max_by_key(|binding| component_depth(&rt, binding.component_id))
            .map(|binding| component_ancestry(&rt, binding.component_id))
    });

    let Some(ancestry_path) = ancestry_path else {
        return false;
    };

    let mut handled = false;
    for component_id in ancestry_path.iter().rev().copied() {
        let binding_count = RUNTIME.with(|rt| rt.borrow().key_bindings.len());
        for index in 0..binding_count {
            let matches = RUNTIME.with(|rt| {
                let rt = rt.borrow();
                rt.key_bindings.get(index).is_some_and(|binding| {
                    binding.component_id == component_id
                        && binding.handler.is_some()
                        && (binding.matches)(&event)
                })
            });
            if !matches {
                continue;
            }

            let Some(mut handler) = RUNTIME.with(|rt| {
                rt.borrow_mut()
                    .key_bindings
                    .get_mut(index)
                    .and_then(|binding| binding.handler.take())
            }) else {
                continue;
            };

            handled = true;
            let propagation = handler(event);
            RUNTIME.with(|rt| {
                if let Some(binding) = rt.borrow_mut().key_bindings.get_mut(index) {
                    binding.handler = Some(handler);
                }
            });
            if propagation == Propagation::Stop {
                return true;
            }
        }
    }
    handled
}

/// Returns `Some(handled)` if chord matching claimed this event (fired a
/// handler, or swallowed it as an in-progress prefix). Returns `None` if
/// chords don't apply this time — either because nothing matches, or
/// because a strictly-more-specific (or equally specific) regular
/// `key_bindings` handler exists and should get the event instead.
///
/// `key_depth` is the depth of the deepest matching `key_bindings` target
/// for this event (see `peek_key_binding_target_depth`), computed by the
/// caller so both systems can be compared before either one commits to
/// handling the event.
fn try_dispatch_chord(event: KeyEvent, key_depth: Option<usize>) -> Option<bool> {
    let now = Instant::now();

    let (has_chords, mut pending) = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let timed_out = rt
            .chord_last_event_at
            .map(|t| now.duration_since(t) > CHORD_TIMEOUT)
            .unwrap_or(false);
        if timed_out {
            rt.chord_pending.clear();
        }
        let has_chords = !rt.chord_bindings.is_empty();
        let pending = std::mem::take(&mut rt.chord_pending);
        (has_chords, pending)
    });

    if !has_chords && pending.is_empty() {
        return None;
    }

    let had_prior_pending = !pending.is_empty();
    pending.push(event);

    // Collect (index, component_id, depth) for every binding that fully
    // matches, same shape dispatch_key uses for key_bindings. Also track
    // the deepest component involved in a *partial* match, so we can
    // compare against key_depth before committing to swallowing the event.
    let (mut full_matches, partial_depth): (Vec<(usize, u64, usize)>, Option<usize>) = RUNTIME
        .with(|rt| {
            let rt = rt.borrow();
            let mut full_matches = Vec::new();
            let mut partial_depth: Option<usize> = None;
            for (i, binding) in rt.chord_bindings.iter().enumerate() {
                if !is_on_focus_path(&rt, binding.component_id) {
                    continue;
                }
                let depth = component_depth(&rt, binding.component_id);
                for alt in binding.alternatives {
                    if pending.len() > alt.len() {
                        continue;
                    }
                    let step_ok = pending.iter().zip(*alt).all(|(e, s)| s.matches(e));
                    if !step_ok {
                        continue;
                    }
                    if pending.len() == alt.len() {
                        full_matches.push((i, binding.component_id, depth));
                    } else {
                        partial_depth = Some(partial_depth.map_or(depth, |d: usize| d.max(depth)));
                    }
                }
            }
            (full_matches, partial_depth)
        });

    // --- Full match: fire it, unless a strictly-more-specific (or
    // equally specific) key_bindings handler exists — in which case defer
    // to that instead. Either way the sequence is over, so clear pending.
    if !full_matches.is_empty() {
        let chord_target_depth = full_matches.iter().map(|(_, _, d)| *d).max().unwrap_or(0);
        let key_wins = key_depth.is_some_and(|depth| depth >= chord_target_depth);

        pending.clear();
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            rt.chord_pending = pending;
            rt.chord_last_event_at = None;
        });

        if key_wins {
            return None;
        }

        // Deepest matching component is the target; only its own ancestry
        // chain gets a chance to fire, deepest first — mirrors dispatch_key.
        let target_id = full_matches
            .iter()
            .max_by_key(|(_, _, depth)| *depth)
            .map(|(_, id, _)| *id)
            .unwrap();

        let ancestry_path = RUNTIME.with(|rt| component_ancestry(&rt.borrow(), target_id));

        full_matches.retain(|(_, comp_id, _)| ancestry_path.contains(comp_id));
        full_matches.sort_by_key(|binding| std::cmp::Reverse(binding.2));

        for (i, _, _) in full_matches {
            let Some(mut handler) = RUNTIME.with(|rt| {
                rt.borrow_mut()
                    .chord_bindings
                    .get_mut(i)
                    .and_then(|b| b.handler.take())
            }) else {
                continue;
            };

            let propagation = handler();

            RUNTIME.with(|rt| {
                if let Some(b) = rt.borrow_mut().chord_bindings.get_mut(i) {
                    b.handler = Some(handler);
                }
            });

            if propagation == Propagation::Stop {
                break;
            }
        }

        return Some(true);
    }

    // --- Partial match only (sequence still in progress): swallow the
    // event and wait for the next key, unless key_bindings has an
    // equally-or-more-specific handler for this exact event — in which
    // case leave the chord's pending state completely untouched and let
    // key_bindings handle it.
    if let Some(depth) = partial_depth {
        let key_wins = key_depth.is_some_and(|key_depth| key_depth >= depth);
        if key_wins {
            pending.pop();
            RUNTIME.with(|rt| {
                rt.borrow_mut().chord_pending = pending;
            });
            return None;
        }

        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            rt.chord_pending = pending;
            rt.chord_last_event_at = Some(now);
        });
        return Some(true);
    }

    pending.clear();
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        rt.chord_pending = pending;
        rt.chord_last_event_at = None;
    });

    if had_prior_pending {
        return try_dispatch_chord(event, key_depth);
    }
    None
}
#[doc(hidden)]
#[must_use]
pub struct ComponentGuard {
    active: bool,
}

impl Drop for ComponentGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            rt.id_stack.pop();
            rt.sibling_counters.pop();
        });
    }
}

/// Entered automatically by `#[component]` — pushes a stable id for
/// this call, derived from the parent id, the function name, and this
/// call's position among its siblings.
#[doc(hidden)]
pub fn __enter_component(name: &'static str) -> ComponentGuard {
    let active = CURRENT_RUNTIME.with(|stack| !stack.borrow().is_empty());
    if !active {
        return ComponentGuard { active: false };
    }

    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let parent_component = rt.id_stack.last().copied();
        let parent = parent_component.unwrap_or(0);
        let sibling_index = match rt.sibling_counters.last_mut() {
            Some(counter) => {
                let value = *counter;
                *counter += 1;
                value
            }
            None => 0,
        };

        let pending_key = rt.pending_component_keys.last().copied();
        let mut id = match pending_key {
            Some(PendingComponentKey {
                hash: key,
                explicit: true,
            }) => component_id(parent, name, key as u32) ^ key.rotate_left(23),
            Some(PendingComponentKey {
                hash: callsite,
                explicit: false,
            }) => component_id(parent, name, callsite as u32) ^ callsite.rotate_left(23),
            None => component_id(parent, name, sibling_index),
        };

        let mut inserted = rt.seen_components.insert(id);
        if !inserted
            && let Some(PendingComponentKey {
                hash: callsite,
                explicit: false,
            }) = pending_key
        {
            loop {
                let occurrence = rt
                    .component_callsite_counts
                    .entry((parent, callsite))
                    .or_insert(1);
                id = component_id(parent, name, *occurrence) ^ callsite.rotate_left(23);
                *occurrence += 1;
                inserted = rt.seen_components.insert(id);
                if inserted {
                    break;
                }
            }
        }

        if !inserted {
            if pending_key.is_some_and(|key| key.explicit) {
                panic!("duplicate explicit key for component `{name}` under the same parent");
            }
            panic!("component identity collision for `{name}`");
        }
        rt.id_stack.push(id);
        rt.sibling_counters.push(0);

        // Always update the parent: keyed components may move between frames.
        rt.component_parents.insert(id, parent_component);
    });
    ComponentGuard { active: true }
}

#[doc(hidden)]
pub fn __with_component_key<K: Hash, R>(key: &K, f: impl FnOnce() -> R) -> R {
    with_component_key(key, true, f)
}

#[doc(hidden)]
pub fn __with_component_callsite<K: Hash, R>(key: &K, f: impl FnOnce() -> R) -> R {
    with_component_key(key, false, f)
}

fn with_component_key<K: Hash, R>(key: &K, explicit: bool, f: impl FnOnce() -> R) -> R {
    struct KeyGuard {
        active: bool,
    }
    impl Drop for KeyGuard {
        fn drop(&mut self) {
            if !self.active {
                return;
            }
            RUNTIME.with(|rt| {
                rt.borrow_mut().pending_component_keys.pop();
            });
        }
    }

    let active = CURRENT_RUNTIME.with(|stack| !stack.borrow().is_empty());
    if !active {
        return f();
    }

    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    RUNTIME.with(|rt| {
        rt.borrow_mut()
            .pending_component_keys
            .push(PendingComponentKey {
                hash: hasher.finish(),
                explicit,
            });
    });
    let _guard = KeyGuard { active: true };
    f()
}

#[doc(hidden)]
pub fn __current_component_id() -> u64 {
    RUNTIME.with(|rt| {
        *rt.borrow()
            .id_stack
            .last()
            .expect("node behavior used outside a #[component] function")
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusHandle {
    component_id: u64,
}

impl FocusHandle {
    pub fn focus(self) {
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            if rt.focused_component != Some(self.component_id) {
                rt.focused_component = Some(self.component_id);
                drop(rt);
                RUNTIME.request_redraw();
            }
        });
    }

    pub fn is_focused(self) -> bool {
        RUNTIME.with(|rt| rt.borrow().focused_component == Some(self.component_id))
    }
}

pub fn use_focus(active: bool) -> FocusHandle {
    let component_id = __current_component_id();
    let handle = FocusHandle { component_id };
    if active {
        RUNTIME.with(|rt| {
            rt.borrow_mut().focused_component = Some(component_id);
        });
    }
    handle
}

fn component_id(parent: u64, name: &'static str, sibling_index: u32) -> u64 {
    let mut hasher = DefaultHasher::new();
    parent.hash(&mut hasher);
    name.hash(&mut hasher);
    sibling_index.hash(&mut hasher);
    hasher.finish()
}

struct StateCell<T> {
    value: RefCell<T>,
    version: Cell<u64>,
    redraw: RedrawHandle,
}

/// A typed handle to persistent state.
///
/// Access is a direct `Rc` + `RefCell` borrow. The runtime's erased hook store
/// is consulted only when this handle is created during a render.
pub struct State<T> {
    cell: Rc<StateCell<T>>,
}

impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        Self {
            cell: self.cell.clone(),
        }
    }
}

impl<T: 'static> State<T> {
    fn new(value: T, redraw: RedrawHandle) -> Self {
        Self {
            cell: Rc::new(StateCell {
                value: RefCell::new(value),
                version: Cell::new(0),
                redraw,
            }),
        }
    }

    fn version(&self) -> u64 {
        self.cell.version.get()
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.cell.value.borrow())
    }

    pub fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let result = {
            let mut borrow = self.cell.value.borrow_mut();
            f(&mut borrow)
        };
        self.cell.version.set(self.version().wrapping_add(1));
        self.cell.redraw.request();
        result
    }

    pub fn with_mut_untracked<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        f(&mut self.cell.value.borrow_mut())
    }

    pub fn set(&self, value: T) {
        *self.cell.value.borrow_mut() = value;
        self.cell.version.set(self.version().wrapping_add(1));
        self.cell.redraw.request();
    }

    pub fn update<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        self.with_mut(f)
    }
}

impl<T: 'static + Clone> State<T> {
    pub fn get(&self) -> T {
        self.with(Clone::clone)
    }
}

impl<T: 'static + PartialEq> State<T> {
    pub fn set_if_changed(&self, value: T) -> bool {
        let mut current = self.cell.value.borrow_mut();
        if *current == value {
            return false;
        }
        *current = value;
        drop(current);
        self.cell.version.set(self.version().wrapping_add(1));
        self.cell.redraw.request();
        true
    }
}

/// Persistent mutable storage that does not schedule a redraw when changed.
pub struct Stored<T>(Rc<RefCell<T>>);

impl<T> Clone for Stored<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Stored<T> {
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.0.borrow())
    }

    pub fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        f(&mut self.0.borrow_mut())
    }
}

impl<T: Clone> Stored<T> {
    pub fn get(&self) -> T {
        self.with(Clone::clone)
    }
}

pub struct Memo<T>(Rc<T>);

impl<T> Clone for Memo<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Deref for Memo<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct DependencyMemo<D, T> {
    deps: D,
    value: Memo<T>,
}

type Cleanup = Box<dyn FnOnce()>;

struct EffectSlot<D> {
    deps: D,
    cleanup: Rc<RefCell<Option<Cleanup>>>,
}

impl<D> Drop for EffectSlot<D> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.borrow_mut().take() {
            cleanup();
        }
    }
}

#[track_caller]
fn hook_site() -> HookSite {
    let location = std::panic::Location::caller();
    (location.file(), location.line(), location.column())
}

fn current_redraw() -> RedrawHandle {
    CURRENT_RUNTIME.with(|stack| {
        stack
            .borrow()
            .last()
            .expect("hook used outside Runtime::render")
            .redraw
            .clone()
    })
}

/// Persistent state scoped to this component and source call site.
///
/// Conditional hooks are supported, but a hook call site may execute at most
/// once in one component render. State at a skipped call site is discarded.
#[track_caller]
pub fn state<T: 'static>(init: impl FnOnce() -> T) -> State<T> {
    let site = hook_site();
    state_at(site, init)
}

#[track_caller]
pub fn use_state<T: 'static>(init: impl FnOnce() -> T) -> State<T> {
    state(init)
}

#[track_caller]
pub fn use_ref<T: 'static>(init: impl FnOnce() -> T) -> Stored<T> {
    let site = hook_site();
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let component_id = *rt
            .id_stack
            .last()
            .expect("use_ref() called outside of a #[component] function");
        let key = (component_id, site);
        assert!(rt.seen_state_keys.insert(key), "hook call site used twice");
        rt.states
            .entry(key)
            .or_insert_with(|| Box::new(Stored(Rc::new(RefCell::new(init())))))
            .downcast_ref::<Stored<T>>()
            .unwrap_or_else(|| panic!("hook type changed at {}:{}:{}", site.0, site.1, site.2))
            .clone()
    })
}

#[track_caller]
pub fn use_memo<D, T>(deps: D, compute: impl FnOnce() -> T) -> Memo<T>
where
    D: PartialEq + 'static,
    T: 'static,
{
    let site = hook_site();
    let mut compute = Some(compute);
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let component_id = *rt
            .id_stack
            .last()
            .expect("use_memo() called outside of a #[component] function");
        let key = (component_id, site);
        assert!(rt.seen_state_keys.insert(key), "hook call site used twice");
        if let Some(entry) = rt.states.get_mut(&key) {
            let entry = entry
                .downcast_mut::<DependencyMemo<D, T>>()
                .unwrap_or_else(|| panic!("memo type changed at {}:{}:{}", site.0, site.1, site.2));
            if entry.deps != deps {
                entry.deps = deps;
                entry.value = Memo(Rc::new(compute.take().expect("memo compute consumed")()));
            }
            entry.value.clone()
        } else {
            let value = Memo(Rc::new(compute.take().expect("memo compute consumed")()));
            rt.states.insert(
                key,
                Box::new(DependencyMemo {
                    deps,
                    value: value.clone(),
                }),
            );
            value
        }
    })
}

#[track_caller]
pub fn use_effect<D, F, C>(deps: D, effect: F)
where
    D: PartialEq + 'static,
    F: FnOnce() -> C + 'static,
    C: FnOnce() + 'static,
{
    let site = hook_site();
    let mut effect = Some(effect);
    let cleanup_to_run = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let component_id = *rt
            .id_stack
            .last()
            .expect("use_effect() called outside of a #[component] function");
        let key = (component_id, site);
        assert!(rt.seen_state_keys.insert(key), "hook call site used twice");

        let mut schedule = None;
        let mut cleanup_to_run = None;
        match rt.states.get_mut(&key) {
            Some(value) => {
                let slot = value.downcast_mut::<EffectSlot<D>>().unwrap_or_else(|| {
                    panic!("effect type changed at {}:{}:{}", site.0, site.1, site.2)
                });
                if slot.deps != deps {
                    cleanup_to_run = slot.cleanup.borrow_mut().take();
                    slot.deps = deps;
                    schedule = Some(slot.cleanup.clone());
                }
            }
            None => {
                let cleanup = Rc::new(RefCell::new(None));
                schedule = Some(cleanup.clone());
                rt.states
                    .insert(key, Box::new(EffectSlot { deps, cleanup }));
            }
        }

        if let Some(cleanup) = schedule {
            let effect = effect.take().expect("effect consumed");
            rt.pending_effects.push(Box::new(move || {
                *cleanup.borrow_mut() = Some(Box::new(effect()));
            }));
        }
        cleanup_to_run
    });
    if let Some(cleanup) = cleanup_to_run {
        cleanup();
    }
}

fn state_at<T: 'static>(site: HookSite, init: impl FnOnce() -> T) -> State<T> {
    let redraw = current_redraw();
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let component_id = *rt
            .id_stack
            .last()
            .expect("state() called outside of a #[component] function");
        let key = (component_id, site);
        assert!(
            rt.seen_state_keys.insert(key),
            "hook at {}:{}:{} was called more than once in one component render",
            site.0,
            site.1,
            site.2
        );
        rt.states
            .entry(key)
            .or_insert_with(|| Box::new(State::new(init(), redraw)))
            .downcast_ref::<State<T>>()
            .unwrap_or_else(|| panic!("state type changed at {}:{}:{}", site.0, site.1, site.2))
            .clone()
    })
}

/// A handle the current component can use to register key bindings
/// for this frame. Registration happens every render (mirroring how
/// the whole node tree is rebuilt every render).
#[derive(Clone, Copy)]
pub struct KeyHandle;

impl KeyHandle {
    pub fn on(&self, code: KeyCode, mut handler: impl FnMut() -> Propagation + 'static) {
        self.on_when(
            move |event| event.code == code && event.modifiers == KeyModifiers::NONE,
            move |_| handler(),
        );
    }

    pub fn on_modified(
        &self,
        code: KeyCode,
        modifiers: KeyModifiers,
        mut handler: impl FnMut() -> Propagation + 'static,
    ) {
        self.on_when(
            move |event| event.code == code && event.modifiers == modifiers,
            move |_| handler(),
        );
    }

    pub fn on_any(&self, handler: impl FnMut(KeyEvent) -> Propagation + 'static) {
        self.on_when(|_| true, handler);
    }

    pub fn on_when(
        &self,
        matches: impl Fn(&KeyEvent) -> bool + 'static,
        handler: impl FnMut(KeyEvent) -> Propagation + 'static,
    ) {
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            let component_id = *rt
                .id_stack
                .last()
                .expect("on_when() called outside of a #[component] function");
            rt.key_bindings.push(KeyBinding {
                component_id,
                matches: Box::new(matches),
                handler: Some(Box::new(handler)),
            });
        });
    }

    pub fn on_chord(
        &self,
        alternatives: &'static [&'static [ParsedKeySpec]],
        handler: impl FnMut() -> Propagation + 'static,
    ) {
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            let component_id = *rt
                .id_stack
                .last()
                .expect("on_chord() called outside of a #[component] function");
            rt.chord_bindings.push(ChordBinding {
                component_id,
                alternatives,
                handler: Some(Box::new(handler)),
            });
        });
    }
}

pub fn use_key() -> KeyHandle {
    RUNTIME.with(|rt| {
        rt.borrow()
            .id_stack
            .last()
            .expect("use_key() called outside of a #[component] function");
    });
    KeyHandle
}
