#[macro_use] extern crate rotor;
extern crate rotor_dns;
extern crate argparse;

use std::error::Error;

use argparse::{ArgumentParser, Store};

use rotor::{Machine, EventSet, Scope, Response};

struct Shutter;
struct Context;

rotor_compose!(enum Composed/CSeed <Context> {
    Shut(Shutter),
    Dns(rotor_dns::Fsm<Context>),
});

impl Machine for Shutter {
    type Seed = (); // Actually void
    type Context = Context;
    fn create(seed: Self::Seed, scope: &mut Scope<Self::Context>)
        -> Result<Self, Box<Error>>
    { unreachable!(); }
    fn ready(self, events: EventSet, scope: &mut Scope<Self::Context>)
        -> Response<Self, Self::Seed>
    { unreachable!(); }
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
    loop_inst.add_machine_with(|scope| {
        resolver.query::<Scope<Context>>(rotor_dns::Query::LookupHost(host), scope);
        Ok(Composed::Shut(Shutter))
    }).unwrap();
    loop_inst.run().unwrap();
}
