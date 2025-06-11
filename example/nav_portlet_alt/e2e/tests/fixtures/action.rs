use super::{find, world::HOST};
use anyhow::Result;
use fantoccini::Client;
use std::result::Result::Ok;

pub async fn goto_path(client: &Client, path: &str) -> Result<()> {
    let url = format!("{}{}", HOST, path);
    client.goto(&url).await?;
    Ok(())
}

pub async fn click_link(client: &Client, text: &str) -> Result<()> {
    find::link_with_text(client, text)
        .await?
        .click()
        .await?;
    Ok(())
}
