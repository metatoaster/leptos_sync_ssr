mod fixtures;

use anyhow::Result;
use cucumber::World;
use fixtures::world::AppWorld;
use std::{ffi::OsStr, fs::read_dir};

#[tokio::main]
async fn main() -> Result<()> {
    for entry in read_dir("./features")? {
        let path = entry?.path();
        if path.extension() == Some(OsStr::new("feature")) {
            AppWorld::cucumber()
                .fail_on_skipped()
                .run_and_exit(path)
                .await;
        }
    }
    Ok(())
}
