use std::i32;
use std::cmp::{min};
use std::sync::Arc;
use std::io::ErrorKind::Interrupted;

use rand::{thread_rng, Rng};
use rotor::void::{unreachable, Void};
use time::{SteadyTime, Duration};
use dns_parser::{Packet, QueryType, QueryClass, RRData, Builder};
use rotor::{Machine, EventSet, Scope, Response};

use {Fsm, Request, Query, Answer, CacheEntry, DnsMachine, Id, QueryError};
use {TimeEntry, MxRecord, SrvRecord};

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
            Query::LookupSrv(ref host) => {
                if q.qtype != QueryType::SRV || q.qclass != QueryClass::IN {
                    return false;
                }
                // TODO(tailhook) optimize the comparison
                if &format!("{}", q.qname) != host {
                    return false;
                }
            }
            Query::LookupMx(ref host) => {
                if q.qtype != QueryType::MX || q.qclass != QueryClass::IN {
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

impl DnsMachine {
    fn refresh_timeouts<C>(&mut self, scope: &mut Scope<C>) {
        self.timeout.take().map(|x| scope.clear_timeout(x));

        let now = SteadyTime::now();
        while self.timeouts.peek().map(|x| x.0 < now).unwrap_or(false) {
            let id = self.timeouts.pop().unwrap().1;
            if let Some(mut req) = self.running.remove(&id) {
                if req.deadline >= now {
                    continue;
                }
                if req.attempts >= self.config.attempts {
                    let bad_cache = Arc::new(CacheEntry {
                        value: Answer::ServerUnavailable,
                        expire: SteadyTime::now(),
                    });
                    for (arc, notifier) in req.notifiers.into_iter() {
                        arc.lock().as_mut()
                            .map(|x| **x = Some(bad_cache.clone()))
                            .ok();
                        notifier.wakeup().unwrap();
                    }
                } else {
                    req.attempts += 1;
                    req.nameserver_index = (req.nameserver_index + 1)
                                           % self.config.nameservers.len();
                    req.server = self.config.nameservers[req.nameserver_index];
                    req.deadline = SteadyTime::now() + self.config.timeout;

                    // There are two kind of errors:
                    // 1. Truncated packet, should never happen because we
                    //    generate exactly same packet as first time
                    // 2. Can't send message. Usually not happen second time
                    //    too, but in case it is, we treat it as a packet
                    //    loss (i.e. retry after a timeout)
                    self.send_request(&req.query, req.nameserver_index)
                        .map(|x| { req.id = x; }).ok(); // TODO(tailhook) log?
                    // TODO(tailhook) is it okay to put back with same id ?
                    self.timeouts.push(TimeEntry(req.deadline, req.id));
                    self.running.insert(req.id, req);
                }
            }
        }

        self.timeout = self.timeouts.peek().map(
            |x| scope.timeout_ms(x.millis()).unwrap());
    }
    fn recv_messages(&mut self) {
        loop {
            let mut buf = [0u8; 4096];
            let (bytes, addr) = match self.sock.recv_from(&mut buf) {
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
            let req = match self.running.remove(&pack.header.id) {
                Some(request) => request,
                None => {
                    // Unsolicited reply. Should we log it?
                    continue;
                }
            };
            if req.server != addr || !req.matches(&pack) {
                // Probably someone tries to spoof us. Log it?
                self.running.insert(req.id, req);
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
                Query::LookupMx(_) => {
                    let mut rows = Vec::with_capacity(pack.answers.len());
                    for ans in pack.answers {
                        ttl = min(ttl, ans.ttl);
                        match ans.data {
                            RRData::MX { preference, exchange } => {
                                rows.push(MxRecord {
                                    preference: preference,
                                    exchange: exchange.to_string(),
                                });
                            }
                            _ => {
                                // Bad value. Log it?
                            }
                        }
                    }
                    Answer::Mx(rows)
                }
                Query::LookupSrv(_) => {
                    let mut rows = Vec::with_capacity(pack.answers.len());
                    for ans in pack.answers {
                        ttl = min(ttl, ans.ttl);
                        match ans.data {
                            RRData::SRV { priority, weight, port, target } => {
                                rows.push(SrvRecord {
                                    priority: priority,
                                    weight: weight,
                                    port: port,
                                    target: target.to_string(),
                                });
                            }
                            _ => {
                                // Bad value. Log it?
                            }
                        }
                    }
                    Answer::Srv(rows)
                }
            };
            let entry = CacheEntry {
                value: result,
                expire: SteadyTime::now() + Duration::seconds(ttl.into()),
            };
            let cache = Arc::new(entry);
            for (result, notifier) in req.notifiers {
                result.lock().as_mut()
                    .map(|x| **x = Some(cache.clone())).ok();
                notifier.wakeup().unwrap();
            }
            self.cache.insert(req.query, cache);
        }
    }
    pub fn send_request(&mut self, query: &Query, idx: usize)
        -> Result<Id, QueryError>
    {
        // TODO(tailhook) limit number of retries somehow
        // Note: we don't use counter here, because this allows us to be
        // a little more resistant to DNS spoofing
        let mut id = thread_rng().gen();
        while self.running.contains_key(&id) {
            id = thread_rng().gen();
        }
        let mut builder = Builder::new_query(id, true);
        match query {
            &Query::LookupIpv4(ref q) => {
                builder.add_question(q, QueryType::A, QueryClass::IN);
            }
            &Query::LookupMx(ref q) => {
                builder.add_question(q, QueryType::MX, QueryClass::IN);
            }
            &Query::LookupSrv(ref q) => {
                builder.add_question(q, QueryType::SRV, QueryClass::IN);
            }
        }
        let pack = try!(builder.build()
            .map_err(|_| QueryError::TruncatedPacket));

        // TODO(tailhook) better server selection algo
        let server = self.config.nameservers[idx];

        try!(self.sock.send_to(&pack, &server));
        Ok(id)
    }
}

impl<C> Machine for Fsm<C> {
    type Seed = Void; // Actually void
    type Context = C;
    fn create(seed: Self::Seed, _scope: &mut Scope<Self::Context>)
        -> Response<Self, Void>
    { unreachable(seed); }
    fn ready(self, _events: EventSet, scope: &mut Scope<Self::Context>)
        -> Response<Self, Self::Seed>
    {
        {
            let mut res = self.0.lock().unwrap();
            res.recv_messages();
            res.refresh_timeouts(scope);
        }
        Response::ok(self)
    }
    fn spawned(self, _scope: &mut Scope<Self::Context>)
        -> Response<Self, Self::Seed>
    { unreachable!(); }
    fn timeout(self, scope: &mut Scope<Self::Context>)
        -> Response<Self, Self::Seed>
    {
        self.0.lock().unwrap().refresh_timeouts(scope);
        Response::ok(self)
    }
    fn wakeup(self, scope: &mut Scope<Self::Context>)
        -> Response<Self, Self::Seed>
    {
        self.0.lock().unwrap().refresh_timeouts(scope);
        Response::ok(self)
    }
}
