use std::sync::Arc;

use crate::params::MetalXrossParams;

pub struct XrossCrunchSystem {
    params: Arc<MetalXrossParams>,
}

impl XrossCrunchSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self { params }
    }
}
