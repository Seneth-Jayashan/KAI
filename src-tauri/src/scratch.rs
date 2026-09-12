use enigo::{Enigo, Settings, Mouse, Keyboard, Button, Key, Direction};
fn test() {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    enigo.move_mouse(100, 100, enigo::Coordinate::Abs).unwrap();
    enigo.button(Button::Left, Direction::Click).unwrap();
    enigo.text("hello").unwrap();
    enigo.key(Key::Return, Direction::Click).unwrap();
}
