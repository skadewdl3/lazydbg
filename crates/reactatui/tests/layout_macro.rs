use ratatui::layout::Direction;
use ratatui::widgets::Paragraph;
use reactatui::layout::Size;
use reactatui::layout::style::Justify;
use reactatui::{Flex, Grid, layout, tui};

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
