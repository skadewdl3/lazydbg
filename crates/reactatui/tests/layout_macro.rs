use ratatui::layout::Direction;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Paragraph, Widget},
};
use reactatui::layout::Size;
use reactatui::layout::style::Justify;
use reactatui::prelude::{Action, Callback, Runtime, State, bind, state};
use reactatui::{Flex, Grid, layout, tui};
use reactatui::{TuiNode, component};

#[component]
fn DefaultSlot<'a>(#[slot(default)] content: Option<TuiNode<'a>>) -> TuiNode<'a> {
    tui! { <{content} /> }
}

#[component]
fn NamedSlot<'a>(#[slot] heading: Option<TuiNode<'a>>) -> TuiNode<'a> {
    tui! { <{heading} /> }
}

#[component]
fn RequiredSlot<'a>(#[slot] heading: TuiNode<'a>) -> TuiNode<'a> {
    tui! { <{heading} /> }
}

#[component]
fn PropsWithConstructor<'a>(
    prefix: &'a str,
    #[prop] second: &'a str,
    #[prop] first: &'a str,
) -> TuiNode<'a> {
    tui! { <Paragraph::new(format!("{prefix}{first}{second}")) /> }
}

#[component]
fn TypedEvents(#[prop] on_activate: Action, #[prop] on_value: Callback<u32>) -> TuiNode<'static> {
    on_activate.call();
    on_value.call(42);
    TuiNode::empty()
}

#[component]
fn KeyedItem() -> TuiNode<'static> {
    TuiNode::empty()
}

#[component]
fn NamedBoundChild(#[bind] value: State<i32>) -> TuiNode<'static> {
    bind(value).set(42);
    TuiNode::empty()
}

#[component]
fn DefaultBoundChild(#[bind(default)] value: State<i32>) -> TuiNode<'static> {
    bind(value).set(24);
    TuiNode::empty()
}

#[component]
fn OptionalBoundChild(#[bind] value: Option<State<i32>>) -> TuiNode<'static> {
    if let Some(value) = value {
        bind(value).set(7);
    }
    TuiNode::empty()
}

#[component]
fn NamedBoundParent(
    capture: std::rc::Rc<std::cell::RefCell<Option<State<i32>>>>,
) -> TuiNode<'static> {
    let value = state(|| 0);
    *capture.borrow_mut() = Some(value.clone());
    tui! { <NamedBoundChild bind:value={value} /> }
}

#[component]
fn DefaultBoundParent(
    capture: std::rc::Rc<std::cell::RefCell<Option<State<i32>>>>,
) -> TuiNode<'static> {
    let value = state(|| 0);
    *capture.borrow_mut() = Some(value.clone());
    tui! { <DefaultBoundChild bind={value} /> }
}

#[component]
fn OptionalBoundParent(
    capture: std::rc::Rc<std::cell::RefCell<Option<State<i32>>>>,
) -> TuiNode<'static> {
    let value = state(|| 0);
    *capture.borrow_mut() = Some(value.clone());
    tui! { <OptionalBoundChild bind:value={value} /> }
}

#[component]
fn OptionalUnboundParent() -> TuiNode<'static> {
    tui! { <OptionalBoundChild /> }
}

#[component]
fn RequiredUnboundParent() -> TuiNode<'static> {
    tui! { <NamedBoundChild /> }
}

#[test]
fn parses_css_like_layout_rules() {
    let style = layout! {
        direction: vertical;
        justify-content: space-between;
        gap: 2;
        columns: 1fr auto 20;
    };

    assert_eq!(style.direction, Some(Direction::Vertical));
    assert_eq!(style.justify_content, Justify::SpaceBetween);
    assert_eq!(style.gap, 2);
    assert_eq!(style.columns.expect("columns").len(), 3);
}

#[test]
fn parses_ignore_boolean() {
    let style = layout! { ignore: true; };

    assert!(style.ignore);
}

#[test]
fn parses_css_size_values() {
    assert_eq!(layout! { size: 3; }.size, Size::Length(3));
    assert_eq!(layout! { size: 1fr; }.size, Size::Fr(1));
    assert_eq!(layout! { size: 10%; }.size, Size::Percent(10));
    assert_eq!(layout! { size: auto; }.size, Size::Auto);
}

#[test]
fn parses_inline_and_block_control_flow() {
    let primary = false;
    let secondary = true;

    let style = layout! {
        justify-content: if primary { start } else if secondary { center } else { end };

        if primary {
            gap: 1;
        } else if secondary {
            gap: 2;
            direction: horizontal;
        } else {
            gap: 3;
        }
    };

    assert_eq!(style.justify_content, Justify::Center);
    assert_eq!(style.direction, Some(Direction::Horizontal));
    assert_eq!(style.gap, 2);
}

