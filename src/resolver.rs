use std::io;
use std::sync::{Arc, Mutex};

use rand::{thread_rng, Rng};
use time::{SteadyTime, Duration};
use rotor::GenericScope;
use dns_parser::{Builder, QueryType, QueryClass};

use {Query, Resolver, CacheEntry, Request};


quick_error! {
    /// Error when creating a query
    ///
    /// Errors here are purely theoretical, should not happen in practice.
    /// It's okay to assert/unwrap on the errors, unless you pass
    /// user-generated data here.
    #[derive(Debug)]
    pub enum QueryError {
        /// Your query is too long so it doesn't fit in 512 bytes.
        /// Should not happen in practice
        TruncatedPacket {
            description("query results in packet truncation")
        }
        Net(err: io::Error) {
            description(err.description())
            display("{}", err)
            cause(err)
            from()
        }
    }
}



impl Resolver {
    pub fn query<S>(&self, query: Query, scope: &mut GenericScope)
        -> Result<Arc<Mutex<Option<Arc<CacheEntry>>>>, QueryError>
        where S: GenericScope
    {
        let ref mut res = *self.0.lock().unwrap();
        if let Some(cache) =  res.cache.get(&query).map(|x| x.clone()) {
            if SteadyTime::now() > cache.expire {
                res.cache.remove(&query);
            } else {
                // TODO(tailhook) wakeup now
                unimplemented!();
            }
        }
        // TODO(tailhook) limit number of retries somehow
        // Note: we don't use counter here, because this allows us to be
        // a little more resistant to DNS spoofing
        let mut id = thread_rng().gen();
        while res.running.contains_key(&id) {
            id = thread_rng().gen();
        }
        let mut builder = Builder::new_query(id, true);
        match query {
            Query::LookupIpv4(ref q) => {
                builder.add_question(q, QueryType::A, QueryClass::IN);
            }
        }
        let pack = try!(builder.build()
            .map_err(|_| QueryError::TruncatedPacket));

        // TODO(tailhook) better server selection algo
        let server = res.config.nameservers[0];

        try!(res.sock.send_to(&pack, &server));

        let result = Arc::new(Mutex::new(None));
        res.running.insert(id, Request {
            id: id,
            query: query,
            server: server,
            deadline: SteadyTime::now() + res.config.timeout,
            notifiers: vec![(result.clone(), scope.notifier())],
        });
        Ok(result)
    }
}
