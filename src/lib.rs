extern crate ip;
extern crate rotor;
extern crate time;
extern crate rand;
extern crate dns_parser;
extern crate resolv_conf;
#[macro_use] extern crate quick_error;

mod serialize;
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
    LookupHost(String),
}

pub enum Response {
    LookupResponse(Vec<String>),
}

struct Request {
    id: Id,
    query: Query,
    deadline: time::SteadyTime,
    notifiers: Vec<(Arc<Mutex<Option<Arc<CacheEntry>>>>, Notifier)>,
}

pub struct CacheEntry {
    pub value: Response,
    pub expire: time::SteadyTime,
}

struct DnsMachine {
    config: Config,
    running: HashMap<Id, Request>,
    queued: HashMap<Query, Id>,
    cache: HashMap<Query, Arc<CacheEntry>>,
    sock: UdpSocket,
}

pub struct Future {
    value: Option<Arc<CacheEntry>>,
}

pub struct Fsm<C>(Arc<Mutex<DnsMachine>>, PhantomData<*const C>);
pub struct Resolver(Arc<Mutex<DnsMachine>>);

pub fn create_resolver<C>(scope: &mut EarlyScope, config: Config)
    -> Result<(Resolver, Fsm<C>), io::Error>
{
    let machine = DnsMachine {
        config: config,
        running: HashMap::new(),
        queued: HashMap::new(),
        cache: HashMap::new(),
        sock: try!(UdpSocket::bound(&SocketAddr::V4(
            SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0)))),
    };
    try!(scope.register(&machine.sock,
        EventSet::readable(), PollOpt::level()));
    let arc = Arc::new(Mutex::new(machine));
    Ok((Resolver(arc.clone()), Fsm(arc.clone(), PhantomData)))
}
