use leptos::prelude::*;
use leptos_meta::{MetaTags, *};
use leptos_router::{
    components::{ParentRoute, Route, Router, Routes, A},
    hooks::use_params,
    nested_router::Outlet,
    params::Params,
    path, MatchNestedRoutes, ParamSegment, SsrMode, StaticSegment,
};

use leptos_sync_ssr::component::SyncSsrSignal;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct Author {
    pub name: String,
    pub email: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct Article {
    pub id: u32,
    pub author_name: String,
    pub title: String,
}

#[cfg(feature = "ssr")]
pub(super) mod server {
    use super::{Article, Author};
    use std::{collections::HashMap, sync::LazyLock};

    impl From<(&'static str, &'static str)> for Author {
        fn from((name, email): (&'static str, &'static str)) -> Self {
            Author {
                name: name.to_string(),
                email: email.to_string(),
            }
        }
    }

    impl From<(u32, &'static str, &'static str)> for Article {
        fn from((id, author_name, title): (u32, &'static str, &'static str)) -> Self {
            Article {
                id,
                author_name: author_name.to_string(),
                title: title.to_string(),
            }
        }
    }

    pub static TIMEOUT: u64 = 50;

    pub static AUTHORS: LazyLock<HashMap<&'static str, Author>> = LazyLock::new(|| {
        HashMap::from([
            ("albert", ("Albert", "albert.g@example.com").into()),
            ("bethany", ("Bethany", "beth@example.com").into()),
            ("carl", ("Carl", "c.smith@example.com").into()),
            ("dorothy", ("Dorothy", "dorothy@example.com").into()),
        ])
    });

    pub static ARTICLES: LazyLock<HashMap<u32, Article>> = LazyLock::new(|| {
        [
            (1, "dorothy", "The top twenty...").into(),
            (2, "albert", "On the practical nature of...").into(),
            (3, "bethany", "How to guide to...").into(),
            (4, "dorothy", "The top ten...").into(),
            (5, "albert", "Why a city's infrastructure...").into(),
            (6, "bethany", "The ultimate guide to...").into(),
            (7, "dorothy", "The top hundred...").into(),
            (8, "carl", "A quick summary on...").into(),
            (9, "dorothy", "The top thousand...").into(),
            (10, "bethany", "Beware of...").into(),
        ]
        .into_iter()
        .map(|article: Article| (article.id, article))
        .collect::<HashMap<_, _>>()
    });
}

#[cfg(feature = "ssr")]
use server::*;

pub mod navigation {
    use super::*;
    use leptos_sync_ssr::portlet::PortletCtx;

    #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
    pub struct NavItem {
        pub href: String,
        pub text: String,
    }

    #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
    pub struct NavItems(Vec<NavItem>);

    impl From<Vec<NavItem>> for NavItems {
        fn from(value: Vec<NavItem>) -> Self {
            Self(value)
        }
    }

    impl NavItems {
        pub fn into_inner(self) -> Vec<NavItem> {
            self.0
        }
    }

    pub type NavPortletCtx = PortletCtx<NavItems>;

    impl IntoRender for NavItems {
        type Output = AnyView;

        fn into_render(self) -> Self::Output {
            view! {
                <section id="NavPortlet">
                    <heading>"Navigation"</heading>
                    <nav>{
                        self.into_inner()
                            .into_iter()
                            .map(|NavItem { href, text }| {
                                view! {
                                    <A href=href>{text}</A>
                                }
                            })
                            .collect_view()
                    }</nav>
                </section>
            }
            .into_any()
        }
    }

    #[component]
    pub fn NavPortlet() -> impl IntoView {
        NavPortletCtx::render()
    }
}

use navigation::*;

pub mod info {
    use super::*;
    use leptos_sync_ssr::portlet::PortletCtx;

    #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
    pub struct Info {
        pub entity: String,
        pub id: String,
    }

    pub type InfoPortletCtx = PortletCtx<Info>;

    impl IntoRender for Info {
        type Output = AnyView;

        fn into_render(self) -> Self::Output {
            view! {
                <section id="Info">
                    <heading>"Info"</heading>
                    <dl>
                        <dt>"Entity"</dt>
                        <dd id="info_entity">{self.entity}</dd>
                        <dt>"id"</dt>
                        <dd id="info_id">{self.id}</dd>
                    </dl>
                </section>
            }
            .into_any()
        }
    }

