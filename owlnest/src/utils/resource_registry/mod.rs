pub mod resources;

use std::collections::HashSet;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info};

use self::resources::Resource;

#[derive(Debug)]
pub enum RegistryMessage {
    Register(Resource, oneshot::Sender<Result<(), ()>>),
    Unregister(Resource, oneshot::Sender<Result<(), ()>>),
}
pub struct ResourceRegistry {
    inner: HashSet<Resource>,
    receiver: mpsc::Receiver<RegistryMessage>,
}
impl ResourceRegistry {
    fn new(receiver: mpsc::Receiver<RegistryMessage>) -> Self {
        Self {
            inner: std::collections::HashSet::new(),
            receiver,
        }
    }
    fn handle_message(&mut self, msg: RegistryMessage) {
        match msg {
            RegistryMessage::Register(resource, sender) => handle_register(self, resource, sender),
            RegistryMessage::Unregister(resource, sender) => handle_unregister(self, resource, sender)
        }
    }
}

async fn startup_resource_registry(mut registry: ResourceRegistry) {
    while let Some(msg) = registry.receiver.recv().await {
        registry.handle_message(msg);
    }
}

#[derive(Clone)]
pub struct RegistryHandle {
    sender: mpsc::Sender<RegistryMessage>,
}
impl RegistryHandle {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let registry = ResourceRegistry::new(receiver);
        tokio::spawn(startup_resource_registry(registry));
        Self { sender }
    }
    pub async fn register(&self, resource: Resource) -> Result<(), ()> {
        let (send, recv) = oneshot::channel();
        let msg = RegistryMessage::Register(resource, send);
        let _ = self.sender.send(msg).await;
        match recv.await {
            Ok(res) => res,
            Err(e) => {
                error!("Failed to receive respond from actor with {:#?}", e);
                Err(())
            }
        }
    }
    pub async fn unregister(&self, resource: Resource) -> Result<(), ()> {
        let (send, recv) = oneshot::channel();
        let msg = RegistryMessage::Unregister(resource, send);
        let _ = self.sender.send(msg).await;
        match recv.await {
            Ok(res) => res,
            Err(e) => {
                error!("Failed to receive respond from actor with {:#?}", e);
                Err(())
            }
        }
    }
}

fn handle_register(
    registry: &mut ResourceRegistry,
    resource: Resource,
    sender: oneshot::Sender<Result<(), ()>>,
) {
    if registry.inner.contains(&resource) {
        error!(
            "{} tried to register resource: {:#?},which has already been registered!",
            resource.owner, resource.resource
        );
    } else {
        info!(
            "{} is trying to register a new resource: {:#?}",
            resource.owner, resource.resource
        );
        match registry.inner.insert(resource.clone()) {
            true => {
                info!("Successfully registered resource: {:#?}", resource);
                let _ = sender.send(Ok(()));
                return;
            }
            false => {
                error!(
                    "Failed to register a resource that hasn't been registered before: {:#?}. 
                This is a bug and you should report.",
                    resource
                );
            }
        }
    }
    sender.send(Err(())).unwrap();
}
fn handle_unregister(
    registry: &mut ResourceRegistry,
    resource: Resource,
    sender: oneshot::Sender<Result<(), ()>>,
) {
    if registry.inner.contains(&resource) {
        info!(
            "{} is trying to unregister resource: {:#?}",
            resource.owner, resource.resource
        );
        match registry.inner.remove(&resource) {
            true => {
                info!("Successfully unregistered resource: {:#?}", resource);
                let _ = sender.send(Ok(()));
                return;
            }
            false => {
                error!(
                    "Failed to register a resource that hasn't been registered before: {:#?}. 
                This is a bug and you should report.",
                    resource
                );
            }
        }
    } else {
        error!(
            "{} tried to unregister resource: {:#?},which doesn't exist!",
            resource.owner, resource.resource
        );
    }
    sender.send(Err(())).unwrap();
}
