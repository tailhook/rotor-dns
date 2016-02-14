#[macro_use] extern crate rotor;
extern crate rotor_dns;
extern crate rotor_tools;
extern crate void;
extern crate time;

use std::error::Error;
use std::process::exit;

use void::{Void, unreachable};
use rotor::{Machine, EventSet, Scope, Response};
use rotor_dns::{CacheEntry, Query, Answer};
use rotor_tools::loop_ext::{LoopExt};

struct Shutter;
struct Context;

rotor_compose!(enum Composed/CSeed <Context> {
    Shut(Shutter),
    Dns(rotor_dns::Fsm<Context>),
});

impl Machine for Shutter {
    type Seed = Void; // Actually void
    type Context = Context;
    fn create(seed: Self::Seed, _scope: &mut Scope<Self::Context>)
        -> Result<Self, Box<Error>>
    { unreachable(seed); }
    fn ready(self, _events: EventSet, _scope: &mut Scope<Self::Context>)
        -> Response<Self, Self::Seed>
    { unreachable!(); }
    fn spawned(self, _scope: &mut Scope<Self::Context>)
        -> Response<Self, Self::Seed>
    { unreachable!(); }
    fn timeout(self, _scope: &mut Scope<Self::Context>)
        -> Response<Self, Self::Seed>
    { unreachable!(); }
    fn wakeup(self, scope: &mut Scope<Self::Context>)
        -> Response<Self, Self::Seed>
    {
        scope.shutdown_loop();
        Response::ok(self)
    }
}


fn main() {
    let host = "gmail.com".to_string();
    let mut loop_creator = rotor::Loop::new(&rotor::Config::new()).unwrap();
    let cfg = rotor_dns::Config::system().unwrap();
    let resolver = loop_creator.add_and_fetch(Composed::Dns, |scope| {
        rotor_dns::create_resolver(scope, cfg)
    }).unwrap();
    let mut loop_inst = loop_creator.instantiate(Context);
    let mut query = None;
    loop_inst.add_machine_with(|scope| {
        query = Some(resolver.query::<Scope<Context>>(
            Query::LookupMx(host), scope).unwrap());
        Ok(Composed::Shut(Shutter))
    }).unwrap();
    loop_inst.run().unwrap();
    let qb = query.unwrap();
    let query = qb.lock().unwrap();
    let entry = query.as_ref().map(|x| &**x);
    if let Some(&CacheEntry { value: Answer::Mx(ref recs), .. }) = entry {
        for record in recs {
            println!("{:5} {}", record.preference, record.exchange);
        }
        exit(0);
    } else {
        exit(1);
    }
}