    #[component]
    pub fn InfoPortlet() -> impl IntoView {
        InfoPortletCtx::render()
    }
}

use info::*;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    // leptos::logging::log!(">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>");
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[server]
async fn list_authors() -> Result<Vec<(String, Author)>, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(TIMEOUT)).await;
    Ok(AUTHORS
        .iter()
        .map(|(k, author)| (k.to_string(), author.clone()))
        .collect::<Vec<_>>())
}

#[server]
async fn get_author(name: String) -> Result<(String, Author), ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(TIMEOUT)).await;
    AUTHORS
        .get_key_value(name.as_str())
        .map(|(k, author)| (k.to_string(), author.clone()))
        .ok_or_else(|| ServerFnError::ServerError(format!("no such author: {name}")))
}

#[server]
async fn list_articles() -> Result<Vec<(u32, Article)>, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(TIMEOUT)).await;
    Ok(ARTICLES
        .iter()
        .map(|(k, article)| (k.to_owned(), article.clone()))
        .collect::<Vec<_>>())
}

#[server]
async fn list_articles_by_author(name: String) -> Result<Vec<(u32, Article)>, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(TIMEOUT)).await;
    Ok(ARTICLES
        .iter()
        .filter_map(|(id, article)| {
            (article.author_name == name).then(|| (id.to_owned(), article.clone()))
        })
        .collect::<Vec<_>>())
}

#[server]
async fn get_article(id: u32) -> Result<Article, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(TIMEOUT)).await;
    ARTICLES
        .get(&id)
        .map(Article::clone)
        .ok_or_else(|| ServerFnError::ServerError(format!("no such article: {id}")))
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    let fallback = || view! { "Page not found." }.into_view();

    view! {
        <Stylesheet id="leptos" href="/pkg/nav_portlet.css"/>
        <Title text="Reactive Portlets in Leptos"/>
        <Meta name="color-scheme" content="dark light"/>
        <Router>
            <header>
                <div id="notice">
                    "This WASM application has panicked, please refer to the console log for details. "
                    <a href="/" target="_self">"Use this link to restart the application"</a>"."
                </div>
                <nav>
                    <A href="/">"Home"</A>
                    <A href="/author/">"Authors"</A>
                    <A href="/article/">"Articles"</A>
                </nav>
            </header>
            <SyncSsrSignal setup=|| {
                NavPortletCtx::provide();
                InfoPortletCtx::provide();
            }>
                <main>
                    <aside>
                        <NavPortlet/>
                        <InfoPortlet/>
                    </aside>
                    <article>
                        <Routes fallback>
                            <Route path=path!("") view=HomePage/>
                            <AuthorRoutes/>
                            <ArticleRoutes/>
                        </Routes>
                    </article>
                    // Uncomment this aside after should work also
                    // <aside>
                    //     <NavPortlet/>
                    // </aside>
                </main>
            </SyncSsrSignal>
        </Router>
    }
}

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <Title text="Home Page"/>
        <h1>"Home Page"</h1>
        <ul>
            <li><a href="/author/">"Authors"</a></li>
            <li><a href="/article/">"Articles"</a></li>
        </ul>
    }
}

#[component]
pub fn AuthorRoutes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=StaticSegment("author") view=AuthorContainer ssr=SsrMode::Async>
            <Route path=StaticSegment("/") view=AuthorListing/>
            <ParentRoute path=ParamSegment("name") view=AuthorTop>
                <Route path=StaticSegment("/") view=AuthorOverview/>
                <Route path=StaticSegment("articles") view=ArticleListing/>
            </ParentRoute>
        </ParentRoute>
    }
    .into_inner()
}

#[component]
pub fn AuthorContainer() -> impl IntoView {
    provide_context(ArcResource::new_blocking(
        move || (),
        move |_| async move { list_authors().await },
    ));

    view! {
        <h2>"<AuthorContainer/>"</h2>
        <Outlet/>
    }
}

#[component]
pub fn AuthorListing() -> impl IntoView {
    let resource = expect_context::<ArcResource<Result<Vec<(String, Author)>, ServerFnError>>>();
    let author_listing = move || {
        let resource = resource.clone();
        Suspend::new(async move {
            resource.await.map(|authors| {
                authors
                    .into_iter()
                    .map(move |(id, author)| {
                        view! {
                            <li><a href=format!("/author/{id}/")>{author.name}</a></li>
                        }
                    })
                    .collect_view()
            })
        })
    };

    view! {
        <h3>"<AuthorListing/>"</h3>
        <ul>
            <Suspense>{author_listing}</Suspense>
        </ul>
    }
}

