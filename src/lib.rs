extern crate rotor;
extern crate time;
extern crate dns_parser;

use std::collections::HashMap;
use std::sync::Arc;

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
    id:
    timeout: rotor::Timeout,
}

pub struct CacheEntry {
    pub value: Response,
    pub expire: time::SteadyTime,
}

struct DnsMachine {
    running: HashMap<Id, Request>,
    queued: HashMap<Query, Id>,
    cache: HashMap<Query, Arc<CacheEntry>>,
}
