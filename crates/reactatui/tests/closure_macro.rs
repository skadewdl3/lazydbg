use std::rc::Rc;

use reactatui::lambda;

#[test]
fn supports_move_borrow_mut_borrow_and_clone_captures() {
    let moved = String::from("moved");
    let borrowed = String::from("borrowed");
    let mut mut_borrowed = String::from("mutable");
    let cloned = Rc::new(String::from("cloned"));

    let handler = lambda!(moved, &borrowed, &mut mut_borrowed, +cloned, |suffix: &str| {
        mut_borrowed.push_str(suffix);
        (moved, borrowed.len(), cloned)
    });

    let (moved, borrowed_len, captured_clone) = handler("!");
    assert_eq!(moved, "moved");
    assert_eq!(borrowed_len, borrowed.len());
    assert_eq!(mut_borrowed, "mutable!");
    assert!(Rc::ptr_eq(&cloned, &captured_clone));
}
