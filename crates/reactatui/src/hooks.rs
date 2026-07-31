use std::any::Any;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;

use crate::keys::ParsedKeySpec;

type AnyCell = Rc<RefCell<Box<dyn Any>>>;

struct KeyBinding {
    component_id: u64,
    matches: Box<dyn Fn(&KeyEvent) -> bool>,
    handler: Option<Box<dyn FnMut(KeyEvent) -> Propagation>>,
}

const CHORD_TIMEOUT: Duration = Duration::from_millis(1000);

struct ChordBinding {
    // Each inner Vec is one alternative full sequence, e.g. `"g-g" | "home"`
    // registers two alternatives, lengths 2 and 1.
    alternatives: Vec<Vec<ParsedKeySpec>>,
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

struct EventListener {
    /// The component that called `listen`.
    component_id: u64,
    /// Position of `component_id` inside the emitter's ancestry path
    /// (filled in at dispatch time for ordering).
    event_name: &'static str,
    handler: Option<Box<dyn FnMut(&dyn Any) -> Propagation>>,
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
    click_handler: Option<Box<dyn FnMut(MouseButton) -> Propagation>>,
    mousein_handler: Option<Box<dyn FnMut()>>,
    mouseout_handler: Option<Box<dyn FnMut()>>,
    scrollx_handler: Option<Box<dyn FnMut(i16) -> Propagation>>,
    scrolly_handler: Option<Box<dyn FnMut(i16) -> Propagation>>,
}

#[derive(Default)]
struct HookRuntime {
    id_stack: Vec<u64>,
    sibling_counters: Vec<u32>,
    hook_indices: Vec<u32>,
    states: HashMap<(u64, u32), AnyCell>,
    keyed_states: HashMap<&'static str, AnyCell>,
    key_bindings: Vec<KeyBinding>,
    /// Per-frame event listeners, re-registered every render (like key_bindings).
    event_listeners: Vec<EventListener>,
    /// Per-frame mouse regions — rebuilt every render.
    mouse_regions: Vec<MouseRegion>,
    /// Which region ids were hovered last frame (for hover-enter tracking).
    prev_hovered: HashSet<u64>,
    /// Maps a component ID to its ancestry path (the id_stack at the time of entry).
    component_paths: HashMap<u64, Rc<Vec<u64>>>,
    /// Bumped every time a `State` is mutated via `set`/`with_mut`.
    /// Read by `memo` to decide whether to recompute.
    versions: HashMap<StateKey, u64>,
    chord_bindings: Vec<ChordBinding>,
    chord_pending: Vec<KeyEvent>,
    chord_last_event_at: Option<Instant>,
    /// Per-component count of mouse regions registered this frame — used
    /// to generate stable region IDs that survive layout shifts.
    mouse_region_idx: HashMap<u64, u32>,
}

thread_local! {
    static RUNTIME: RefCell<HookRuntime> = RefCell::new(HookRuntime::default());
}

/// Call once per frame, before building the component tree, to reset
/// per-frame bookkeeping. `state` values are untouched — they
/// persist for the lifetime of the process (or until you drop the app).
pub fn begin_frame() {
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        debug_assert!(
            rt.id_stack.is_empty(),
            "begin_frame() called while a #[component] is still on the stack"
        );
        rt.key_bindings.clear();
        rt.event_listeners.clear();
        // mouse_regions are cleared here; prev_hovered is preserved across frames
        // so hover-enter detection works.
        rt.mouse_regions.clear();
        rt.chord_bindings.clear();
        // Reset per-component mouse region counters for the new frame.
        rt.mouse_region_idx.clear();
    });
}

