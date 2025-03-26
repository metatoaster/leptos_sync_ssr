use anyhow::{anyhow, Result};
use fantoccini::{elements::Element, Client, Locator};

pub async fn element_at_id(client: &Client, id: &str) -> Result<Element> {
    client
        .wait()
        .for_element(Locator::Id(id))
        .await
        .map_err(|_| anyhow!("no such id: `{}`", id))
}

pub async fn text_at_id(client: &Client, id: &str) -> Result<String> {
    let text = element_at_id(client, id)
        .await?
        .text()
        .await?;
    Ok(text)
}

pub async fn link_with_text(client: &Client, text: &str) -> Result<Element> {
    client
        .wait()
        .for_element(Locator::LinkText(text))
        .await
        .map_err(|_| anyhow!("no such link with text: `{}`", text))
}
