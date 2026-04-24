#![cfg(feature = "server")]
use std::path::Path;

use log::error;
use vowlgrapher_database::prelude::VOWLGrapherStore;
use vowlgrapher_util::prelude::ErrorRecord;
use vowlgrapher_util::prelude::VOWLGRAPHER_ENVIRONMENT;

/// Applies applicable environment variables, if available.
pub async fn execute_env() {
    load_initial_file().await;
}

/// Loads a file into the database on server startup.
async fn load_initial_file() {
    if let Some(path) = &VOWLGRAPHER_ENVIRONMENT.load_initial_file {
        let store = VOWLGrapherStore::default();
        match store.insert_file(Path::new(path), false).await {
            Ok(w) => {
                if let Some(warnings) = w {
                    error!("Loaded file '{path}' with warnings:\n{warnings}");
                }
            }
            Err(e) => {
                let record: ErrorRecord = e.into();
                error!("Failed to load file '{path}':\n{record}");
            }
        }
    }
}
