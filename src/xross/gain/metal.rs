use std::sync::Arc;

use crate::params::MetalXrossParams;

pub struct XrossMetalSystem {
    params: Arc<MetalXrossParams>,
}

impl XrossMetalSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self { params }
    }
}
