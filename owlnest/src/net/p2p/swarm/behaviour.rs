use owlnest_macro::behaviour_select;
use owlnest_proc::{generate_behaviour_select, with_field};


#[cfg_attr(any(feature = "owlnest-protocols", feature = "owlnest-messaging"), with_field({pub messaging:Messaging}))]
#[cfg_attr(any(feature = "owlnest-protocols", feature = "owlnest-advertise"), with_field({pub advertise:Advertise}))]
#[cfg_attr(any(feature = "owlnest-protocols", feature = "owlnest-blob"), with_field({pub blob:Blob}))]
#[cfg_attr(any(feature = "libp2p-protocols", feature = "libp2p-mdns"), with_field({pub mdns:Mdns}))]
#[cfg_attr(any(feature = "libp2p-protocols", feature = "libp2p-autonat"), with_field({pub autonat:AutoNat}))]
#[cfg_attr(any(feature = "libp2p-protocols", feature = "libp2p-identify"), with_field({pub identify:Identify}))]
#[cfg_attr(any(feature = "libp2p-protocols", feature = "libp2p-dcutr"), with_field({pub dcutr:Dcutr}))]
#[cfg_attr(any(feature = "libp2p-protocols", feature = "libp2p-upnp"), with_field({pub upnp:Upnp}))]
#[cfg_attr(any(feature = "libp2p-protocols", feature = "libp2p-ping"), with_field({pub ping:Ping}))]
#[cfg_attr(any(feature = "libp2p-protocols", feature = "libp2p-relay-server"), with_field({pub relay_server:RelayServer}))]
#[cfg_attr(any(feature = "libp2p-protocols", feature = "libp2p-kad"), with_field({pub kad:Kad}))]
#[cfg_attr(any(feature = "libp2p-protocols", feature = "libp2p-relay-client"), with_field({pub relay_client:RelayClient}))]
#[cfg_attr(any(feature = "libp2p-protocols", feature = "libp2p-gossipsub"), with_field({pub gossipsub:Gossipsub}))]
#[generate_behaviour_select]
pub struct Behaviour {}
