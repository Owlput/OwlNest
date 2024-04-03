use owlnest_macro::behaviour_select;
use owlnest_proc::{generate_behaviour_select, with_field};

#[cfg_attr(feature = "owlnest-protocols",with_field({pub messaging:Messaging}))]
#[cfg_attr(feature = "owlnest-protocols",with_field({pub advertise:Advertise}))]
#[cfg_attr(feature = "owlnest-protocols",with_field({pub blob:Blob}))]
#[cfg_attr(feature = "libp2p-protocols",with_field({pub relay_server:RelayServer}))]
#[cfg_attr(feature = "libp2p-protocols",with_field({pub kad:Kad}))]
#[cfg_attr(feature = "libp2p-protocols",with_field({pub relay_client:RelayClient}))]
#[cfg_attr(feature = "libp2p-protocols",with_field({pub mdns:Mdns}))]
#[cfg_attr(feature = "libp2p-protocols",with_field({pub autonat:AutoNat}))]
#[cfg_attr(feature = "libp2p-protocols",with_field({pub identify:Identify}))]
#[cfg_attr(feature = "libp2p-protocols",with_field({pub dcutr:Dcutr}))]
#[cfg_attr(feature = "libp2p-protocols",with_field({pub upnp:Upnp}))]
#[cfg_attr(feature = "libp2p-protocols",with_field({pub ping:Ping}))]
#[generate_behaviour_select]
pub struct Behaviour {}