/// Called from macro-generated render closures to record that an
/// interactive element occupies `rect` this frame.
///
/// **Click and scroll handlers now return `Propagation`** so that bubbling
/// can be stopped at the appropriate layer.
#[doc(hidden)]
pub fn register_mouse_region(
    rect: Rect,
    click_handler: Option<Box<dyn FnMut(MouseButton) -> Propagation>>,
    mousein_handler: Option<Box<dyn FnMut()>>,
    mouseout_handler: Option<Box<dyn FnMut()>>,
    scrollx_handler: Option<Box<dyn FnMut(i16) -> Propagation>>,
    scrolly_handler: Option<Box<dyn FnMut(i16) -> Propagation>>,
) {
    // Derive a stable ID from (component_id, per-component registration count).
    // This survives layout shifts (window resize, scroll) because it doesn't
    // encode absolute screen coordinates.
    let id = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let comp = *rt.id_stack.last().unwrap_or(&0);
        let local_idx = {
            let entry = rt.mouse_region_idx.entry(comp).or_insert(0);
            let val = *entry;
            *entry += 1;
            val
        };
        let mut hasher = DefaultHasher::new();
        comp.hash(&mut hasher);
        local_idx.hash(&mut hasher);
        hasher.finish()
    });

    RUNTIME.with(|rt| {
        rt.borrow_mut().mouse_regions.push(MouseRegion {
            rect,
            id,
            click_handler,
            mousein_handler,
            mouseout_handler,
            scrollx_handler,
            scrolly_handler,
        });
    });
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

#[inline]
fn rect_area(r: Rect) -> u32 {
    u32::from(r.width) * u32::from(r.height)
}

/// Returns true when `outer` completely contains `inner` (not necessarily
/// a *strict* superset — equal rects pass too).
#[inline]
fn rect_contains_rect(outer: Rect, inner: Rect) -> bool {
    outer.x <= inner.x
        && outer.y <= inner.y
        && outer.x.saturating_add(outer.width) >= inner.x.saturating_add(inner.width)
        && outer.y.saturating_add(outer.height) >= inner.y.saturating_add(inner.height)
}

