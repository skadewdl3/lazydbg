use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use reactatui::hooks::{Propagation, register_mouse_region};
use reactatui::prelude::*;

fn render(runtime: &Runtime, view: impl FnOnce() -> TuiNode<'static>) {
    let area = Rect::new(0, 0, 20, 4);
    runtime.render_to_buffer(&mut Buffer::empty(area), area, view);
}

#[component]
fn StateProbe(capture: Rc<RefCell<Option<State<i32>>>>) -> TuiNode<'static> {
    let value = state(|| 1);
    *capture.borrow_mut() = Some(value);
    TuiNode::empty()
}

#[test]
fn runtimes_isolate_state_and_redraw_flags() {
    let first = Runtime::new();
    let second = Runtime::new();
    let first_capture = Rc::new(RefCell::new(None));
    let second_capture = Rc::new(RefCell::new(None));

    render(&first, {
        let capture = first_capture.clone();
        move || StateProbe(capture)
    });
    render(&second, {
        let capture = second_capture.clone();
        move || StateProbe(capture)
    });
    assert!(!first.needs_render());
    assert!(!second.needs_render());

    first_capture.borrow().as_ref().unwrap().set(7);
    assert!(first.needs_render());
    assert!(!second.needs_render());
    assert_eq!(first_capture.borrow().as_ref().unwrap().get(), 7);
    assert_eq!(second_capture.borrow().as_ref().unwrap().get(), 1);
}

#[component]
fn RefProbe(capture: Rc<RefCell<Option<Stored<i32>>>>) -> TuiNode<'static> {
    let value = use_ref(|| 1);
    *capture.borrow_mut() = Some(value);
    TuiNode::empty()
}

#[test]
fn refs_mutate_without_requesting_a_render() {
    let runtime = Runtime::new();
    let capture = Rc::new(RefCell::new(None));
    render(&runtime, {
        let capture = capture.clone();
        move || RefProbe(capture)
    });

    capture
        .borrow()
        .as_ref()
        .unwrap()
        .with_mut(|value| *value = 2);
    assert_eq!(capture.borrow().as_ref().unwrap().get(), 2);
    assert!(!runtime.needs_render());
}

struct ConditionalCapture {
    last: Option<State<i32>>,
    temp_initializations: usize,
    observed_last: i32,
}

#[component]
fn ConditionalProbe(
    show_temporary: bool,
    capture: Rc<RefCell<ConditionalCapture>>,
) -> TuiNode<'static> {
    let _first = state(|| 1);
    if show_temporary {
        let temporary = state(|| {
            capture.borrow_mut().temp_initializations += 1;
            2
        });
        assert_eq!(temporary.get(), 2);
    }
    let last = state(|| 3);
    let mut capture = capture.borrow_mut();
    capture.observed_last = last.get();
    capture.last = Some(last);
    TuiNode::empty()
}

#[test]
fn call_site_hooks_survive_conditional_neighbors() {
    let runtime = Runtime::new();
    let capture = Rc::new(RefCell::new(ConditionalCapture {
        last: None,
        temp_initializations: 0,
        observed_last: 0,
    }));

    render(&runtime, {
        let capture = capture.clone();
        move || ConditionalProbe(true, capture)
    });
    capture.borrow().last.as_ref().unwrap().set(9);
    render(&runtime, {
        let capture = capture.clone();
        move || ConditionalProbe(false, capture)
    });
    assert_eq!(capture.borrow().observed_last, 9);

    render(&runtime, {
        let capture = capture.clone();
        move || ConditionalProbe(true, capture)
    });
    assert_eq!(capture.borrow().observed_last, 9);
    assert_eq!(capture.borrow().temp_initializations, 2);
}

#[component]
fn MemoProbe(dep: u32, computes: Rc<Cell<usize>>, output: Rc<Cell<u32>>) -> TuiNode<'static> {
    let value = use_memo(dep, move || {
        computes.set(computes.get() + 1);
        dep * 2
    });
    output.set(*value);
    TuiNode::empty()
}

