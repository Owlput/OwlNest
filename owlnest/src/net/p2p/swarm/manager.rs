use owlnest_proc::{generate_manager, with_field};
use super::super::protocols::*;

#[cfg_attr(feature = "owlnest-protocols",with_field({messaging:Messaging}))]
#[cfg_attr(feature = "owlnest-protocols",with_field({advertise:Advertise}))]
#[cfg_attr(feature = "owlnest-protocols",with_field({blob:Blob}))]
#[cfg_attr(feature = "libp2p-protocols",with_field({pub kad:Kad}))]
#[cfg_attr(feature = "libp2p-protocols",with_field({pub mdns:Mdns}))]
#[cfg_attr(feature = "libp2p-protocols",with_field({pub autonat:AutoNat}))]
#[generate_manager]
pub(crate) struct RxBundle {}
