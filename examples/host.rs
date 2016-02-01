#[macro_use] extern crate rotor;
extern crate rotor_dns;
extern crate argparse;
extern crate void;

use std::error::Error;

use void::{Void, unreachable};
use argparse::{ArgumentParser, Store};
use rotor::{Machine, EventSet, Scope, Response};
use rotor_dns::{CacheEntry, Query, Answer};

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
    let mut host = "".to_string();
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("
            Host-like comand to resolve the host
            ");
        ap.refer(&mut host).add_argument("name", Store, "
            Hostname to resolve");
        ap.parse_args_or_exit();
    }
    let mut loop_creator = rotor::Loop::new(&rotor::Config::new()).unwrap();
    let mut resolver = None;
    loop_creator.add_machine_with(|scope| {
        let (res, fsm) = rotor_dns::create_resolver(
            scope, rotor_dns::Config::system().unwrap()).unwrap();
        resolver = Some(res);
        Ok(Composed::Dns(fsm))
    }).unwrap();
    let resolver = resolver.unwrap();
    let mut loop_inst = loop_creator.instantiate(Context);
    let mut query = None;
    loop_inst.add_machine_with(|scope| {
        query = Some(resolver.query::<Scope<Context>>(
            Query::LookupIpv4(host), scope).unwrap());
        Ok(Composed::Shut(Shutter))
    }).unwrap();
    loop_inst.run().unwrap();
    let qb = query.unwrap();
    let query = qb.lock().unwrap();
    let entry = query.as_ref().map(|x| &**x);
    if let Some(&CacheEntry { value: Answer::Ipv4(ref ips), .. }) = entry {
        for ip in ips {
            println!("{}", ip);
        }
    }
}
