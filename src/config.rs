use std::io;
use std::io::Read;
use std::fs::File;
use std::net::SocketAddr;
use std::path::PathBuf;

use ip::SocketAddrExt;
use resolv_conf::{Config as Resolv};


quick_error! {
    #[derive(Debug)]
    pub enum SystemConfigError {
        FileError(path: PathBuf, err: io::Error) {
            description("error parsing reading system config file")
            display("Error reading {:?}: {}", path, err)
            cause(err)
        }
        ResolvConf(err: u32 /*crappy resolv-conf*/) {
            description("Error parsing resolv.conf")
            from()
        }
    }
}


pub struct Config {
    nameservers: Vec<SocketAddr>,
}


impl Config {
    pub fn system() -> Result<Config, SystemConfigError> {
        use self::SystemConfigError::*;
        let mut buf = Vec::with_capacity(512);
        try!(File::open("/etc/resolv.conf")
            .and_then(|mut f| f.read_to_end(&mut buf))
            .map_err(|e| FileError("/etc/resolv.conf".into(), e)));
        let cfg = try!(Resolv::parse(&buf));
        Ok(Config {
            nameservers: cfg.nameservers.iter()
                .map(|ns| <SocketAddr as SocketAddrExt>::new(*ns, 53))
                .collect(),
        })
    }
}
