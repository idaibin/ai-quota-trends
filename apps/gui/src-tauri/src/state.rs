use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use ai_quota_core::{Database, SharedCollectorState};
use tokio::sync::Notify;

pub struct AppState {
    pub database: Arc<Mutex<Database>>,
    pub collector_state: SharedCollectorState,
    pub collector_refresh: Arc<Notify>,
    pub collector_reload: Arc<Notify>,
    pub data_dir: PathBuf,
}
