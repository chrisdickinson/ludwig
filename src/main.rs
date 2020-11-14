use async_std::net::{TcpStream, TcpListener};
use async_std::prelude::*;
use async_std::task;
use http_types::{Response, StatusCode};
use std::collections::HashMap;

use ludwig::{ Context, Handler };
use serde_json::json;

async fn hello(_context: Context<(), ()>) -> anyhow::Result<(u16, &'static str)> {
    Ok((201, "hello world"))
}

async fn world(_context: Context<(), ()>) -> anyhow::Result<serde_json::Value> {
    Ok(json!({
        "message": "hello world"
    }))
}

async fn how(_context: Context<(), ()>) -> Result<String, std::io::Error> {
    Ok(async_std::fs::read_to_string("/usr/share/dict/words").await?)
}

async fn are(_context: Context<(), ()>) -> &'static str {
    "okay, I guess"
}

async fn you(_context: Context<(), ()>) {
    println!("sometimes you just wanna 204 No Content! I'm not gonna judge")
}

#[async_std::main]
async fn main() -> http_types::Result<()> {

    // Open up a TCP connection and create a URL.
    let listener = TcpListener::bind(("127.0.0.1", 8080)).await?;
    let addr = format!("http://{}", listener.local_addr()?);
    println!("listening on {}", addr);

    // For each incoming TCP connection, spawn a task and call `accept`.
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        task::spawn(async {
            if let Err(err) = accept(stream).await {
                eprintln!("{}", err);
            }
        });
    }
    Ok(())
}

use std::borrow::Borrow;
// Take a TCP stream, and convert it into sequential HTTP request / response pairs.
async fn accept(stream: TcpStream) -> http_types::Result<()> {
    println!("starting new connection from {}", stream.peer_addr()?);
    async_h1::accept(stream.clone(), |_req| async move {
        let handler = Handler::<(), ()>::new("hello".to_string(), "GET".to_string(), "/".to_string(), hello)?;
        let handler = Handler::<(), ()>::new("hello".to_string(), "GET".to_string(), "/".to_string(), world)?;
        let handler = Handler::<(), ()>::new("hello".to_string(), "GET".to_string(), "/".to_string(), how)?;
        let handler = Handler::<(), ()>::new("hello".to_string(), "GET".to_string(), "/".to_string(), are)?;
        let handler = Handler::<(), ()>::new("hello".to_string(), "GET".to_string(), "/".to_string(), you)?;

        let context = Context::new(std::sync::Arc::new(()), (), HashMap::new());
        let result = (handler.action)(context).await;

        let mut res = Response::new(result.status);
        for (key, value) in result.headers {
            let key: &str = key.borrow();
            let value: &str = value.borrow();
            res.insert_header(key, value);
        }
        res.set_body(result.body);
        Ok(res)
    })
    .await?;
    Ok(())
}
