#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate quick_error;
extern crate url;
extern crate futures;
extern crate tokio_core;
#[macro_use]
extern crate tokio_io;
extern crate tokio_dns;

mod config;
mod proxy;

use config::{parse_args};
use proxy::{run_tunnel};
use std::process::exit;
use std::io::{self, Write};
use tokio_core::reactor::Core;
use futures::{future, Future};

fn main() {
    
    let config=match parse_args() {
        Err(e) => {
            writeln!(&mut io::stderr(), "Arguments error: {:?}",e).unwrap();
            exit(1)
        }
        Ok(c) => c
    };
    debug!("Started with following config {:?}", config);

    
    let mut reactor = Core::new().unwrap();
    let mut servers: Box<Future<Item=(), Error=std::io::Error>> = Box::new(future::ok(()));
    for t in config.tunnels {
        debug!("Staring tunnel {:?}", t);
        let server = run_tunnel(reactor.handle(), t, config.proxy.clone());
        servers = Box::new(servers.join(server).map(|_| ()));
    }
    
    reactor.run(servers).unwrap();

}
