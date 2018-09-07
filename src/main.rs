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
extern crate tokio;
#[macro_use]
extern crate tokio_io;
extern crate tokio_dns;
extern crate data_encoding;

mod config;
mod proxy;

use config::{parse_args};
use proxy::{run_tunnel};
use std::process::exit;
use std::io::{self, Write};
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

    let user_encoded = config.user.map(|u| u.encoded());
    let mut servers: Box<Future<Item=(), Error=std::io::Error>+Send> = Box::new(future::ok(()));
    for t in config.tunnels {
        debug!("Staring tunnel {}:{:?} on ", config.local_addr,t);
        let server = run_tunnel(
                config.local_addr.clone(), 
                t, 
                config.proxy.clone(),
                user_encoded.clone());
        servers = Box::new(servers.join(server).map(|_| ()));
    }

    let servers = servers.map_err(|e| error!("Error in proxy connection {}", e));
    
    if config.multithreaded {
        debug!("Running in thread pool");
        tokio::run(servers);
    } else {
        debug!("Running in current thread");
        let mut rt = tokio::runtime::current_thread::Runtime::new().unwrap();
        let _ = rt.block_on(servers); //ignore error
        
    }

}
