use clap::{Arg, App};
use env_logger::{LogBuilder};
use log::{LogLevelFilter};
use std::str::FromStr;
use std::env;
use url::Url;

lazy_static! {
    static ref PROGRAM_NAME:&'static str = option_env!("CARGO_PKG_NAME").unwrap_or("ptunnel");
    static ref PROGRAM_VERSION:Option<&'static str> = option_env!("CARGO_PKG_VERSION");

}


quick_error! { 
#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidProxy {
        description("Invalid proxy specification")
    }
    InvalidTunnel {
        description("Invalid tunnel specification")
    }

    InvalidPort(err: ::std::num::ParseIntError) {
        from()
    }
}
}

type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Tunnel {
    pub local_port: u16,
    pub remote_port: u16,
    pub remote_host: String
}

impl Tunnel {
    pub fn remote(&self) -> String {
        format!("{}:{}", self.remote_host, self.remote_port)
    }
}

#[derive(Debug, PartialEq)]
pub struct Proxy {
    pub host: String,
    pub port: u16
}
#[derive(Debug)]
pub struct Config {
    pub log_level: LogLevelFilter,
    pub proxy: Option<Proxy>,
    pub tunnels: Vec<Tunnel>
}

type Parser<'a> = App<'a, 'a>;

fn create_parser<'a>() -> Parser<'a> {
    let mut arg_parser = App::new(*PROGRAM_NAME);
    if let Some(ver) = *PROGRAM_VERSION {
        arg_parser =arg_parser.version(ver);
    }

    arg_parser
    .author(crate_authors!())
    .about(crate_description!())
    .arg(
        Arg::with_name("quiet")
        .short("q")
        .long("quiet")
        .help("absolutely quite - logging off even for errors")
        )
    .arg(Arg::with_name("verbose")
        .short("v")
        .long("verbose")
        .multiple(true)
        .conflicts_with("quiet")
        )
    .arg(Arg::with_name("proxy")
        .short("p")
        .long("proxy")
        .takes_value(true)
        .value_name("HOST:PORT")
        .help("https proxy (accepting CONNECT method), specify as host:port, if not specified https_proxy environment var is used")
    )
    .arg(Arg::with_name("tunnel")
        .value_name("LOCAL_POST:REMOTE_HOST:REMOTE_PORT")
        .help("tunnel specfication in form of local_port:remote_host:remote_port")
        .required(true)
        .multiple(true)
        )

}

fn config_log_level(level: LogLevelFilter) {
    let mut log_builder = LogBuilder::new();
    log_builder.filter(None, level)
        .filter(Some("tokio_core"), LogLevelFilter::Warn)
        .filter(Some("mio"), LogLevelFilter::Warn);
    log_builder.init().unwrap();
}

fn parse_proxy(proxy:&str) -> Result<Proxy> {
    let parts = proxy.split(':').collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(Error::InvalidProxy);
    }
    let port = u16::from_str(parts[1])?;
    Ok(Proxy{
        host: parts[0].to_owned(),
        port
    })
}

fn parse_proxy_from_uri(url_in:&str) -> Result<Proxy> {
    let u = Url::parse(url_in).map_err(|_| Error::InvalidProxy)?;
    let host = match u.host_str() {
        Some(h) => h.to_owned(),
        None => {
            error!("host is missing in proxy url {}",url_in);
            return Err(Error::InvalidProxy)
        }
    };
    let port = match u.port() {
        Some(p) => p,
        None => 80
    };
    Ok(Proxy{host,port})
}

fn get_any_env_var(vars: &[&str]) -> Option<String> {
    for name in vars.into_iter() {
        if let Ok(p) = env::var(name) {
            return Some(p)
        }
    }
    None
}

fn parse_tunnel(t: &str) -> Result<Tunnel> {
    let parts = t.split(':').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(Error::InvalidTunnel)
    }

    Ok(Tunnel{
        local_port: u16::from_str(parts[0])?,
        remote_host: parts[1].into(),
        remote_port: u16::from_str(parts[2])?
    })
}

pub fn parse_args() -> Result<Config>{
    let p = create_parser();
    let args = p.get_matches();

    let log_level = if args.is_present("quiet") {
        LogLevelFilter::Off
    } else {
        match args.occurrences_of("verbose") {
            0 => LogLevelFilter::Error,
            1 => LogLevelFilter::Warn,
            2 => LogLevelFilter::Info,
            3 => LogLevelFilter::Debug,
            _ => LogLevelFilter::Trace
        }
    };

    config_log_level(log_level);
    debug!("Arguments are {:?}", args);

    let proxy = match args.value_of("proxy") {
        Some(p) => Some(parse_proxy(p)?),
        None => {
            get_any_env_var(&["https_proxy", "HTTPS_PROXY"]).
            and_then(|p| parse_proxy_from_uri(&p)
                .map_err(|e| {error!("Environment proxy is invalid");e})
                .ok())
        }
    };

    let mut tunnels = vec![];
    for t in args.values_of("tunnel").unwrap() {
        tunnels.push(parse_tunnel(t)?)
    }

   Ok(Config{log_level, proxy, tunnels})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_proxy() {
        let proxy = "example.com:8080";
        let parsed = parse_proxy(proxy).unwrap();
        assert_eq!(parsed, Proxy{host: "example.com".into(), port:8080});

        assert_eq!(parse_proxy("spatenka"), Err(Error::InvalidProxy));
    }

    #[test]
    fn test_parse_proxy_from_uri() {
        let proxy = "http://proxy.example.com:8080";
        assert_eq!(parse_proxy_from_uri(proxy).unwrap(), 
        Proxy{host: "proxy.example.com".into(), port:8080 });
        assert_eq!(parse_proxy_from_uri("spatenka"), Err(Error::InvalidProxy));
    }

    #[test]
    fn test_parse_tunnel() {
        let t = "2121:mail.example.com:21";
        let parsed = parse_tunnel(t).unwrap();
        assert_eq!(parsed, Tunnel{local_port:2121, remote_host:"mail.example.com".into(), remote_port:21});
        match parse_tunnel("host:1:2") {
            Err(Error::InvalidPort(_)) => (),
            _ => panic!("Should return invalid port error")

    }
    }
}