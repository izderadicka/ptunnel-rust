#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate clap;
extern crate data_encoding;
extern crate futures;
extern crate tokio;
extern crate url;

mod config;
mod proxy;
mod error;

use config::parse_args;
use proxy::run_tunnel;
use tokio::runtime;
use std::process::exit;
use futures::{
    FutureExt,
    future
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = match parse_args() {
        Err(e) => {
            eprintln!("Arguments error: {:?}", e);
            exit(1)
        }
        Ok(c) => c,
    };
    debug!("Started with following config {:?}", config);

    let rt = if config.multithreaded {
        debug!("Running in thread pool");
        runtime::Runtime::new()?
    } else {
        debug!("Running in current thread");
        runtime::Builder::new_current_thread().enable_all().build()?
    };

    let user_encoded = config.user.as_ref().map(|u| u.encoded());

    rt.block_on(async move {
        let mut servers = vec![];
        for t in config.tunnels {
            debug!("Starting tunnel on local address {} {:?}", config.local_addr, t);
            let server = run_tunnel(
                config.local_addr,
                t,
                config.proxy.clone(),
                user_encoded.clone(),
            ).then(|r| {
                if let Err(e) = r {
                    error!("Error when creating tunnel: {}", e)
                }
                future::ready(())
            });

            servers.push(server);
        }
        futures::future::join_all(servers).await
    });

    Ok(())
}
