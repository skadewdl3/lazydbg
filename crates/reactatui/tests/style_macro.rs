use ratatui::style::{Color, Modifier, Style};
use reactatui::style;

#[test]
fn returns_a_plain_ratatui_style() {
    let actual: ratatui::prelude::Style = style! {
        color: red;
        background-color: #0f0;
        text-decoration-color: rgb(1, 2, 3);
    }
    .into();

    assert_eq!(
        actual,
        Style::new()
            .fg(Color::Red)
            .bg(Color::Rgb(0, 255, 0))
            .underline_color(Color::Rgb(1, 2, 3))
    );
}

#[test]
fn parses_color_forms_and_property_aliases() {
    let indexed = 214u8;
    let from_expression = Color::LightMagenta;

    assert_eq!(style! { fg: #Ff8800; }.fg, Some(Color::Rgb(255, 136, 0)));
    assert_eq!(
        style! { bg: indexed(indexed); }.bg,
        Some(Color::Indexed(214))
    );
    assert_eq!(
        style! { underline-color: {from_expression}; }.underline_color,
        Some(Color::LightMagenta)
    );
    assert_eq!(style! { background: silver; }.bg, Some(Color::Gray));
    assert_eq!(style! { color: bright-black; }.fg, Some(Color::DarkGray));
    assert_eq!(style! { color: bright-white; }.fg, Some(Color::White));
}

#[test]
fn supports_the_complete_named_ratatui_palette() {
    let cases = [
        (style! { color: reset; }.fg, Color::Reset),
        (style! { color: black; }.fg, Color::Black),
        (style! { color: red; }.fg, Color::Red),
        (style! { color: green; }.fg, Color::Green),
        (style! { color: yellow; }.fg, Color::Yellow),
        (style! { color: blue; }.fg, Color::Blue),
        (style! { color: magenta; }.fg, Color::Magenta),
        (style! { color: cyan; }.fg, Color::Cyan),
        (style! { color: gray; }.fg, Color::Gray),
        (style! { color: dark-gray; }.fg, Color::DarkGray),
        (style! { color: light-red; }.fg, Color::LightRed),
        (style! { color: light-green; }.fg, Color::LightGreen),
        (style! { color: light-yellow; }.fg, Color::LightYellow),
        (style! { color: light-blue; }.fg, Color::LightBlue),
        (style! { color: light-magenta; }.fg, Color::LightMagenta),
        (style! { color: light-cyan; }.fg, Color::LightCyan),
        (style! { color: white; }.fg, Color::White),
    ];

    for (actual, expected) in cases {
        assert_eq!(actual, Some(expected));
    }
}

#[test]
fn maps_css_properties_to_modifiers() {
    let actual = style! {
        font-weight: bold;
        font-style: italic;
        text-decoration-line: underline line-through blink rapid-blink;
        visibility: hidden;
        text-style: dim reversed;
    };

    let expected = Modifier::BOLD
        | Modifier::DIM
        | Modifier::ITALIC
        | Modifier::UNDERLINED
        | Modifier::SLOW_BLINK
        | Modifier::RAPID_BLINK
        | Modifier::REVERSED
        | Modifier::HIDDEN
        | Modifier::CROSSED_OUT;
    assert_eq!(actual.add_modifier, expected);
    assert_eq!(actual.sub_modifier, Modifier::empty());
}

#[test]
fn supports_every_explicit_modifier_removal() {
    let actual = style! {
        text-style: bold dim italic underline slow-blink rapid-blink reversed hidden crossed-out;
        text-style: not-bold not-dim not-italic not-underline not-slow-blink not-rapid-blink
            not-reversed not-hidden not-crossed-out;
    };

    assert_eq!(actual.add_modifier, Modifier::empty());
    assert_eq!(
        actual.sub_modifier,
        Modifier::BOLD
            | Modifier::DIM
            | Modifier::ITALIC
            | Modifier::UNDERLINED
            | Modifier::SLOW_BLINK
            | Modifier::RAPID_BLINK
            | Modifier::REVERSED
            | Modifier::HIDDEN
            | Modifier::CROSSED_OUT
    );
}

#[test]
fn css_normal_visible_none_values_remove_their_modifier_groups() {
    let actual = style! {
        font-weight: normal;
        font-style: normal;
        text-decoration-line: none;
        visibility: visible;
    };

    assert!(actual.sub_modifier.contains(Modifier::BOLD));
    assert!(actual.sub_modifier.contains(Modifier::ITALIC));
    assert!(actual.sub_modifier.contains(Modifier::UNDERLINED));
    assert!(actual.sub_modifier.contains(Modifier::SLOW_BLINK));
    assert!(actual.sub_modifier.contains(Modifier::RAPID_BLINK));
    assert!(actual.sub_modifier.contains(Modifier::CROSSED_OUT));
    assert!(actual.sub_modifier.contains(Modifier::HIDDEN));
}

#[test]
fn text_style_none_removes_all_modifiers_before_later_operations() {
    let actual = style! {
        text-style: bold italic;
        text-style: none bold;
    };

    assert_eq!(actual.add_modifier, Modifier::BOLD);
    assert!(!actual.sub_modifier.contains(Modifier::BOLD));
    assert!(actual.sub_modifier.contains(Modifier::ITALIC));
    assert!(actual.sub_modifier.contains(Modifier::REVERSED));
}

#[test]
fn parses_inline_and_block_conditionals() {
    let focused = false;
    let disabled = true;

    let actual = style! {
        color: if focused { yellow } else if disabled { dark-gray } else { white };
        background-color: if focused { blue };
        text-style: if disabled { dim not-bold };

        if focused {
            font-weight: bold;
        } else if disabled {
            font-style: italic;
            visibility: hidden;
        } else {
            text-decoration-line: underline;
        }
    };

    assert_eq!(actual.fg, Some(Color::DarkGray));
    assert_eq!(actual.bg, None);
    assert!(actual.add_modifier.contains(Modifier::DIM));
    assert!(actual.add_modifier.contains(Modifier::ITALIC));
    assert!(actual.add_modifier.contains(Modifier::HIDDEN));
    assert!(actual.sub_modifier.contains(Modifier::BOLD));
}

#[test]
fn parses_guarded_match_rules_blocks_and_shorthand_arms() {
    let state = 2;

    let actual = style! {
        match state {
            1 => color: green;
            2 if true => yellow;
            _ => {
                color: red;
                text-style: bold;
            }
        }
    };

    assert_eq!(actual.fg, Some(Color::Yellow));
    assert_eq!(actual.add_modifier, Modifier::empty());
}

#[test]
fn all_and_patch_follow_declaration_order() {
    let base = Style::new().bg(Color::Blue).add_modifier(Modifier::ITALIC);

    let initial = style! {
        color: red;
        all: initial;
        color: yellow;
        patch: {base};
    };
    assert_eq!(
        ratatui::prelude::Style::from(initial),
        Style::new()
            .fg(Color::Yellow)
            .bg(Color::Blue)
            .add_modifier(Modifier::ITALIC)
    );

    let reset = style! {
        color: red;
        all: reset;
        color: yellow;
    };
    assert_eq!(
        ratatui::prelude::Style::from(reset),
        Style::reset().fg(Color::Yellow)
    );
}
