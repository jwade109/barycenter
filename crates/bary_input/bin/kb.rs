use rdev::{Event, listen};

fn callback(event: Event) {
    println!("My callback {:?}", event);
    if let Some(string) = event.name { println!("User wrote {:?}", string) }
}
fn main() {
    // This will block.
    if let Err(error) = listen(callback) {
        println!("Error: {:?}", error)
    }
}
