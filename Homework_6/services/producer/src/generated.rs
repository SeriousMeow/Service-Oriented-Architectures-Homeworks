pub mod warehouse {
    pub mod events {
        include!(concat!(env!("OUT_DIR"), "/avrogen/warehouse/events.rs"));
    }
}