#[test]
fn memo_only_recomputes_when_dependencies_change() {
    let runtime = Runtime::new();
    let computes = Rc::new(Cell::new(0));
    let output = Rc::new(Cell::new(0));
    for dep in [2, 2, 3] {
        render(&runtime, {
            let computes = computes.clone();
            let output = output.clone();
            move || MemoProbe(dep, computes, output)
        });
    }
    assert_eq!(computes.get(), 2);
    assert_eq!(output.get(), 6);
}

struct EffectCounts {
    starts: Cell<usize>,
    cleanups: Cell<usize>,
}

#[component]
fn EffectProbe(dep: u32) -> TuiNode<'static> {
    use_effect(dep, move || {
        resource::<EffectCounts>()
            .starts
            .set(resource::<EffectCounts>().starts.get() + 1);
        move || {
            let counts = resource::<EffectCounts>();
            counts.cleanups.set(counts.cleanups.get() + 1);
        }
    });
    TuiNode::empty()
}

#[test]
fn effects_clean_up_on_dependency_change_and_unmount() {
    let runtime = Runtime::new();
    let counts = runtime.insert_resource(EffectCounts {
        starts: Cell::new(0),
        cleanups: Cell::new(0),
    });

    render(&runtime, || EffectProbe(1));
    render(&runtime, || EffectProbe(1));
    render(&runtime, || EffectProbe(2));
    assert_eq!(counts.starts.get(), 2);
    assert_eq!(counts.cleanups.get(), 1);

    render(&runtime, TuiNode::empty);
    assert_eq!(counts.cleanups.get(), 2);
}

#[component]
fn KeyChild(log: Rc<RefCell<Vec<&'static str>>>) -> TuiNode<'static> {
    use_focus(true);
    use_key().on(KeyCode::Char('x'), move || {
        log.borrow_mut().push("child");
        Propagation::Continue
    });
    TuiNode::empty()
}

#[component]
fn KeyParent(log: Rc<RefCell<Vec<&'static str>>>) -> TuiNode<'static> {
    let parent_log = log.clone();
    use_key().on(KeyCode::Char('x'), move || {
        parent_log.borrow_mut().push("parent");
        Propagation::Stop
    });
    KeyChild(log)
}

#[test]
fn keys_route_from_the_focused_component_to_its_ancestors() {
    let runtime = Runtime::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    render(&runtime, {
        let log = log.clone();
        move || KeyParent(log)
    });

    let outcome = runtime.handle_event(Event::Key(KeyEvent::new(
        KeyCode::Char('x'),
        KeyModifiers::NONE,
    )));
    assert!(outcome.handled);
    assert_eq!(*log.borrow(), ["child", "parent"]);

    let release = KeyEvent::new_with_kind(
        KeyCode::Char('x'),
        KeyModifiers::NONE,
        KeyEventKind::Release,
    );
    assert!(!runtime.handle_event(Event::Key(release)).handled);
}

#[component]
fn MouseChild(log: Rc<RefCell<Vec<&'static str>>>) -> TuiNode<'static> {
    let owner = reactatui::hooks::__current_component_id();
    TuiNode::Widget(Box::new(move |area, _| {
        let _region = register_mouse_region(
            owner,
            area,
            Some(Box::new(move |_| {
                log.borrow_mut().push("child");
                Propagation::Continue
            })),
            None,
            None,
            None,
            None,
        );
    }))
}

#[component]
fn MouseParent(log: Rc<RefCell<Vec<&'static str>>>) -> TuiNode<'static> {
    let owner = reactatui::hooks::__current_component_id();
    let child = MouseChild(log.clone());
    TuiNode::Widget(Box::new(move |area, buffer| {
        let _region = register_mouse_region(
            owner,
            area,
            Some(Box::new(move |_| {
                log.borrow_mut().push("parent");
                Propagation::Stop
            })),
            None,
            None,
            None,
            None,
        );
        child.render(area, buffer);
    }))
}

#[test]
fn mouse_events_target_the_last_painted_region_then_bubble() {
    let runtime = Runtime::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    render(&runtime, {
        let log = log.clone();
        move || MouseParent(log)
    });

    runtime.handle_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 1,
        row: 1,
        modifiers: KeyModifiers::NONE,
    }));
    assert_eq!(*log.borrow(), ["child", "parent"]);
    assert!(runtime.needs_render());
}