#[derive(Params, PartialEq, Clone, Debug)]
struct AuthorTopParams {
    name: Option<String>,
}

#[component]
pub fn AuthorTop() -> impl IntoView {
    let params = use_params::<AuthorTopParams>();
    provide_context(ArcResource::new_blocking(
        move || params.get().map(|p| p.name),
        move |name| async move {
            match name {
                Ok(Some(name)) => get_author(name).await,
                _ => Err(ServerFnError::ServerError("parameter error".to_string())),
            }
        },
    ));
    provide_context(ArcResource::new_blocking(
        move || params.get().map(|p| p.name),
        move |name| async move {
            match name {
                Ok(Some(name)) => list_articles_by_author(name).await,
                _ => Err(ServerFnError::ServerError("parameter error".to_string())),
            }
        },
    ));

    let author = expect_context::<ArcResource<Result<(String, Author), ServerFnError>>>();
    let authors = expect_context::<ArcResource<Result<Vec<(String, Author)>, ServerFnError>>>();
    let nav_ctx = expect_context::<NavPortletCtx>();
    let info_ctx = expect_context::<InfoPortletCtx>();

    #[cfg(not(feature="ssr"))]
    on_cleanup({
        let nav_ctx = nav_ctx.clone();
        let info_ctx = info_ctx.clone();
        move || {
            nav_ctx.clear();
            info_ctx.clear();
        }
    });

    view! {
        {nav_ctx.update_with(move || {
            let authors = authors.clone();
            #[cfg(not(feature = "ssr"))]
            authors.track();
            async move {
                authors.await
                    .map(|authors| {
                        authors
                            .into_iter()
                            .map(move |(id, author)| NavItem {
                                href: format!("/author/{id}/"),
                                text: author.name.to_string(),
                            })
                            .collect::<Vec<_>>()
                            .into()
                    })
                    .ok()
            }
        })}
        {info_ctx.update_with(move || {
            let author = author.clone();
            #[cfg(not(feature = "ssr"))]
            author.track();
            async move {
                author.await
                    .map(|(id, _)| Info {
                        entity: "Author".to_string(),
                        id,
                    })
                    .ok()
            }
        })}
        <h3>"<AuthorTop/>"</h3>
        <Outlet/>
    }
}

#[component]
pub fn AuthorOverview() -> impl IntoView {
    let resource = expect_context::<ArcResource<Result<(String, Author), ServerFnError>>>();
    let author = move || {
        let resource = resource.clone();
        Suspend::new(async move {
            resource.await.map(move |(id, author)| {
                view! {
                    <dl id="author-overview">
                        <dt>"ID:"</dt>
                        <dd>{id}</dd>
                        <dt>"Name:"</dt>
                        <dd>{author.name}</dd>
                        <dt>"Email:"</dt>
                        <dd>{author.email}</dd>
                    </dl>
                    <ul>
                        <li><a href="articles">"Articles by this author"</a></li>
                    </ul>
                }
            })
        })
    };

    view! {
        <h4>"<AuthorOverview/>"</h4>
        <Suspense>{author}</Suspense>
    }
}

#[component]
pub fn ArticleRoutes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=StaticSegment("article") view=ArticleContainer ssr=SsrMode::Async>
            <Route path=StaticSegment("/") view=ArticleListing/>
            <ParentRoute path=ParamSegment("id") view=ArticleTop>
                <Route path=StaticSegment("/") view=ArticleView/>
                <Route path=StaticSegment("comments") view=ArticleComments/>
                <Route path=StaticSegment("history") view=ArticleHistory/>
            </ParentRoute>
        </ParentRoute>
    }
    .into_inner()
}

#[component]
pub fn ArticleContainer() -> impl IntoView {
    provide_context(ArcResource::new_blocking(
        move || (),
        move |_| async move { list_articles().await },
    ));

    view! {
        <h2>"<ArticleContainer/>"</h2>
        <Outlet/>
    }
}