/// Collect mouse-region indices on the bubbling path for the point `(col, row)`.
///
/// The "target" is the smallest-area region containing the point (innermost
/// child in a correctly-laid-out tree). Only regions that fully contain the
/// target's rect are considered ancestors and included in the path.
/// Result is sorted innermost-first (ascending area) — the natural bubbling
/// order for DOM-style event propagation.
///
/// `has_handler` filters to only regions that have a relevant handler set.
fn bubbling_path(col: u16, row: u16, has_handler: impl Fn(&MouseRegion) -> bool) -> Vec<usize> {
    RUNTIME.with(|rt| {
        let rt = rt.borrow();

        let mut candidates: Vec<(usize, u32)> = rt
            .mouse_regions
            .iter()
            .enumerate()
            .filter(|(_, r)| contains(r.rect, col, row) && has_handler(r))
            .map(|(i, r)| (i, rect_area(r.rect)))
            .collect();

        if candidates.is_empty() {
            return Vec::new();
        }

        // Smallest area = innermost = target
        candidates.sort_by_key(|(_, area)| *area);
        let target_rect = rt.mouse_regions[candidates[0].0].rect;

        // Keep target + any region whose rect contains the target rect (true ancestors)
        candidates
            .into_iter()
            .filter(|(i, _)| rect_contains_rect(rt.mouse_regions[*i].rect, target_rect))
            .map(|(i, _)| i)
            .collect()
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
pub fn dispatch_mouse(event: MouseEvent) {
    let col = event.column;
    let row = event.row;

    match event.kind {
        MouseEventKind::Down(button) => {
            dispatch_mouse_click(col, row, button);
        }
        MouseEventKind::ScrollUp => {
            dispatch_mouse_scroll_y(col, row, -1);
        }
        MouseEventKind::ScrollDown => {
            dispatch_mouse_scroll_y(col, row, 1);
        }
        MouseEventKind::ScrollLeft => {
            dispatch_mouse_scroll_x(col, row, -1);
        }
        MouseEventKind::ScrollRight => {
            dispatch_mouse_scroll_x(col, row, 1);
        }
        MouseEventKind::Moved => {
            dispatch_mouse_move(col, row);
        }
        _ => {}
    }
}

fn dispatch_mouse_click(col: u16, row: u16, button: MouseButton) {
    let indices = bubbling_path(col, row, |r| r.click_handler.is_some());
    for i in indices {
        let handler_opt = RUNTIME.with(|rt| {
            rt.borrow_mut()
                .mouse_regions
                .get_mut(i)
                .and_then(|r| r.click_handler.take())
        });
        if let Some(mut handler) = handler_opt {
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
}

fn dispatch_mouse_scroll_y(col: u16, row: u16, delta: i16) {
    let indices = bubbling_path(col, row, |r| r.scrolly_handler.is_some());
    for i in indices {
        let handler_opt = RUNTIME.with(|rt| {
            rt.borrow_mut()
                .mouse_regions
                .get_mut(i)
                .and_then(|r| r.scrolly_handler.take())
        });
        if let Some(mut handler) = handler_opt {
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
}

fn dispatch_mouse_scroll_x(col: u16, row: u16, delta: i16) {
    let indices = bubbling_path(col, row, |r| r.scrollx_handler.is_some());
    for i in indices {
        let handler_opt = RUNTIME.with(|rt| {
            rt.borrow_mut()
                .mouse_regions
                .get_mut(i)
                .and_then(|r| r.scrollx_handler.take())
        });
        if let Some(mut handler) = handler_opt {
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
}

fn dispatch_mouse_move(col: u16, row: u16) {
    let mut mousein_handlers_to_run = Vec::new();
    let mut mouseout_handlers_to_run = Vec::new();

    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let prev = rt.prev_hovered.clone();
        let mut now_hovered = HashSet::new();

        for region in rt.mouse_regions.iter_mut() {
            if contains(region.rect, col, row) {
                now_hovered.insert(region.id);
                // Fire on:mousein only when the cursor first enters.
                if !prev.contains(&region.id) {
                    if let Some(handler) = region.mousein_handler.take() {
                        mousein_handlers_to_run.push(handler);
                    }
                }
            } else if prev.contains(&region.id) {
                // Fire on:mouseout when the cursor leaves.
                if let Some(handler) = region.mouseout_handler.take() {
                    mouseout_handlers_to_run.push(handler);
                }
            }
        }
        rt.prev_hovered = now_hovered;
    });

    for mut handler in mousein_handlers_to_run {
        handler();
    }
    for mut handler in mouseout_handlers_to_run {
        handler();
    }
}

#[inline]
fn contains(rect: Rect, col: u16, row: u16) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

/// Run every handler registered this frame whose binding matches.
/// Returns `true` if at least one handler fired.
pub fn dispatch_key(event: KeyEvent) -> bool {
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

/// Read-only lookup of the deepest `key_bindings` component whose matcher
/// fires for `event`, without taking or running any handler. Used only to
/// compare against chord candidates so the two systems can agree on who
/// gets the event before either commits to handling it.
fn peek_key_binding_target_depth(event: KeyEvent) -> Option<usize> {
    RUNTIME.with(|rt| {
        let rt = rt.borrow();
        rt.key_bindings
            .iter()
            .filter(|binding| binding.handler.is_some() && (binding.matches)(&event))
            .map(|binding| {
                rt.component_paths
                    .get(&binding.component_id)
                    .map(|p| p.len())
                    .unwrap_or(0)
            })
            .max()
    })
}

/// The regular (non-chord) dispatch path — this is the old body of
/// `dispatch_key`, split out so `dispatch_key` can decide whether chords
/// or plain bindings get first crack at an event.
fn dispatch_key_bindings(event: KeyEvent) -> bool {
    let mut matches: Vec<(usize, u64, usize)> = RUNTIME.with(|rt| {
        let rt = rt.borrow();
        rt.key_bindings
            .iter()
            .enumerate()
            .filter_map(|(i, binding)| {
                if binding.handler.is_some() && (binding.matches)(&event) {
                    let depth = rt
                        .component_paths
                        .get(&binding.component_id)
                        .map(|p| p.len())
                        .unwrap_or(0);
                    Some((i, binding.component_id, depth))
                } else {
                    None
                }
            })
            .collect()
    });

    if matches.is_empty() {
        return false;
    }

    // Deepest matching component is the "target" — the thing the key was
    // really meant for (e.g. the focused widget).
    let target_id = matches
        .iter()
        .max_by_key(|(_, _, depth)| *depth)
        .map(|(_, id, _)| *id)
        .unwrap();

    let ancestry_path = RUNTIME
        .with(|rt| rt.borrow().component_paths.get(&target_id).cloned())
        .unwrap_or_default();

    // Only bindings on target's own ancestry chain get to bubble;
    // unrelated matches elsewhere in the tree don't fire.
    matches.retain(|(_, comp_id, _)| ancestry_path.contains(comp_id));
    matches.sort_by(|a, b| b.2.cmp(&a.2)); // deepest first

    let indices: Vec<usize> = matches.into_iter().map(|(i, _, _)| i).collect();
    let handled = !indices.is_empty();

    for i in indices {
        let Some(mut handler) = RUNTIME.with(|rt| {
            rt.borrow_mut()
                .key_bindings
                .get_mut(i)
                .and_then(|binding| binding.handler.take())
        }) else {
            continue;
        };

        let propagation = handler(event);

        RUNTIME.with(|rt| {
            if let Some(binding) = rt.borrow_mut().key_bindings.get_mut(i) {
                binding.handler = Some(handler);
            }
        });

        if propagation == Propagation::Stop {
            break;
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
        (!rt.chord_bindings.is_empty(), rt.chord_pending.clone())
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
                let depth = rt
                    .component_paths
                    .get(&binding.component_id)
                    .map(|p| p.len())
                    .unwrap_or(0);
                for alt in &binding.alternatives {
                    if pending.len() > alt.len() {
                        continue;
                    }
                    let step_ok = pending.iter().zip(alt).all(|(e, s)| s.matches(e));
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
        let key_wins = key_depth.map_or(false, |kd| kd >= chord_target_depth);

        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            rt.chord_pending.clear();
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

        let ancestry_path = RUNTIME
            .with(|rt| rt.borrow().component_paths.get(&target_id).cloned())
            .unwrap_or_default();

        full_matches.retain(|(_, comp_id, _)| ancestry_path.contains(comp_id));
        full_matches.sort_by(|a, b| b.2.cmp(&a.2)); // deepest first

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
        let key_wins = key_depth.map_or(false, |kd| kd >= depth);
        if key_wins {
            return None;
        }

        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            rt.chord_pending = pending;
            rt.chord_last_event_at = Some(now);
        });
        return Some(true);
    }

    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        rt.chord_pending.clear();
        rt.chord_last_event_at = None;
    });

    if had_prior_pending {
        return try_dispatch_chord(event, key_depth);
    }
    None
}
#[doc(hidden)]
#[must_use]
pub struct ComponentGuard;

impl Drop for ComponentGuard {
    fn drop(&mut self) {
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            rt.id_stack.pop();
            rt.sibling_counters.pop();
            rt.hook_indices.pop();
        });
    }
}

/// Entered automatically by `#[component]` — pushes a stable id for
/// this call, derived from the parent id, the function name, and this
/// call's position among its siblings.
#[doc(hidden)]
pub fn __enter_component(name: &'static str) -> ComponentGuard {
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let parent = *rt.id_stack.last().unwrap_or(&0);
        let sibling_index = match rt.sibling_counters.last_mut() {
            Some(counter) => {
                let value = *counter;
                *counter += 1;
                value
            }
            None => 0,
        };

        let id = component_id(parent, name, sibling_index);

        rt.id_stack.push(id);
        rt.sibling_counters.push(0);
        rt.hook_indices.push(0);

        // Always update the ancestry path — the component may have moved
        // in the tree (e.g. inside a conditional) since the last frame.
        let path_clone = Rc::new(rt.id_stack.clone());
        rt.component_paths.insert(id, path_clone);
    });
    ComponentGuard
}

#[doc(hidden)]
pub fn __next_component_id(name: &'static str) -> u64 {
    RUNTIME.with(|rt| {
        let rt = rt.borrow();
        let parent = *rt.id_stack.last().unwrap_or(&0);
        let sibling_index = *rt.sibling_counters.last().unwrap_or(&0);
        component_id(parent, name, sibling_index)
    })
}

fn component_id(parent: u64, name: &'static str, sibling_index: u32) -> u64 {
    let mut hasher = DefaultHasher::new();
    parent.hash(&mut hasher);
    name.hash(&mut hasher);
    sibling_index.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum StateKey {
    Positional(u64, u32),
    Keyed(&'static str),
}

/// A handle to a persistent value. Cheap to clone (it's a shared
/// pointer into the hook store), so it's fine to move copies into
/// `use_key` closures.
pub struct State<T> {
    key: StateKey,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for State<T> {}

impl<T: 'static> State<T> {
    fn get_cell(&self) -> AnyCell {
        RUNTIME.with(|rt| {
            let rt = rt.borrow();
            match self.key {
                StateKey::Positional(comp, idx) => rt
                    .states
                    .get(&(comp, idx))
                    .expect("state not found")
                    .clone(),
                StateKey::Keyed(k) => rt.keyed_states.get(k).expect("state not found").clone(),
            }
        })
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let cell = self.get_cell();
        let borrow = cell.borrow();
        f(borrow.downcast_ref::<T>().expect("state type mismatch"))
    }

    pub fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let cell = self.get_cell();
        let result = {
            let mut borrow = cell.borrow_mut();
            f(borrow.downcast_mut::<T>().expect("state type mismatch"))
        };
        RUNTIME.with(|rt| {
            *rt.borrow_mut().versions.entry(self.key).or_insert(0) += 1;
        });
        result
    }

    pub fn set(&self, value: T) {
        let cell = self.get_cell();
        *cell.borrow_mut() = Box::new(value);
        RUNTIME.with(|rt| {
            *rt.borrow_mut().versions.entry(self.key).or_insert(0) += 1;
        });
    }
}

impl<T: 'static + Clone> State<T> {
    pub fn get(&self) -> T {
        self.with(Clone::clone)
    }
}

/// Persistent state scoped to *this component's position in the tree*.
/// Must be called unconditionally, in the same order, every render —
/// same rule as React hooks.
pub fn state<T: 'static>(init: impl FnOnce() -> T) -> State<T> {
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let component_id = *rt
            .id_stack
            .last()
            .expect("state() called outside of a #[component] function");
        let index = {
            let counter = rt
                .hook_indices
                .last_mut()
                .expect("hook index stack unexpectedly empty");
            let value = *counter;
            *counter += 1;
            value
        };
        rt.states
            .entry((component_id, index))
            .or_insert_with(|| Rc::new(RefCell::new(Box::new(init()) as Box<dyn Any>)));
        State {
            key: StateKey::Positional(component_id, index),
            _marker: std::marker::PhantomData,
        }
    })
}

