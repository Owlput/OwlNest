use crate::event_bus::ListenedEvent;
use owlnest_macro::behaviour_select;

behaviour_select! {
    messaging: Messaging,
    tethering: Tethering,
    relay_server: RelayServer,
    relay_client: RelayClient,
    relay_ext:RelayExt,
    kad: Kad,
    identify: Identify,
    mdns: Mdns,
    dcutr: Dcutr,
}

impl Into<ListenedEvent> for ToSwarmEvent {
    fn into(self) -> ListenedEvent {
        ListenedEvent::new(format!("swarmEvent:{:?}", self), self)
    }
}
