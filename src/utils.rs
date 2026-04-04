pub trait DbToLinear {
    fn db_to_gain(self) -> f32;
}

impl DbToLinear for f32 {
    fn db_to_gain(self) -> f32 {
        10.0f32.powf(self / 20.0)
    }
}
