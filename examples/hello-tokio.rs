use anyhow::Result;
use futures::future;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::net::TcpListener;
use std::sync::atomic::{self, AtomicU64};
use std::time::Duration;
use tokio::task;
use tokio::time;

static TOTAL_REQUESTS: AtomicU64 = atomic::AtomicU64::new(0);

async fn hello(_req: Request<Body>) -> Result<Response<Body>> {
    TOTAL_REQUESTS.fetch_add(1, atomic::Ordering::SeqCst);
    let text = "Hello, world!\n";
    Ok(Response::new(Body::from(text)))
}

async fn monitor() {
    loop {
        time::delay_for(Duration::from_secs(1)).await;
        let total_requests = TOTAL_REQUESTS.load(atomic::Ordering::SeqCst);
        eprintln!("total requests: {:>8}", total_requests);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:8080";

    let listener = TcpListener::bind(&addr)?;

    let make_service: _ =
        make_service_fn(move |_| future::ready(Ok::<_, anyhow::Error>(service_fn(hello))));

    let server = Server::from_tcp(listener)?.serve(make_service);

    task::spawn(monitor());

    eprintln!("server is listening on {}", addr);
    server.await?;

    Ok(())
}
