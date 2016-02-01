use std::i32;
use std::cmp::min;
use std::sync::Arc;
use std::io::ErrorKind::Interrupted;
use std::error::Error;

use time::{SteadyTime, Duration};
use dns_parser::{Packet, QueryType, QueryClass, RRData};
use rotor::{Machine, EventSet, Scope, Response};

use {Fsm, Request, Query, Answer, CacheEntry};

impl Request {
    pub fn matches(&self, pack: &Packet) -> bool {
        if pack.questions.len() != 1 {
            return false;
        }
        let ref q = pack.questions[0];
        match self.query {
            Query::LookupIpv4(ref host) => {
                if q.qtype != QueryType::A || q.qclass != QueryClass::IN {
                    return false;
                }
                // TODO(tailhook) optimize the comparison
                if &format!("{}", q.qname) != host {
                    return false;
                }
            }
        }
        return true;
    }
}


impl<C> Machine for Fsm<C> {
    type Seed = (); // Actually void
    type Context = C;
    fn create(seed: Self::Seed, scope: &mut Scope<Self::Context>)
        -> Result<Self, Box<Error>>
    { unreachable!(); }
    fn ready(self, events: EventSet, scope: &mut Scope<Self::Context>)
        -> Response<Self, Self::Seed>
    {
        {
            let mut res = self.0.lock().unwrap();
            let mut buf = vec![0u8; 4096];
            loop {
                let (bytes, addr) = match res.sock.recv_from(&mut buf) {
                    Ok(Some((bytes, addr))) => (bytes, addr),
                    Ok(None) => break,
                    Err(ref ioerr) if ioerr.kind() == Interrupted
                    => continue,
                    Err(_) => {
                        // Nothing we can do. Should we log it?
                        // TODO(tailhook) Should we continue by default?
                        break;
                    }
                };
                assert!(bytes < buf.len()); // TODO(tailhook) truncated
                let pack = match Packet::parse(&buf[..bytes]) {
                    Ok(pack) => pack,
                    Err(_) => {
                        // Just a bad packet. Should we log it?
                        continue;
                    }
                };
                let req = match res.running.remove(&pack.header.id) {
                    Some(request) => request,
                    None => {
                        // Unsolicited reply. Should we log it?
                        continue;
                    }
                };
                if req.server != addr || !req.matches(&pack) {
                    // Probably someone tries to spoof us. Log it?
                    res.running.insert(req.id, req);
                    continue;
                }
                let mut ttl = i32::MAX as u32;
                let result = match req.query {
                    Query::LookupIpv4(_) => {
                        let mut ips = Vec::with_capacity(pack.answers.len());
                        for ans in pack.answers {
                            ttl = min(ttl, ans.ttl);
                            match ans.data {
                                RRData::A(ip) => {
                                    ips.push(ip);
                                }
                                _ => {
                                    // Bad value. Log it?
                                }
                            }
                        }
                        Answer::Ipv4(ips)
                    }
                };
                let entry = CacheEntry {
                    value: result,
                    expire: SteadyTime::now() + Duration::seconds(ttl.into()),
                };
                let cache = Arc::new(entry);
                for (result, notifier) in req.notifiers {
                    *result.lock().unwrap() = Some(cache.clone());
                    notifier.wakeup().unwrap();
                }
                res.cache.insert(req.query, cache);
            }
        }
        Response::ok(self)
    }
    fn spawned(self, scope: &mut Scope<Self::Context>)
        -> Response<Self, Self::Seed>
    { unreachable!(); }
    fn timeout(self, scope: &mut Scope<Self::Context>)
        -> Response<Self, Self::Seed>
    { unreachable!(); }
    fn wakeup(self, scope: &mut Scope<Self::Context>)
        -> Response<Self, Self::Seed>
    { unimplemented!(); }
}
