#[macro_use] extern crate rotor;
extern crate rotor_dns;
extern crate rotor_tools;
extern crate argparse;
extern crate time;

use std::process::exit;

use time::Duration;
use rotor::void::{Void, unreachable};
use argparse::{ArgumentParser, Store, List, ParseOption};
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
    type Seed = Void;
    type Context = Context;
    fn create(seed: Self::Seed, _scope: &mut Scope<Self::Context>)
        -> Response<Self, Void>
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
    let mut servers = vec![];
    let mut timeout = None;
    let mut attempts = None;
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("
            Host-like comand to resolve the host
            ");
        ap.refer(&mut servers).add_option(&["--servers"], List, "
            Override servers to use for name resolving (Note, it's useless to
            set number of servers more than attepmpts).")
            .metavar("HOST:PORT");
        ap.refer(&mut attempts).add_option(&["--attempts"], ParseOption, "
            Override number of attempts");
        ap.refer(&mut timeout).add_option(&["--timeout-ms"], ParseOption, "
            Override the network timeout. In milliseconds.");
        ap.refer(&mut host).add_argument("name", Store, "
            Hostname to resolve");
        ap.parse_args_or_exit();
    }
    let mut loop_creator = rotor::Loop::new(&rotor::Config::new()).unwrap();
    let mut cfg = rotor_dns::Config::system().unwrap();
    if servers.len() > 0 {
        cfg.nameservers = servers;
    }
    attempts.map(|x| cfg.attempts = x);
    timeout.map(|x| cfg.timeout = Duration::milliseconds(x));
    let resolver = loop_creator.add_and_fetch(Composed::Dns, |scope| {
        rotor_dns::create_resolver(scope, cfg)
    }).unwrap();
    let mut loop_inst = loop_creator.instantiate(Context);
    let mut query = None;
    loop_inst.add_machine_with(|scope| {
        query = Some(resolver.query::<Scope<Context>>(
            Query::LookupIpv4(host), scope).unwrap());
        Response::ok(Composed::Shut(Shutter))
    }).unwrap();
    loop_inst.run().unwrap();
    let qb = query.unwrap();
    let query = qb.lock().unwrap();
    let entry = query.as_ref().map(|x| &**x);
    if let Some(&CacheEntry { value: Answer::Ipv4(ref ips), .. }) = entry {
        for ip in ips {
            println!("{}", ip);
        }
        exit(0);
    } else {
        exit(1);
    }
}
