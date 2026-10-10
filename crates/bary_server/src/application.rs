pub trait Application: Sized {
    fn update(&mut self);

    fn draw(&mut self);

    fn should_exit(&self) -> bool;

    fn spin_forever(mut self) {
        while !self.should_exit() {
            self.spin_once();
        }
    }

    fn spin_once(&mut self) {
        self.update();
        self.draw();
    }
}