#[test]
fn parses_block_match_rules_and_shorthand_arms() {
    let choice = 2;

    let style = layout! {
        match choice {
            1 => justify-content: start;
            2 if true => center;
            _ => {
                justify-content: end;
                gap: 4;
            }
        }
    };

    assert_eq!(style.justify_content, Justify::Center);
}

#[test]
fn layout_prop_is_reserved_and_accepted() {
    let _node = tui! {
        <Flex::vertical layout={layout! { gap: 1; }}>
            <Paragraph::new("child") layout={layout! { size: 1; }} />
        </Flex>
    };
}

#[test]
fn layout_containers_use_imported_types_constructors_props_and_styles() {
    let _node = tui! {
        <Flex::horizontal style={layout! { gap: 2; padding: 1; }}>
            <Paragraph::new("left") layout={layout! { size: 1fr; }} />
            <Grid layout={layout! { columns: 1fr 1fr; rows: auto; gap: 1; }}>
                <Paragraph::new("cell") layout={layout! { column: 1; }} />
            </Grid>
        </Flex>
    };

    let _: Option<Flex<'static>> = None;
    let _: Option<Grid<'static>> = None;
}

#[test]
fn dynamic_nodes_accept_node_level_props() {
    let input = tui! { <Paragraph::new("input") /> };
    let node = tui! {
        <{input} layout={layout! { size: 3; }} />
    };

    let (style, _) = node.take_style();
    assert_eq!(style.size, Size::Length(3));
}

#[test]
#[should_panic(
    expected = "dynamic component requires exactly one top-level component, but received 2"
)]
fn dynamic_nodes_reject_multiple_top_level_components() {
    let multiple = tui! {
        <>
            <Paragraph::new("first") />
            <Paragraph::new("second") />
        </>
    };

    let _ = tui! { <{multiple} layout={layout! { size: 3; }} /> };
}

#[test]
fn components_project_default_and_named_slots() {
    let default = tui! {
        <DefaultSlot>
            <Paragraph::new("default") />
        </DefaultSlot>
    };
    let named = tui! {
        <NamedSlot>
            <Paragraph::new("heading") slot={"heading"} />
            <Paragraph::new("ignored") />
        </NamedSlot>
    };

    let mut default_buffer = Buffer::empty(Rect::new(0, 0, 12, 1));
    default.render(Rect::new(0, 0, 12, 1), &mut default_buffer);
    assert_eq!(default_buffer[(0, 0)].symbol(), "d");

    let mut named_buffer = Buffer::empty(Rect::new(0, 0, 12, 1));
    named.render(Rect::new(0, 0, 12, 1), &mut named_buffer);
    assert_eq!(named_buffer[(0, 0)].symbol(), "h");
}

#[test]
fn component_props_bind_by_name_after_constructor_arguments() {
    let node = tui! {
        <PropsWithConstructor("value:") second="2" first="1" />
    };

    let mut buffer = Buffer::empty(Rect::new(0, 0, 12, 1));
    node.render(Rect::new(0, 0, 12, 1), &mut buffer);
    assert_eq!(buffer[(0, 0)].symbol(), "v");
    assert_eq!(buffer[(6, 0)].symbol(), "1");
    assert_eq!(buffer[(7, 0)].symbol(), "2");
}

#[test]
fn component_props_support_same_name_shorthand() {
    let first = "1";
    let second = "2";
    let node = tui! {
        <PropsWithConstructor("value:") {second} {first} />
    };

    let mut buffer = Buffer::empty(Rect::new(0, 0, 12, 1));
    node.render(Rect::new(0, 0, 12, 1), &mut buffer);
    assert_eq!(buffer[(0, 0)].symbol(), "v");
    assert_eq!(buffer[(6, 0)].symbol(), "1");
    assert_eq!(buffer[(7, 0)].symbol(), "2");
}

#[test]
fn a_slot_groups_multiple_nodes_into_one_prop() {
    let node = tui! {
        <DefaultSlot>
            <Paragraph::new("first") />
            <Paragraph::new("second") />
        </DefaultSlot>
    };

    let mut buffer = Buffer::empty(Rect::new(0, 0, 12, 1));
    node.render(Rect::new(0, 0, 12, 1), &mut buffer);
    assert_eq!(buffer[(0, 0)].symbol(), "s");
}

