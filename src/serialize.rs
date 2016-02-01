use dns_parser::{Builder, QueryType, QueryClass};

use {Query, Error};

pub fn serialize(id: u16, q: &Query) -> Result<Vec<u8>, Error> {
    use Query::*;
    match *q {
        Query::LookupIpv4(ref h) => {
            let mut buf = Builder::new_query(id, true);
            buf.add_question(h, QueryType::A, QueryClass::IN);
            buf.build().map_err(|_| Error::QueryIsTooLong)
        }
    }
}