/// Access global state that you expect to already exist (initialized by
/// some other `global_or` call earlier in
/// the frame, or in a previous frame). Panics if it hasn't been created yet.
///
/// Use this when you're a "consumer" component that doesn't own the
/// state's lifecycle — e.g. reading a theme that's set up once at the root.
pub fn global<T: 'static>(key: &'static str) -> State<T> {
    RUNTIME.with(|rt| {
        assert!(
            rt.borrow().keyed_states.contains_key(key),
            "global::<{}>(\"{}\") read before it was initialized — \
             call global_or somewhere first",
            std::any::type_name::<T>(),
            key
        );
    });
    State {
        key: StateKey::Keyed(key),
        _marker: std::marker::PhantomData,
    }
}

/// Access global state, constructing it with `init` on first access.
/// Accepts closures (`|| MyStruct { .. }`) and fn items (`MyStruct::new`)
/// equally well since both satisfy `FnOnce() -> T`.
pub fn global_or<T: 'static>(key: &'static str, init: impl FnOnce() -> T) -> State<T> {
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        rt.keyed_states
            .entry(key)
            .or_insert_with(|| Rc::new(RefCell::new(Box::new(init()) as Box<dyn Any>)));
    });
    State {
        key: StateKey::Keyed(key),
        _marker: std::marker::PhantomData,
    }
}

