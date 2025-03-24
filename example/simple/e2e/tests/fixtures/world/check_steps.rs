use crate::fixtures::{check, world::AppWorld};
use anyhow::{Ok, Result};
use cucumber::then;

#[then(regex = r"^I see the bolded text is (.*)$")]
async fn i_see_the_bolded_text_is(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    let client = &world.client;
    check::text_at_id_is(client, "target", &text).await?;
    Ok(())
}

#[then(regex = r"^I find that the application has panicked")]
async fn i_find_that_the_application_has_panicked(
    world: &mut AppWorld,
) -> Result<()> {
    let client = &world.client;
    check::id_is_visible(client, "notice").await?;
    Ok(())
}

#[then(regex = r"^I find that the application is still working")]
async fn i_find_that_the_application_is_still_working(
    world: &mut AppWorld,
) -> Result<()> {
    let client = &world.client;
    check::id_is_invisible(client, "notice").await?;
    Ok(())
}
