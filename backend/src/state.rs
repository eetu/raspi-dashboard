use std::sync::Arc;

use reqwest::Client;
use tokio::sync::Mutex;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub http: Client,
    /// Cached beszel (PocketBase) auth token. Minted on first use and reused
    /// until it's rejected — either an explicit 401 or a silent guest-downgrade
    /// that yields an empty `systems` list, both of which force a re-auth (see
    /// [`crate::beszel`]). `None` until the first successful login, or always
    /// `None` when beszel creds are unset.
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
