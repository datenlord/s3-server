//! ```shell
//! s3-server 0.1.0-dev
//!
//! USAGE:
//! s3 [OPTIONS]
//!
//! FLAGS:
//! -h, --help       Prints help information
//! -V, --version    Prints version information
//!
//! OPTIONS:
//!     --fs-root <fs-root>     [default: .]
//!     --host <host>           [default: localhost]
//!     --port <port>           [default: 8014]
//! ```

use s3_server::fs::TokioFileSystem as FileSystem;
use s3_server::S3Service;

use anyhow::Result;
use futures::future;
use hyper::server::Server;
use hyper::service::make_service_fn;
use std::net::TcpListener;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Args {
    #[structopt(long, default_value = ".")]
    fs_root: PathBuf,
    #[structopt(long, default_value = "localhost")]
    host: String,
    #[structopt(long, default_value = "8014")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args: Args = Args::from_args();

    let fs = FileSystem::new(&args.fs_root)?;
    log::debug!("fs: {:?}", &fs);

    let service = S3Service::new(fs).into_shared();

    let server = {
        let listener = TcpListener::bind((args.host.as_str(), args.port))?;
        let make_service: _ =
            make_service_fn(move |_| future::ready(Ok::<_, anyhow::Error>(service.clone())));
        Server::from_tcp(listener)?.serve(make_service)
    };

    log::info!("server is running at http://{}:{}/", args.host, args.port);
    server.await?;

    Ok(())
}
