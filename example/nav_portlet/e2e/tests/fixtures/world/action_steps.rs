use crate::fixtures::{action, find, world::AppWorld};
use anyhow::{Ok, Result};
use cucumber::{given, when, gherkin::Step};

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

#[when(expr = "I access the following links in the following order")]
async fn i_access_the_following_links(
    world: &mut AppWorld,
    step: &Step,
) -> Result<()> {
    let client = &world.client;
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter() {
            action::click_link(client, &row[0]).await?;
        }
    }
    Ok(())
}

#[when(regex = "^once I see the article view is populated$")]
async fn once_i_see_the_article_view_is_populated(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    find::element_at_id(client, "article-view").await?;
    Ok(())
}

#[when(regex = "^once I see the author overview is populated$")]
async fn once_i_see_the_author_overview_is_populated(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    find::element_at_id(client, "author-overview").await?;
    Ok(())
}
