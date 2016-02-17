use std::io;
use std::io::Read;
use std::fs::File;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use ip::SocketAddrExt;
use resolv_conf;


quick_error! {
    #[derive(Debug)]
    pub enum SystemConfigError {
        FileError(path: PathBuf, err: io::Error) {
            description("error parsing reading system config file")
            display("Error reading {:?}: {}", path, err)
            cause(err)
        }
        ResolvConf(err: resolv_conf::ParseError) {
            description("Error parsing resolv.conf")
            from()
        }
    }
}


pub struct Config {
    pub nameservers: Vec<SocketAddr>,
    pub timeout: Duration,
    pub attempts: u32,
}


impl Config {
    pub fn system() -> Result<Config, SystemConfigError> {
        use self::SystemConfigError::*;
        let mut buf = Vec::with_capacity(512);
        try!(File::open("/etc/resolv.conf")
            .and_then(|mut f| f.read_to_end(&mut buf))
            .map_err(|e| FileError("/etc/resolv.conf".into(), e)));
        let cfg = try!(resolv_conf::Config::parse(&buf));
        Ok(Config {
            nameservers: cfg.nameservers.iter()
                .map(|ns| <SocketAddr as SocketAddrExt>::new(*ns, 53))
                .collect(),
            timeout: Duration::new(cfg.timeout.into(), 0),
            attempts: cfg.attempts.into(),
        })
    }
}