/// Non-panicking existence check, for the rare case where you need to
/// branch on whether the state has been created yet at all.
pub fn try_use_global<T: 'static>(key: &'static str) -> Option<State<T>> {
    let exists = RUNTIME.with(|rt| rt.borrow().keyed_states.contains_key(key));
    exists.then_some(State {
        key: StateKey::Keyed(key),
        _marker: std::marker::PhantomData,
    })
}

/// Derives a value from another `State<T>` and caches it in its own
/// hook slot, recomputed once every render.
///
/// The `compute` closure receives a shared `&T` reference — it must not
/// mutate the source (`state` + explicit mutation for that).
///
/// Must be called unconditionally, in the same order, every render —
/// same rule as `state`.
pub fn computed<T: 'static, R: 'static>(
    source: State<T>,
    compute: impl FnOnce(&T) -> R,
) -> State<R> {
    // Borrow `source` read-only — this does NOT bump the version counter,
    // which is correct since we're only reading.
    let value: R = source.with(compute);

    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let component_id = *rt
            .id_stack
            .last()
            .expect("computed() called outside of a #[component] function");
        let index = {
            let counter = rt
                .hook_indices
                .last_mut()
                .expect("hook index stack unexpectedly empty");
            let idx = *counter;
            *counter += 1;
            idx
        };

        let boxed: Box<dyn Any> = Box::new(value);
        match rt.states.get(&(component_id, index)) {
            Some(cell) => {
                // Overwrite in place — same cell every frame, so the
                // returned State<R> handle stays valid across renders.
                *cell.borrow_mut() = boxed;
            }
            None => {
                rt.states
                    .insert((component_id, index), Rc::new(RefCell::new(boxed)));
            }
        }

        State {
            key: StateKey::Positional(component_id, index),
            _marker: std::marker::PhantomData,
        }
    })
}

