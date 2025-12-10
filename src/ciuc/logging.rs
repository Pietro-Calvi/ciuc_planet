use std::collections::BTreeMap;
use common_game::logging::{ActorType, Channel, EventType, LogEvent};
use crate::CiucAI;

impl CiucAI{
    ///Function for logging
    pub(crate) fn log(&self, msg:String, id:u32, actor_type:ActorType, event_type: EventType, receiver:String, channel:Channel) {
        let mut p: BTreeMap<String, String> = BTreeMap::new();
        p.insert("msg".to_string(), msg);
        let start_event = LogEvent::new(ActorType::Planet, id, actor_type, receiver, event_type, channel, p);
        println!("{}", start_event);
    }
}