use std::collections::HashSet;

use tokio::sync::{mpsc, oneshot};
use tracing::{error, info};

#[derive(Debug)]
pub struct Resource {
    resource: String,
    owner: String,
    desp: String,
    detail: String,
}
impl std::hash::Hash for Resource {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.resource.hash(state)
    }
}
impl PartialEq for Resource {
    fn eq(&self, other: &Self) -> bool {
        self.resource == other.resource
    }
}
impl Eq for Resource {}

pub enum RegistryMessage {
    Register(Resource, oneshot::Sender<Result<(), ()>>),
    Unregister(Resource, oneshot::Sender<Result<(), ()>>),
}
struct ResourceRegistry {
    registry: HashSet<Resource>,
    receiver: mpsc::Receiver<RegistryMessage>,
}
impl ResourceRegistry {
    fn new(receiver: mpsc::Receiver<RegistryMessage>) -> Self {
        Self {
            registry: std::collections::HashSet::new(),
            receiver,
        }
    }
    fn handle_message(&mut self, msg: RegistryMessage) {
        match msg {
            RegistryMessage::Register(resource, sender) => {
                if self.registry.contains(&resource) {
                    error!(
                        "{} tried to register resource: {:#?},which has already been registered!",
                        resource.owner, resource.resource
                    );
                    let _ = sender.send(Err(()));
                } else {
                    info!(
                        "{} is trying to register a new resource: {:#?}",
                        resource.owner, resource.resource
                    );
                    let resource_self = resource.resource.clone();
                    match self.registry.insert(resource) {
                        true => {
                            info!("Successfully registered resource: {:#?}", resource_self);
                            let _ = sender.send(Ok(()));
                        }
                        false => {
                            error!("Failed to register a resource that hasn't been registered before: {:#?}. 
                            This is a bug and you should report.",resource_self);
                            let _ = sender.send(Err(()));
                        }
                    }
                }
            }
            RegistryMessage::Unregister(resource, sender) => {
                if self.registry.contains(&resource) {
                    info!(
                        "{} is trying to unregister resource: {:#?}",
                        resource.owner, resource.resource
                    );
                    let resource_self = resource.resource.clone();
                    match self.registry.remove(&resource) {
                        true => {
                            info!("Successfully unregistered resource: {:#?}", resource_self);
                            let _ = sender.send(Ok(()));
                        }
                        false => {
                            error!("Failed to register a resource that hasn't been registered before: {:#?}. 
                            This is a bug and you should report.",resource_self);
                            let _ = sender.send(Err(()));
                        }
                    }
                } else {
                    error!(
                        "{} tried to unregister resource: {:#?},which doesn't exist!",
                        resource.owner, resource.resource
                    );
                    let _ = sender.send(Err(()));
                }
            }
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

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_hash_comat() {
        let mut set = HashSet::new();
        let final_set = HashSet::from([
            Resource {
                resource: format!("127.0.0.1:100{}", 1),
                owner: format!("grpc{}", 1),
                desp: format!("socket used by grpc{}", 1),
                detail: "".into(),
            },
            Resource {
                resource: format!("127.0.0.1:100{}", 2),
                owner: format!("grpc{}", 2),
                desp: format!("socket used by grpc{}", 2),
                detail: "".into(),
            },
        ]);
        for i in 1..5 {
            let resource = Resource {
                resource: format!("127.0.0.1:100{}", i),
                owner: format!("grpc{}", i),
                desp: format!("socket used by grpc{}", i),
                detail: "".into(),
            };
            assert_eq!(set.insert(resource), true);
        }
        for i in 1..5 {
            let resource = Resource {
                resource: format!("127.0.0.1:100{}", i),
                owner: format!("grpc{}", i),
                desp: format!("socket used by grpc{}", i),
                detail: "".into(),
            };
            assert_eq!(set.insert(resource), false);
        }
        for i in 3..5 {
            let resource = Resource {
                resource: format!("127.0.0.1:100{}", i),
                owner: format!("grpc{}", i),
                desp: format!("socket used by grpc{}", i),
                detail: "".into(),
            };
            assert_eq!(set.contains(&resource), true);
            assert_eq!(set.remove(&resource), true);
            assert_eq!(set.contains(&resource), false);
        }
        assert_eq!(set, final_set);
    }
    #[tokio::test]
    async fn test_registery() {
        let registry_handle = RegistryHandle::new();
        for i in 1..5 {
            let registry_handle = registry_handle.clone();
            let resource = Resource {
                resource: format!("127.0.0.1:100{}", i),
                owner: format!("grpc{}", i),
                desp: format!("socket used by grpc{}", i),
                detail: "".into(),
            };
            assert_eq!(registry_handle.register(resource).await, Ok(()));
        }
        for i in 3..5 {
            let registry_handle = registry_handle.clone();
            let resource = Resource {
                resource: format!("127.0.0.1:100{}", i),
                owner: format!("grpc{}", i),
                desp: format!("socket used by grpc{}", i),
                detail: "".into(),
            };
            assert_eq!(registry_handle.unregister(resource).await, Ok(()))
        }
    }
}
