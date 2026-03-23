use std::sync::Arc;

use crate::params::MetalXrossParams;

pub struct XrossDriveSystem {
    params: Arc<MetalXrossParams>,
}

impl XrossDriveSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self { params }
    }
}
