pub trait Point {
    fn coordinates(&self) -> (isize, isize);
}

pub trait Site: Point {
    fn weight(&self) -> f32;
}

impl Point for (isize, isize, f32) {
    fn coordinates(&self) -> (isize, isize) {
        (self.0, self.1)
    }
}

impl Site for (isize, isize, f32) {
    fn weight(&self) -> f32 {
        self.2
    }
}

impl Point for [isize; 3] {
    fn coordinates(&self) -> (isize, isize) {
        (self[0], self[1])
    }
}

impl Site for [isize; 3] {
    fn weight(&self) -> f32 {
        self[2] as f32
    }
}
