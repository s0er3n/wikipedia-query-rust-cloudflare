use select::document::Document;
use select::predicate::{Attr, Name};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use worker::*;

mod utils;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    );
}

#[derive(Clone, Serialize)]
struct Wiki {
    parse: Parse,
}

#[derive(Clone, Serialize)]
struct Parse {
    text: HashMap<String, String>,
    title: String,
}
async fn make_query(target: &str) -> Wiki {
    let resp_txt = reqwest::get(format!("https://en.wikipedia.org/wiki/{}", target))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let wiki_page = Document::from(resp_txt.as_str());

    let title = wiki_page.find(Name("h1")).next().unwrap().text();

    let article = wiki_page
        .find(Attr("id", "mw-content-text"))
        .next()
        .unwrap();

    // let links: HashSet<String> = article
    //     .find(Name("a"))
    //     .filter_map(|n| n.attr("href"))
    //     .filter(|l| {
    //         if l.starts_with("/wiki/Help") || l.starts_with("/wiki/File") {
    //             return false;
    //         };
    //         if l.starts_with("/wiki/") {
    //             return true;
    //         };
    //         false
    //     })
    //     .map(|x| String::from(&x[6..]))
    //     .collect();
    let content = article.html();
    let mut hashmap = HashMap::new();

    hashmap.insert("*".to_string(), content);

    return Wiki {
        parse: Parse {
            text: hashmap,
            title,
        },
    };
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);

    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

    // Optionally, use the Router to handle matching endpoints, use ":name" placeholders, or "*name"
    // catch-alls to match on specific patterns. Alternatively, use `Router::with_data(D)` to
    // provide arbitrary data that will be accessible in each route via the `ctx.data()` method.
    let router = Router::new();

    // Add as many routes as your Worker needs! Each route will get a `Request` for handling HTTP
    // functionality and a `RouteContext` which you can use to  and get route parameters and
    // Environment bindings like KV Stores, Durable Objects, Secrets, and Variables.
    router
        .get_async("/wiki/:article", |mut req, ctx| async move {
            if let Some(name) = ctx.param("article") {
                let cors = worker::Cors::new()
                    .with_origins(["*"])
                    .with_methods([Method::Get]);
                let response_json = make_query(name).await;
                return Response::from_json(&response_json)?.with_cors(&cors);
            }

            Response::error("Bad Request", 400)
        })
        .run(req, env)
        .await
}
