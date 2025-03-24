use crate::fixtures::{action, world::AppWorld};
use anyhow::{Ok, Result};
use cucumber::{given, when};

#[given("I see the app")]
#[when("I open the app")]
async fn i_open_the_app(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    action::goto_path(client, "").await?;
    Ok(())
}

#[when(regex = "^I access the link (.*)$")]
async fn i_access_the_link(world: &mut AppWorld, text: String) -> Result<()> {
    let client = &world.client;
    action::click_link(client, &text).await?;
    Ok(())
}

#[given(regex = "^I (refresh|reload) the (browser|page)$")]
#[when(regex = "^I (refresh|reload) the (browser|page)$")]
async fn i_refresh_the_browser(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    client.refresh().await?;

    Ok(())
}

