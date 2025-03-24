use crate::fixtures::find;
use anyhow::{Ok, Result};
use fantoccini::Client;
use pretty_assertions::assert_eq;

pub async fn text_at_id_is(
    client: &Client,
    id: &str,
    expected: &str,
) -> Result<()> {
    let actual = find::text_at_id(client, id).await?;
    assert_eq!(&actual, expected);
    Ok(())
}

pub async fn id_is_visible(
    client: &Client,
    id: &str,
) -> Result<()> {
    let displayed = find::element_at_id(client, id)
        .await?
        .is_displayed()
        .await?;
    assert!(displayed);
    Ok(())
}

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
