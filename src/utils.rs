use truce::params::FloatParam;

pub trait FloatParamNormalizedExt {
    fn value_normalized(&self) -> f64;
    fn set_value_normalized(&self, norm: f64); // FloatParamは内部でAtomicを使うので&selfでOKなはずです
}

impl FloatParamNormalizedExt for FloatParam {
    fn value_normalized(&self) -> f64 {
        let val = self.value() as f64;
        let range = &self.info.range;
        range.normalize(val)
    }

    fn set_value_normalized(&self, norm: f64) {
        let range = &self.info.range;
        let val = range.denormalize(norm);
        self.set_value(val);
    }
}
