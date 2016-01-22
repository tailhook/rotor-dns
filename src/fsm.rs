use std::error::Error;

use rotor::{Machine, EventSet, Scope, Response};

use {Fsm};


impl<C> Machine for Fsm<C> {
    type Seed = (); // Actually void
    type Context = C;
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
