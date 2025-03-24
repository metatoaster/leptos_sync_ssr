use crate::fixtures::{check, world::AppWorld};
use anyhow::{Ok, Result};
use cucumber::{then, gherkin::Step};

#[then(regex = r"^I find that the application is still working")]
async fn i_find_that_the_application_is_still_working(
    world: &mut AppWorld,
) -> Result<()> {
    let client = &world.client;
    check::id_is_invisible(client, "notice").await?;
    Ok(())
}

#[then(regex = "^I can see the following links$")]
async fn i_can_see_the_following_links(
    world: &mut AppWorld,
    step: &Step,
) -> Result<()> {
    let client = &world.client;
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter() {
            check::link_present(client, &row[0]).await?;
        }
    }
    Ok(())
}

#[then(regex = "^I can see the following links under Navigation$")]
async fn i_can_see_the_following_links_under_navigation(
    world: &mut AppWorld,
    step: &Step,
) -> Result<()> {
    let client = &world.client;
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter() {
            check::link_present_under_nav_portlet(client, &row[0]).await?;
        }
    }
    Ok(())
}

#[then(regex = "^I will not find the following links under Navigation$")]
async fn i_will_not_find_the_following_links_anywhere(
    world: &mut AppWorld,
    step: &Step,
) -> Result<()> {
    let client = &world.client;
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter() {
            check::link_absent_under_navigation(client, &row[0]).await?;
        }
    }
    Ok(())
}
