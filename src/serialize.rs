use dns_parser::Builder;

use {Query};

pub fn serialize(id: u16, q: &Query) -> Result<Vec<u8>, Error> {
    use Query::*;
    match q {
        LookupHost(ref h) => {
            let mut buf = Builder::new_query(id, true)
            buf.add
        }
    }
}
