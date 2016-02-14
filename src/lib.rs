extern crate ip;
extern crate rotor;
extern crate time;
extern crate rand;
extern crate void;
extern crate dns_parser;
extern crate resolv_conf;
#[macro_use] extern crate quick_error;

mod config;
mod fsm;
mod resolver;
mod time_util;

use std::io;
use std::marker::PhantomData;
use std::collections::{HashMap, BinaryHeap};
use std::sync::{Arc, Mutex};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use rotor::{EarlyScope, PollOpt, EventSet, Notifier, Timeout};
use rotor::mio::udp::UdpSocket;

pub use config::Config;
pub use resolver::QueryError;
pub use dns_parser::QueryType;

type Id = u16;
#[derive(Debug)]
struct TimeEntry(time::SteadyTime, Id);


/// Human friendly query types
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Query {
    /// Simple host lookup (A record)
    LookupIpv4(String),
    /// Plain SRV record lookup
    LookupSrv(String),
    /// Plain MX record lookup
    LookupMx(String),
}

/// A generic DNS answer
#[derive(Debug)]
pub enum Answer {
    ServerUnavailable,
    Ipv4(Vec<Ipv4Addr>),
    Srv(Vec<SrvRecord>),
    Mx(Vec<MxRecord>),
}

/// Single SRV record
#[derive(Debug, Clone)]
pub struct SrvRecord {
    pub priority: u16,
    pub weight: u16,
    pub port: u16,
    pub target: String,
}

/// Single MX record
#[derive(Debug, Clone)]
pub struct MxRecord {
    pub preference: u16,
    pub exchange: String,
}

struct Request {
    id: Id,
    query: Query,
    nameserver_index: usize,
    attempts: u32,
    server: SocketAddr,
    deadline: time::SteadyTime,
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
    cache: HashMap<Query, Arc<CacheEntry>>,
    sock: UdpSocket,
    timeouts: BinaryHeap<TimeEntry>,
    timeout: Option<Timeout>,
    notifier: Notifier,
}

pub struct Fsm<C>(Arc<Mutex<DnsMachine>>, PhantomData<*const C>);
pub struct Resolver(Arc<Mutex<DnsMachine>>);

pub fn create_resolver<C>(scope: &mut EarlyScope, config: Config)
    -> Result<(Fsm<C>, Resolver), io::Error>
{
    let machine = DnsMachine {
        config: config,
        running: HashMap::new(),
        // TODO(tailhook) implement duplicate checking
        // queued: HashMap::new(),
        cache: HashMap::new(),
        sock: try!(UdpSocket::bound(&SocketAddr::V4(
            SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0)))),
        timeouts: BinaryHeap::new(),
        timeout: None,
        notifier: scope.notifier(),
    };
    try!(scope.register(&machine.sock,
        EventSet::readable(), PollOpt::level()));
    let arc = Arc::new(Mutex::new(machine));
    Ok((Fsm(arc.clone(), PhantomData), Resolver(arc.clone())))
}
