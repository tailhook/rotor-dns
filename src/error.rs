quick_error! {
    #[derive(Debug)]
    pub enum Error {
        /// The query when serialized is just too long to fit 512 bytes
        ///
        /// I'm not sure how is this possible, unless you have supplied an
        /// invalid name (valid name has length < 253 bytes)
        QueryIsTooLong {
            description("Query is too long")
        }
    }
}
