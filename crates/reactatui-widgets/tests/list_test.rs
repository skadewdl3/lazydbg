use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use reactatui::prelude::*;
use reactatui_widgets::*;

#[component]
fn CustomContainer<'a>(title: &'a str, #[children] children: Vec<TuiNode<'a>>) -> TuiNode<'a> {
    tui! {
        <Flex direction={Direction::Vertical}>
            <Paragraph text={title} />
            {
                for child in children {
                    tui! { child }
                }
            }
        </Flex>
    }
}

#[test]
fn test_list_with_children() {
    let node = tui! {
        <List>
            <ListItem text={"First"} />
            <ListItem text={"Second"} />
            <ListItem text={"Third"} />
        </List>
    };

    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 10));
    node.render(Rect::new(0, 0, 20, 10), &mut buf);
}

#[test]
fn test_custom_component_with_children_attr() {
    let node = tui! {
        <CustomContainer title={"Header"}>
            <ListItem text={"Child 1"} />
            <ListItem text={"Child 2"} />
        </CustomContainer>
    };

    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 10));
    node.render(Rect::new(0, 0, 20, 10), &mut buf);
}
