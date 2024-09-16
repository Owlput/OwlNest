use super::super::protocols::*;
use owlnest_proc::{generate_manager, with_field};

#[cfg_attr(any(feature = "owlnest-protocols", feature = "owlnest-messaging"), with_field({pub messaging:Messaging}))]
#[cfg_attr(any(feature = "owlnest-protocols", feature = "owlnest-advertise"), with_field({pub advertise:Advertise}))]
#[cfg_attr(any(feature = "owlnest-protocols", feature = "owlnest-blob"), with_field({pub blob:Blob}))]
#[cfg_attr(any(feature = "libp2p-protocols", feature = "libp2p-kad"), with_field({pub kad:Kad}))]
#[cfg_attr(any(feature = "libp2p-protocols", feature = "libp2p-autonat"), with_field({pub autonat:AutoNat}))]
#[cfg_attr(any(feature = "libp2p-protocols", feature = "libp2p-mdns"), with_field({pub mdns:Mdns}))]
#[cfg_attr(any(feature = "libp2p-protocols", feature = "libp2p-gossipsub"), with_field({pub gossipsub:Gossipsub}))]
#[generate_manager]
pub(crate) struct RxBundle {}
