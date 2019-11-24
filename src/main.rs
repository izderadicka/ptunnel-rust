#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate quick_error;
extern crate data_encoding;
extern crate futures;
extern crate tokio;
extern crate url;

mod config;
mod proxy;

use config::parse_args;
use proxy::run_tunnel;
use std::io::{self, Write};
use std::process::exit;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = match parse_args() {
        Err(e) => {
            writeln!(&mut io::stderr(), "Arguments error: {:?}", e).unwrap();
            exit(1)
        }
        Ok(c) => c,
    };
    debug!("Started with following config {:?}", config);

    let mut builder = tokio::runtime::Builder::new();
    let mut rt = if config.multithreaded {
        debug!("Running in thread pool");
        builder.threaded_scheduler().enable_all().build()?
    } else {
        debug!("Running in current thread");
        builder.basic_scheduler().enable_all().build()?
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
            );

            servers.push(server);
        }
        futures::future::join_all(servers).await
    });

    Ok(())
}
