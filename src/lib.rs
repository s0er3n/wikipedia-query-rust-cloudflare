use reqwest::header::USER_AGENT;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tl::{queryselector::iterable::QueryIterable, HTMLTag, Node::Tag, NodeHandle, ParserOptions};
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
    console_log!("header = {:?}", &req.headers());
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
async fn make_query(target: &str, headers_req: &Headers) -> Wiki {
    let client = reqwest::Client::new();
    let resp_txt = client
        .get(format!(
            "https://en.wikipedia.org/wiki/{}?useskin=vector",
            target
        ))
        .header(USER_AGENT, &headers_req.get("user-agent").unwrap().unwrap())
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let dom = tl::parse(&resp_txt, ParserOptions::default()).unwrap();

    let parser = dom.parser();
    let title = dom.query_selector("h1").unwrap().next().unwrap();

    let title = NodeHandle::get(&title, parser).unwrap().inner_text(parser);

    // let title = wiki_page.find(Name("h1")).next().unwrap().text();
    let article = dom
        .get_element_by_id("mw-content-text")
        .unwrap()
        .get(parser)
        .unwrap()
        .inner_html(parser);

    // let see_also_tag = dom
    //     .query_selector("span#See_also")
    //     .unwrap()
    //     .next()
    //     .unwrap()
    //     .get(parser);

    let content = article;
    // match see_also_tag {
    //     Some(Tag(span)) => {
    //         let (mut start, _) = span.boundaries(parser);
    //         loop {
    //             start = start - 1;
    //             if resp_txt.chars().nth(start).unwrap() == '<' {
    //                 break;
    //             }
    //         }
    //         &resp_txt[..start]
    //     }
    //     _ => &resp_txt,
    // };

    // .find(Attr("id", "mw-content-text"))
    // .next()
    // .unwrap();

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
    let mut hashmap = HashMap::new();

    hashmap.insert("*".to_string(), content.to_string());

    return Wiki {
        parse: Parse {
            text: hashmap,
            title: title.into(),
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
                let response_json = make_query(name, req.headers()).await;
                return Response::from_json(&response_json)?.with_cors(&cors);
            }

            Response::error("Bad Request", 400)
        })
        .run(req, env)
        .await
}
