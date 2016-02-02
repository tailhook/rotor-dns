=========
Rotor DNS
=========

A pure-rust asynchronous domain name system resolver library.

:Status: Pre-alpha
:Documentation: http://tailhook.github.com/rotor-dns/

The library based on `resolv-conf`_ and `dns-parser`_ and mostly provides only
asynchronous layer on top of `rotor`_.

The plan, is to implement good absractions:

* Resolve SRV and fall back to regular host name
* Subscribe to the domain name, not just resolve

The subscription should work as follows:

1. Resolve name by normal means
2. Sleep almost a TTL time (get some time to resolve)
3. Re-resolve name
4. Check if current connection is connected to one of the names resolved
5. Reconnect if needed
6. If new connection is successful drop the old one

The steps 4-6 above are obviously a protocol handler's job. But we should
provide good abstractions to do that.

.. _resolv-conf: http://github.com/tailhook/resolv-conf
.. _dns-parser: http://github.com/tailhook/dns-parser
.. _rotor: http://github.com/tailhook/rotor

=======
License
=======

Licensed under either of

* Apache License, Version 2.0,
  (./LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license (./LICENSE-MIT or http://opensource.org/licenses/MIT)
  at your option.

------------
Contribution
------------

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
