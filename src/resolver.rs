use std::io;
use std::sync::{Arc, Mutex};

use time::{SteadyTime};
use rotor::GenericScope;

use {Query, Resolver, CacheEntry, Request, TimeEntry};

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
                // TODO(tailhook) should we trade off possible bugs for
                //                performance?
                scope.notifier().wakeup().unwrap();
                return Ok(Arc::new(Mutex::new(Some(cache.clone()))));
            }
        }
        // TODO(tailhook) implement round-robin/random server selection
        let server = 0;
        let id = try!(res.send_request(&query, server));

        let result = Arc::new(Mutex::new(None));
        let deadline = SteadyTime::now() + res.config.timeout;
        res.running.insert(id, Request {
            id: id,
            query: query,
            nameserver_index: server,
            attempts: 1,
            server: res.config.nameservers[server],
            deadline: deadline,
            notifiers: vec![(result.clone(), scope.notifier())],
        });
        res.timeouts.push(TimeEntry(deadline, id));
        res.notifier.wakeup().unwrap();  // to schedule a timeout
        Ok(result)
    }
}
