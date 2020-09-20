//! ```shell
//! s3-server 0.1.0-dev
//!
//! USAGE:
//!     s3 [OPTIONS]
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! OPTIONS:
//!         --fs-root <fs-root>           [default: .]
//!         --host <host>                 [default: localhost]
//!         --port <port>                 [default: 8014]
//!         --access-key <access-key>    
//!         --secret-key <secret-key>
//! ```

use s3_server::S3Service;
use s3_server::{fs::TokioFileSystem as FileSystem, SimpleAuth};

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

    #[structopt(long, requires("secret-key"), display_order = 1000)]
    access_key: Option<String>,

    #[structopt(long, requires("access-key"), display_order = 1000)]
    secret_key: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args: Args = Args::from_args();

    let fs = FileSystem::new(&args.fs_root)?;
    log::debug!("fs: {:?}", &fs);

    let service = match (args.access_key, args.secret_key) {
        (None, None) => S3Service::new(fs),
        (Some(access_key), Some(secret_key)) => {
            let mut auth = SimpleAuth::new();
            auth.register(access_key, secret_key);
            log::debug!("auth: {:?}", &auth);
            S3Service::with_auth(fs, auth)
        }
        _ => unreachable!(),
    }
    .into_shared();

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
