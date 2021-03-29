//! ```shell
//! s3-server 0.1.0-dev
//!
//! USAGE:
//!     s3-server [OPTIONS]
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

use s3_server::storages::fs::FileSystem;
use s3_server::S3Service;
use s3_server::SimpleAuth;

use std::net::TcpListener;
use std::path::PathBuf;

use anyhow::Result;
use futures::future;
use hyper::server::Server;
use hyper::service::make_service_fn;
use structopt::StructOpt;
use tracing::{debug, info};

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

pub fn setup_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{fmt, EnvFilter};

    tracing_subscriber::fmt()
        .event_format(fmt::format::Format::default().pretty())
        .with_env_filter(EnvFilter::from_default_env())
        .with_timer(fmt::time::ChronoLocal::rfc3339())
        .finish()
        .with(ErrorLayer::default())
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    setup_tracing();

    let args: Args = Args::from_args();

    // setup the storage
    let fs = FileSystem::new(&args.fs_root)?;
    debug!(?fs);

    // setup the service
    let mut service = S3Service::new(fs);

    if let (Some(access_key), Some(secret_key)) = (args.access_key, args.secret_key) {
        let mut auth = SimpleAuth::new();
        auth.register(access_key, secret_key);
        debug!(?auth);
        service.set_auth(auth);
    }

    let server = {
        let service = service.into_shared();
        let listener = TcpListener::bind((args.host.as_str(), args.port))?;
        let make_service: _ =
            make_service_fn(move |_| future::ready(Ok::<_, anyhow::Error>(service.clone())));
        Server::from_tcp(listener)?.serve(make_service)
    };

    info!("server is running at http://{}:{}/", args.host, args.port);
    server.await?;

    Ok(())
}