/// Derives a value from another `State<T>` and caches it in its own hook
/// slot. The `compute` closure only re-runs when `source` has been mutated
/// (via `set`/`with_mut`).
///
/// The `compute` closure receives a shared `&T` reference — it must not
/// mutate the source (`state` + explicit mutation for that).
///
/// Must be called unconditionally, in the same order, every render — same
/// rule as `state`.
pub fn memo<T: 'static, R: 'static>(source: State<T>, compute: impl FnOnce(&T) -> R) -> State<R> {
    // Claim this hook's slot first (bumps hook_indices like any other hook).
    let (component_id, index) = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let component_id = *rt
            .id_stack
            .last()
            .expect("memo() called outside of a #[component] function");
        let index = {
            let counter = rt
                .hook_indices
                .last_mut()
                .expect("hook index stack unexpectedly empty");
            let idx = *counter;
            *counter += 1;
            idx
        };
        (component_id, index)
    });
    let computed_key = StateKey::Positional(component_id, index);

    let current_version = RUNTIME.with(|rt| *rt.borrow().versions.get(&source.key).unwrap_or(&0));
    let last_seen_version = RUNTIME.with(|rt| rt.borrow().versions.get(&computed_key).copied());
    let exists = RUNTIME.with(|rt| rt.borrow().states.contains_key(&(component_id, index)));

    let needs_recompute = !exists || last_seen_version != Some(current_version);

    if needs_recompute {
        // Read-only access — does NOT bump the source's version counter.
        let value = source.with(compute);

        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            let boxed: Box<dyn Any> = Box::new(value);
            match rt.states.get(&(component_id, index)) {
                Some(cell) => *cell.borrow_mut() = boxed,
                None => {
                    rt.states
                        .insert((component_id, index), Rc::new(RefCell::new(boxed)));
                }
            }
            rt.versions.insert(computed_key, current_version);
        });
    }

    State {
        key: computed_key,
        _marker: std::marker::PhantomData,
    }
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
        alternatives: Vec<Vec<ParsedKeySpec>>,
        mut handler: impl FnMut() -> Propagation + 'static,
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
                handler: Some(Box::new(move || handler())),
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

