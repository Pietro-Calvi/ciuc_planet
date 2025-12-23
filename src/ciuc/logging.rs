use common_game::logging::Participant;
use crate::CiucAI;
use common_game::logging::{ActorType, Channel, EventType, LogEvent};
use std::collections::BTreeMap;

impl CiucAI {
    ///Function for logging
    pub fn log_event(
        sender: Option<Participant>,
        receiver: Option<Participant>,
        event_type: EventType,
        channel: Channel,
        payload: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) {
        let payload: BTreeMap<String, String> = payload
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();

        let event = LogEvent::new(sender, receiver, event_type, channel, payload);
        event.emit();
    }
}