#[test]
fn optional_slots_may_be_omitted() {
    let node = tui! { <NamedSlot /> };
    let mut buffer = Buffer::empty(Rect::new(0, 0, 12, 1));
    node.render(Rect::new(0, 0, 12, 1), &mut buffer);
    assert_eq!(buffer[(0, 0)].symbol(), " ");
}

#[test]
#[should_panic(expected = "required slot `heading` was not provided")]
fn required_slots_must_be_supplied() {
    let _ = tui! { <RequiredSlot /> };
}

#[test]
fn custom_events_are_typed_props_with_familiar_syntax() {
    use std::cell::Cell;
    use std::rc::Rc;

    let activations = Rc::new(Cell::new(0));
    let value = Rc::new(Cell::new(0));
    let action_count = activations.clone();
    let output = value.clone();
    let _ = tui! {
        <TypedEvents
            on:activate={move || action_count.set(action_count.get() + 1)}
            on:value={move |next| output.set(next)}
        />
    };

    assert_eq!(activations.get(), 1);
    assert_eq!(value.get(), 42);
}

#[test]
fn named_bindings_share_parent_state() {
    use std::{cell::RefCell, rc::Rc};

    let capture = Rc::new(RefCell::new(None));
    let runtime = Runtime::new();
    let area = Rect::new(0, 0, 1, 1);
    let mut buffer = Buffer::empty(area);
    runtime.render_to_buffer(&mut buffer, area, || {
        tui! { <NamedBoundParent(capture.clone()) /> }
    });

    assert_eq!(capture.borrow().as_ref().expect("parent state").get(), 42);
}

#[test]
fn default_bindings_use_the_unnamed_bind_attribute() {
    use std::{cell::RefCell, rc::Rc};

    let capture = Rc::new(RefCell::new(None));
    let runtime = Runtime::new();
    let area = Rect::new(0, 0, 1, 1);
    let mut buffer = Buffer::empty(area);
    runtime.render_to_buffer(&mut buffer, area, || {
        tui! { <DefaultBoundParent(capture.clone()) /> }
    });

    assert_eq!(capture.borrow().as_ref().expect("parent state").get(), 24);
}

#[test]
fn optional_bindings_may_be_omitted_or_supplied() {
    use std::{cell::RefCell, rc::Rc};

    let runtime = Runtime::new();
    let area = Rect::new(0, 0, 1, 1);
    let mut buffer = Buffer::empty(area);
    runtime.render_to_buffer(&mut buffer, area, || {
        tui! { <OptionalUnboundParent /> }
    });

    let capture = Rc::new(RefCell::new(None));
    runtime.render_to_buffer(&mut buffer, area, || {
        tui! { <OptionalBoundParent(capture.clone()) /> }
    });
    assert_eq!(capture.borrow().as_ref().expect("parent state").get(), 7);
}

#[test]
#[should_panic(expected = "required binding `value` was not provided")]
fn required_bindings_must_be_supplied() {
    let runtime = Runtime::new();
    let area = Rect::new(0, 0, 1, 1);
    let mut buffer = Buffer::empty(area);
    runtime.render_to_buffer(&mut buffer, area, || tui! { <RequiredUnboundParent /> });
}

#[test]
fn explicit_keys_identify_looped_components() {
    let runtime = Runtime::new();
    let area = Rect::new(0, 0, 4, 2);
    let mut buffer = Buffer::empty(area);
    runtime.render_to_buffer(&mut buffer, area, || {
        tui! {
            <Flex::vertical>
                for id in [10, 20] {
                    <KeyedItem key={id} />
                }
            </Flex>
        }
    });
}

#[test]
fn unkeyed_looped_components_fall_back_to_occurrence_identity() {
    let runtime = Runtime::new();
    let area = Rect::new(0, 0, 4, 2);
    let mut buffer = Buffer::empty(area);
    runtime.render_to_buffer(&mut buffer, area, || {
        tui! {
            <Flex::vertical>
                for _ in 0..2 {
                    <KeyedItem />
                }
            </Flex>
        }
    });
}

#[test]
#[should_panic(expected = "duplicate explicit key for component `KeyedItem`")]
fn duplicate_explicit_keys_are_rejected() {
    let runtime = Runtime::new();
    let area = Rect::new(0, 0, 4, 2);
    let mut buffer = Buffer::empty(area);
    runtime.render_to_buffer(&mut buffer, area, || {
        tui! {
            <Flex::vertical>
                for _ in 0..2 {
                    <KeyedItem key={10} />
                }
            </Flex>
        }
    });
}