// -------------------------------------------------------------------------
// Custom events with bubbling
// -------------------------------------------------------------------------

/// A callable handle returned by [`emitter`]. Cheaply cloneable so it can
/// be moved into multiple closures.
///
/// Calling it fires every [`listen`] listener registered for the same event
/// name whose component is an ancestor of (or equal to) the emitting
/// component, in **bubbling order** (closest ancestor first → root last).
pub struct Emitter<T: 'static> {
    event_name: &'static str,
    component_id: u64,
    _marker: std::marker::PhantomData<fn(T)>,
}

impl<T> Clone for Emitter<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Emitter<T> {}

impl<T: 'static> Emitter<T> {
    /// Emit the event, bubbling up through ancestor listeners.
    pub fn emit(&self, data: T) {
        let data_any: Box<dyn Any> = Box::new(data);

        let ancestry_path = RUNTIME
            .with(|rt| rt.borrow().component_paths.get(&self.component_id).cloned())
            .expect("ancestry path not found");

        // Collect matching listener indices ordered by bubbling:
        // highest position in ancestry_path = closest ancestor = fires first.
        let indices: Vec<usize> = RUNTIME.with(|rt| {
            let rt = rt.borrow();
            let mut matching: Vec<(usize, usize)> = rt
                .event_listeners
                .iter()
                .enumerate()
                .filter_map(|(i, listener)| {
                    if listener.event_name != self.event_name {
                        return None;
                    }
                    // rposition: last occurrence gives the deepest (closest) match.
                    ancestry_path
                        .iter()
                        .rposition(|&id| id == listener.component_id)
                        .map(|depth| (i, depth))
                })
                .collect();
            // Descending by depth → closest ancestor fires first (bubbling).
            matching.sort_by(|a, b| b.1.cmp(&a.1));
            matching.into_iter().map(|(i, _)| i).collect()
        });

        for i in indices {
            let Some(mut handler) = RUNTIME.with(|rt| {
                rt.borrow_mut()
                    .event_listeners
                    .get_mut(i)
                    .and_then(|listener| listener.handler.take())
            }) else {
                continue;
            };

            let propagation = handler(data_any.as_ref());

            RUNTIME.with(|rt| {
                if let Some(listener) = rt.borrow_mut().event_listeners.get_mut(i) {
                    listener.handler = Some(handler);
                }
            });

            if propagation == Propagation::Stop {
                break;
            }
        }
    }

    /// Emit the event **globally** — every registered [`listen`] listener for
    /// this event name is called, regardless of whether it is an ancestor of
    /// the emitting component. Listeners fire in registration order.
    ///
    /// Use sparingly; prefer [`emit`](Self::emit) for normal parent-child
    /// communication and reserve `emit_global` for app-wide broadcasts
    /// (e.g. theme changes, global notifications).
    pub fn emit_global(&self, data: T) {
        let data_any: Box<dyn Any> = Box::new(data);

        let indices: Vec<usize> = RUNTIME.with(|rt| {
            rt.borrow()
                .event_listeners
                .iter()
                .enumerate()
                .filter_map(|(i, listener)| {
                    if listener.event_name == self.event_name {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect()
        });

        for i in indices {
            let Some(mut handler) = RUNTIME.with(|rt| {
                rt.borrow_mut()
                    .event_listeners
                    .get_mut(i)
                    .and_then(|listener| listener.handler.take())
            }) else {
                continue;
            };

            let propagation = handler(data_any.as_ref());

            RUNTIME.with(|rt| {
                if let Some(listener) = rt.borrow_mut().event_listeners.get_mut(i) {
                    listener.handler = Some(handler);
                }
            });

            if propagation == Propagation::Stop {
                break;
            }
        }
    }
}

/// Returns an [`Emitter`] that, when called, dispatches a typed custom
/// event to every [`listen`] listener registered on an ancestor component
/// (including this component itself), in bubbling order.
///
/// Must be called inside a `#[component]` function.
///
/// # Example
/// ```ignore
/// #[component]
/// fn my_input<'a>() -> TuiNode<'a> {
///     let keys  = use_key();
///     let emit  = emitter::<String>("submitted");
///     keys.on(KeyCode::Enter, move || emit.emit("hello".into()));
///     tui! { <Input placeholder={"press Enter"} /> }
/// }
/// ```
pub fn emitter<T: 'static>(event_name: &'static str) -> Emitter<T> {
    let component_id = RUNTIME.with(|rt| {
        let rt = rt.borrow();
        *rt.id_stack
            .last()
            .expect("emitter() called outside of a #[component] function")
    });
    Emitter {
        event_name,
        component_id,
        _marker: std::marker::PhantomData,
    }
}

pub struct Bind<T: 'static> {
    current: T,
    emitter: Emitter<T>,
}

