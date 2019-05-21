use clap::{Arg, App};
use env_logger::{Builder};
use log::{LevelFilter};
use std::str::FromStr;
use std::env;
use url::Url;
use tokio_dns::{ToEndpoint, Endpoint};
use std::net::IpAddr;
use data_encoding::BASE64;

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

    InvalidAddress(err: ::std::net::AddrParseError) {
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

impl <'a>ToEndpoint<'a> for &'a Tunnel {
    fn to_endpoint(self) -> ::std::io::Result<Endpoint<'a>> {
        Ok(Endpoint::Host(&self.remote_host, self.remote_port))
    }
}

impl Tunnel {
    pub fn remote(&self) -> String {
        format!("{}:{}", self.remote_host, self.remote_port)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Proxy {
    pub host: String,
    pub port: u16
}

#[derive(Debug,Clone)]
pub struct User {
    pub name: String,
    pub password: Option<String>,
}

impl User {
    pub fn encoded(&self) -> String {
        match self.password.as_ref() {
            None => BASE64.encode(&self.name.as_bytes()),
            Some(p) => {
                let s = format!("{}:{}", self.name, p);
                BASE64.encode(s.as_bytes())
            }
        }
        
    }
}

#[derive(Debug)]
pub struct Config {
    pub log_level: LevelFilter,
    pub local_addr: IpAddr,
    pub proxy: Option<Proxy>,
    pub tunnels: Vec<Tunnel>,
    pub user: Option<User>,
    pub multithreaded: bool
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
        .help("verbosity of logging - can be used multiple times to increase verbosity")
        )
    .arg(Arg::with_name("listen")
        .short("l")
        .long("listen")
        .takes_value(true)
        .help("local address to listen on - default is 127.0.0.1")
    )
    .arg(Arg::with_name("proxy")
        .short("p")
        .long("proxy")
        .takes_value(true)
        .value_name("HOST:PORT")
        .help("https proxy (accepting CONNECT method), specify as host:port, if not specified https_proxy environment var is used")
    )
    .arg(Arg::with_name("user")
        .short("U")
        .long("user")
        .takes_value(true)
        .help("Proxy username - for basic authentication to proxy")
    )
    .arg(Arg::with_name("password")
        .short("P")
        .long("password")
        .takes_value(true)
        .help("Proxy user password - for basic authentication to proxy")
        .requires("user")
    )
    .arg(Arg::with_name("multithreaded")
        .short("m")
        .long("multithreaded")
        .help("Runs multithreaded - normally not needed")
    )
    .arg(Arg::with_name("tunnel")
        .value_name("LOCAL_PORT:REMOTE_HOST:REMOTE_PORT")
        .help("tunnel specfication in form of local_port:remote_host:remote_port")
        .required(true)
        .multiple(true)
        )

}

fn config_log_level(level: LevelFilter) {
    let mut log_builder = Builder::new();
    log_builder.filter(None, level)
        .filter(Some("tokio"), LevelFilter::Warn)
        .filter(Some("mio"), LevelFilter::Warn);
    log_builder.init();
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
        LevelFilter::Off
    } else {
        match args.occurrences_of("verbose") {
            0 => LevelFilter::Error,
            1 => LevelFilter::Warn,
            2 => LevelFilter::Info,
            3 => LevelFilter::Debug,
            _ => LevelFilter::Trace
        }
    };

    config_log_level(log_level);
    debug!("Arguments are {:?}", args);

    let local_addr = match args.value_of("listen") {
        None => "127.0.0.1".parse().unwrap(),
        Some(s) => s.parse()?

    };

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

    let user = match args.value_of("user") {
        None => None,
        Some(name) => Some(User{name:name.into(), password: args.value_of("password").map(|s| s.into())})
    };

    let multithreaded = args.is_present("multithreaded");

   Ok(Config{log_level, proxy, tunnels, local_addr, user, multithreaded})
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

    #[test]
    fn test_user_encoded() {
        let u = User{name:"Aladdin".into(), password: Some("OpenSesame".into())};
        let e = u.encoded();
        assert_eq!("QWxhZGRpbjpPcGVuU2VzYW1l", &e);
    }
}