use time::SteadyTime;
use rotor::GenericScope;

use {Query};

impl Resolver {
    fn query<S: GenericScope>(&self, query: Query, scope: GenericScope) {
        let ref mut res = *self.lock();
        match res.cache.get(&query).map(|x| x.clone()) {
            Some(x) => {
                if SteadyTime::now() > x.expire() {
                    res.cache.remove(&query);
                } else {
                    return x;
                }
            }
        }
        unimplemented!();
    }
}
