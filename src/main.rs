use async_std::net::{TcpStream, TcpListener};
use async_std::prelude::*;
use async_std::task;
use http_types::{Response, StatusCode};
use std::collections::HashMap;
use std::sync::Arc;

use ludwig::{ Context, EmptyContext, Application };
use serde_json::json;
use maplit::hashmap;

async fn hello(_context: EmptyContext) -> anyhow::Result<(u16, HashMap<&'static str, &'static str>, &'static str)> {
    Ok((201, hashmap!(
        "x-clacks-overhead" => "GNU/Terry Pratchett"
    ), "hello world"))
}

async fn world(_context: EmptyContext) -> anyhow::Result<serde_json::Value> {
    Ok(json!({
        "message": "hello world"
    }))
}

async fn how(_context: EmptyContext) -> Result<String, std::io::Error> {
    Ok(async_std::fs::read_to_string("/usr/share/dict/words").await?)
}

async fn are(_context: EmptyContext) -> &'static str {
    "okay, I guess"
}

async fn you(_context: EmptyContext) {
    println!("sometimes you just wanna 204 No Content! I'm not gonna judge")
}

#[async_std::main]
async fn main() -> http_types::Result<()> {

    // Open up a TCP connection and create a URL.
    let listener = TcpListener::bind(("127.0.0.1", 8080)).await?;
    let addr = format!("http://{}", listener.local_addr()?);
    println!("listening on {}", addr);

    let app = Arc::new(Application::new(())
        .route(("hello", "GET", "/hello", hello))
        .route(("world", "GET", "/world", world))
        .route(("how", "GET", "/how", how))
        .route(("are", "GET", "/are", are))
        .route(("you", "GET", "/you", you))
        );

    // For each incoming TCP connection, spawn a task and call `accept`.
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        let app = app.clone();
        task::spawn(async {
            if let Err(err) = accept(stream, app).await {
                eprintln!("{}", err);
            }
        });
    }
    Ok(())
}

use std::borrow::Borrow;
// Take a TCP stream, and convert it into sequential HTTP request / response pairs.
async fn accept<'a>(stream: TcpStream, app: Arc<Application<'a, (), ()>>) -> http_types::Result<()> {
    println!("starting new connection from {}", stream.peer_addr()?);
    let hurk = app.clone();
    async_h1::accept(stream.clone(), |req| async {
        if let Some(result) = app.execute(req).await {
            let mut res = Response::new(result.status);
            for (key, value) in result.headers {
                let key: &str = key.borrow();
                let value: &str = value.borrow();
                res.insert_header(key, value);
            }
            res.set_body(result.body);
            Ok(res)
        } else {
            let res = Response::new(404);
            Ok(res)
        }
    })
    .await?;
    Ok(())
}
