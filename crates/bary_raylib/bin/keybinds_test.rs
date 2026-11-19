use bary_input::InputHandler;
use bary_raylib::load_keybinds;
use std::time::Duration;

fn main() {
    let binds = load_keybinds("assets/test_keybinds.yaml");

    let mut input_handler = InputHandler::new();

    for (action, keybind) in binds.iter() {
        println!("{action:?} -> {keybind}");

        for (a, ka) in binds.iter() {
            if action == a {
                continue;
            }
            if keybind.is_conflicted_with(ka) {
                println!("Keybind collision: {action:?} ({keybind}) and {a:?} ({ka})")
            }
        }
    }

    println!("=======");

    let mut frame_no = 0;

    loop {
        let input = input_handler.update(true);

        for (action, keybind) in binds.iter() {
            if keybind.is_satisfied(&input) {
                println!("{frame_no}: {action:?} -> {keybind}");
            }
        }

        frame_no += 1;

        std::thread::sleep(Duration::from_millis(4));
    }
}