#[component]
pub fn ArticleListing() -> impl IntoView {
    let resource = expect_context::<ArcResource<Result<Vec<(u32, Article)>, ServerFnError>>>();
    let article_listing = move || {
        let resource = resource.clone();
        Suspend::new(async move {
            resource.await.map(|articles| {
                articles
                    .into_iter()
                    .map(move |(id, article)| {
                        view! {
                            <li><a href=format!("/article/{id}/")>{article.title}</a></li>
                        }
                    })
                    .collect_view()
            })
        })
    };

    view! {
        <h3>"<ArticleListing/>"</h3>
        <ul>
            <Suspense>{article_listing}</Suspense>
        </ul>
    }
}

#[derive(Params, PartialEq, Clone, Debug)]
struct ArticleTopParams {
    id: Option<u32>,
}

#[component]
pub fn ArticleTop() -> impl IntoView {
    let params = use_params::<ArticleTopParams>();
    provide_context(ArcResource::new_blocking(
        move || params.get().map(|p| p.id),
        move |id| async move {
            match id {
                Ok(Some(id)) => get_article(id).await,
                _ => Err(ServerFnError::ServerError("parameter error".to_string())),
            }
        },
    ));

    let articles = expect_context::<ArcResource<Result<Vec<(u32, Article)>, ServerFnError>>>();
    let article = expect_context::<ArcResource<Result<Article, ServerFnError>>>();
    let nav_ctx = expect_context::<NavPortletCtx>();
    let info_ctx = expect_context::<InfoPortletCtx>();

    on_cleanup({
        let nav_ctx = nav_ctx.clone();
        let info_ctx = info_ctx.clone();
        move || {
            // leptos::logging::log!("<ArticleTop> on_cleanup");
            nav_ctx.clear();
            info_ctx.clear();
        }
    });

    view! {
        {nav_ctx.update_with(move || {
            let articles = articles.clone();
            #[cfg(not(feature = "ssr"))]
            articles.track();
            async move {
                articles.await
                    .map(|articles| {
                        articles
                            .into_iter()
                            .map(move |(id, article)| NavItem {
                                href: format!("/article/{id}/"),
                                text: article.title.to_string(),
                            })
                            .collect::<Vec<_>>()
                            .into()
                    })
                    .ok()
            }
        })}
        {info_ctx.update_with(move || {
            let article = article.clone();
            #[cfg(not(feature = "ssr"))]
            article.track();
            async move {
                article.await
                    .map(|article| Info {
                        entity: "Article".to_string(),
                        id: article.id.to_string(),
                    })
                    .ok()
            }
        })}
        <h3>"<ArticleTop/>"</h3>
        <Outlet/>
    }
}

#[component]
pub fn ArticleView() -> impl IntoView {
    let resource = expect_context::<ArcResource<Result<Article, ServerFnError>>>();
    let article = move || {
        let resource = resource.clone();
        Suspend::new(async move {
            resource.await.map(move |article| {
                let author_href = format!("/author/{}/", article.author_name);
                view! {
                    <dl id="article-view">
                        <dt>"Title:"</dt>
                        <dd>{article.title}</dd>
                        <dt>"Author:"</dt>
                        <dd>
                            <a href=author_href>{article.author_name}</a>
                        </dd>
                    </dl>
                    <ul>
                        <li><a href="comments">"Comments"</a></li>
                        <li><a href="history">"Article History"</a></li>
                    </ul>
                }
            })
        })
    };

    view! {
        <h4>"<ArticleView/>"</h4>
        <Suspense>{article}</Suspense>
    }
}

#[component]
pub fn ArticleComments() -> impl IntoView {
    let resource = expect_context::<ArcResource<Result<Article, ServerFnError>>>();
    let article = move || {
        let resource = resource.clone();
        Suspend::new(async move {
            resource.await.map(move |article| {
                view! {
                    <h5>"Comments on article: "{article.title}</h5>
                    <p><A href="..">"Back to article"</A></p>
                }
            })
        })
    };

    view! {
        <h4>"<ArticleComments/>"</h4>
        <Suspense>{article}</Suspense>
    }
}

#[component]
pub fn ArticleHistory() -> impl IntoView {
    let resource = expect_context::<ArcResource<Result<Article, ServerFnError>>>();
    let article = move || {
        let resource = resource.clone();
        Suspend::new(async move {
            resource.await.map(move |article| {
                view! {
                    <h5>"History of "{article.title}</h5>
                    <p><A href="..">"Back to article"</A></p>
                }
            })
        })
    };

    view! {
        <h4>"<ArticleHistory/>"</h4>
        <Suspense>{article}</Suspense>
    }
}
