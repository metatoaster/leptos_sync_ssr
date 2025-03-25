use std::time::Duration;

use reactive_graph::owner::provide_context;
use tokio::time::timeout;

use super::set_reactive_owner;
use crate::Ready;

#[tokio::test]
async fn timeout_from_incomplete() -> anyhow::Result<()> {
    // Do actually demonstrate waiting will fail if Ready is provided
    let _owner = set_reactive_owner();
    let ready = Ready::new();
    provide_context(ready);

    let handle = Ready::handle();
    let subscription = handle.subscribe();

    let task = tokio::spawn(async move {
        timeout(Duration::from_millis(100), subscription.wait())
            .await
            .expect_err("subscription.wait() shouldn't return here");
    });
    task.await?;

    Ok(())
}

#[tokio::test]
async fn wait_after_ready() {
    let _owner = set_reactive_owner();
    let ready = Ready::new();
    provide_context(ready.clone());

    let handle = Ready::handle();

    let subscription_pre = handle.subscribe();
    ready.complete();
    let subscription_post = handle.subscribe();

    // wait should return immediately after completion.
    timeout(Duration::from_millis(100), subscription_pre.wait())
        .await
        .expect("subscription_pre.wait() should not have timed out");
    timeout(Duration::from_millis(100), subscription_post.wait())
        .await
        .expect("subscription_post.wait() should not have timed out");
}

#[tokio::test]
async fn wait_before_ready() -> anyhow::Result<()> {
    let _owner = set_reactive_owner();
    let ready = Ready::new();
    provide_context(ready.clone());

    let handle = Ready::handle();
    let subscription = handle.subscribe();

    let handle = tokio::spawn(async move {
        timeout(Duration::from_millis(100), subscription.wait())
            .await
            .expect("subscription.wait() should not have timed out");
    });
    tokio::spawn(async move {
        ready.complete();
    });
    handle.await?;

    Ok(())
}

#[tokio::test]
async fn without_context_no_waiting() -> anyhow::Result<()> {
    let handle = Ready::handle();
    let subscription = handle.subscribe();

    let task = tokio::spawn(async move {
        timeout(Duration::from_millis(100), subscription.wait())
            .await
            .expect("subscription.wait() shouldn't wait here");
    });
    task.await?;

    Ok(())
}
