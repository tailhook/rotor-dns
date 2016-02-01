extern crate ip;
extern crate rotor;
extern crate time;
extern crate rand;
extern crate void;
extern crate dns_parser;
extern crate resolv_conf;
#[macro_use] extern crate quick_error;

mod error;
mod config;
mod fsm;
mod resolver;

use std::io;
use std::marker::PhantomData;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use rotor::{EarlyScope, PollOpt, EventSet, Notifier};
use rotor::mio::udp::UdpSocket;

pub use config::Config;
pub use error::Error;
pub use dns_parser::QueryType;
type Id = u16;


/// Human friendly query types
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Query {
    /// Simple host lookup (A record)
    LookupIpv4(String),
}

#[derive(Debug)]
pub enum Answer {
    Ipv4(Vec<Ipv4Addr>),
}

struct Request {
    id: Id,
    query: Query,
    server: SocketAddr,
    // deadline: time::SteadyTime, TODO(tailhook) implement deadlines
    notifiers: Vec<(Arc<Mutex<Option<Arc<CacheEntry>>>>, Notifier)>,
}

#[derive(Debug)]
pub struct CacheEntry {
    pub value: Answer,
    pub expire: time::SteadyTime,
}

struct DnsMachine {
    config: Config,
    running: HashMap<Id, Request>,
    // ueued: HashMap<Query, Id>,  TODO(tailhook) implement duplicate checking
    cache: HashMap<Query, Arc<CacheEntry>>,
    sock: UdpSocket,
}

pub struct Fsm<C>(Arc<Mutex<DnsMachine>>, PhantomData<*const C>);
pub struct Resolver(Arc<Mutex<DnsMachine>>);

pub fn create_resolver<C>(scope: &mut EarlyScope, config: Config)
    -> Result<(Resolver, Fsm<C>), io::Error>
{
    let machine = DnsMachine {
        config: config,
        running: HashMap::new(),
        // TODO(tailhook) implement duplicate checking
        // queued: HashMap::new(),
        cache: HashMap::new(),
        sock: try!(UdpSocket::bound(&SocketAddr::V4(
            SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0)))),
    };
    try!(scope.register(&machine.sock,
        EventSet::readable(), PollOpt::level()));
    let arc = Arc::new(Mutex::new(machine));
    Ok((Resolver(arc.clone()), Fsm(arc.clone(), PhantomData)))
}
