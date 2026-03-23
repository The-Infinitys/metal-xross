use std::sync::Arc;

use crate::params::MetalXrossParams;

pub struct XrossDistSystem {
    params: Arc<MetalXrossParams>,
}

impl XrossDistSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self { params }
    }
}