impl<T: Clone + 'static> Bind<T> {
    pub fn get(&self) -> T {
        self.current.clone()
    }

    pub fn set(&self, value: T) {
        self.emitter.emit(value);
    }
}

pub fn bind<T: Clone + 'static>(current: T, event_name: &'static str) -> Bind<T> {
    Bind {
        current,
        emitter: emitter::<T>(event_name),
    }
}

/// Registers a typed handler that receives custom events emitted by any
/// descendant component (or the component itself) for the given event name.
///
/// Handlers receive a shared reference to the event data and return a
/// [`Propagation`] value. Return [`Propagation::Stop`] to prevent the event
/// from reaching further ancestors; return [`Propagation::Continue`] to let
/// it keep bubbling.
///
/// Must be called inside a `#[component]` function.
///
/// # Example
/// ```ignore
/// #[component]
/// fn parent<'a>() -> TuiNode<'a> {
///     let log = state(Vec::<String>::new);
///     listen::<String>("submitted", {
///         let log = log.clone();
///         move |msg: &String| {
///             log.with_mut(|v| v.push(msg.clone()));
///             Propagation::Continue
///         }
///     });
///     tui! { <my_input /> }
/// }
/// ```
pub fn listen<T: 'static>(
    event_name: &'static str,
    handler: impl FnMut(&T) -> Propagation + 'static,
) {
    RUNTIME.with(|rt| {
        let component_id = *rt
            .borrow()
            .id_stack
            .last()
            .expect("listen() called outside of a #[component] function");
        use_on_component_id(component_id, event_name, handler);
    });
}

#[doc(hidden)]
pub fn use_on_component_id<T: 'static>(
    component_id: u64,
    event_name: &'static str,
    mut handler: impl FnMut(&T) -> Propagation + 'static,
) {
    RUNTIME.with(|rt| {
        rt.borrow_mut().event_listeners.push(EventListener {
            component_id,
            event_name,
            handler: Some(Box::new(move |data: &dyn Any| {
                if let Some(typed) = data.downcast_ref::<T>() {
                    handler(typed)
                } else {
                    Propagation::Continue
                }
            })),
        });
    });
}
