use crate::fixtures::find;
use anyhow::{Ok, Result};
use fantoccini::{Client, Locator};

pub async fn id_is_invisible(
    client: &Client,
    id: &str,
) -> Result<()> {
    let displayed = find::element_at_id(client, id)
        .await?
        .is_displayed()
        .await?;
    assert!(!displayed);
    Ok(())
}

pub async fn link_present(
    client: &Client,
    text: &str,
) -> Result<()> {
    let result = find::link_with_text(client, text).await;
    assert!(result.is_ok());
    Ok(())
}

pub async fn link_present_under_nav_portlet(
    client: &Client,
    text: &str,
) -> Result<()> {
    let element = find::element_at_id(client, "NavPortlet").await?;
    let result = element.find(Locator::LinkText(text)).await;
    assert!(result.is_ok());
    Ok(())
}

pub async fn link_absent_under_navigation(
    client: &Client,
    text: &str,
) -> Result<()> {
    let element = find::element_at_id(client, "NavPortlet").await?;
    let result = element.find(Locator::LinkText(text)).await;
    assert!(result.is_err());
    Ok(())
}
