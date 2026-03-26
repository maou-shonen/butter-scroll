use std::sync::{Arc, Mutex};

use crossbeam_channel::Sender;

use crate::config::ConfigStore;
use crate::threshold::AppThresholdCache;
use crate::traits::EngineCommand;

pub struct AppState {
    pub engine_tx: Sender<EngineCommand>,
    pub config_store: Arc<dyn ConfigStore>,
    pub threshold_cache: Arc<Mutex<AppThresholdCache>>,
    /// True when running from a portable (non-installed) distribution.
    pub portable: bool,
}
