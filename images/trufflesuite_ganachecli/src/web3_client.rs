use std::ops::Deref;
use web3::{
    transports::{EventLoopHandle, Http, Result},
    Web3,
};

pub struct Web3Client {
    _event_loop: EventLoopHandle,
    web3: Web3<Http>,
}

impl Web3Client {
    pub fn new(endpoint: &str) -> Result<Self> {
        let (_event_loop, transport) = Http::new(endpoint)?;
        let web3 = Web3::new(transport);

        Ok(Web3Client { _event_loop, web3 })
    }
}

impl Deref for Web3Client {
    type Target = Web3<Http>;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.web3
    }
}
