    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("Invalid proxy specification")]
        InvalidProxy,
        #[error("Invalid tunnel specification")]
        InvalidTunnel,
        #[error("Invalid port number")]
        InvalidPort(#[from] std::num::ParseIntError),
        #[error("Invalid IP address")]
        InvalidAddress(#[from] std::net::AddrParseError),
        #[error("Problem with address resolution: {0}")]
        AddressResolution(#[from] std::io::Error),
    }
    
    
    pub type Result<T> = ::std::result::Result<T, Error>;