pub struct TrackerRequest<'a> {
    pub info_hash: &'a [u8; 20],
    pub peer_id: &'a [u8; 20],
    pub ip: Option<std::net::IpAddr>,
    pub port: u16,
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub event: Option<Event>,
    pub compact: u8,
}

pub enum Event {
    Started,
    Completed,
    Stopped,
}
