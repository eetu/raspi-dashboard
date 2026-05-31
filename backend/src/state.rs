use std::sync::Arc;

use reqwest::Client;
use tokio::sync::Mutex;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub http: Client,
    /// Cached beszel (PocketBase) auth token. Minted on first use, reused until
    /// a 401 forces a re-auth (see [`crate::beszel`]). `None` until the first
    /// successful login, or always `None` when beszel creds are unset.
    pub beszel_token: Arc<Mutex<Option<String>>>,
}

impl AppState {
    pub fn new(cfg: Config, http: Client) -> Self {
        Self {
            cfg: Arc::new(cfg),
            http,
            beszel_token: Arc::new(Mutex::new(None)),
        }
    }
}
