use std::hash::Hash;

pub mod abstract_resource;
pub mod net;

#[derive(Debug, Clone)]
pub struct Resource {
    pub resource: Resources,
    pub owner: String,
    pub desp: String,
    pub detail: String,
}

impl Hash for Resource {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.resource.hash(state);
    }
}
impl PartialEq for Resource {
    fn eq(&self, other: &Self) -> bool {
        self.resource == other.resource
    }
}
impl Eq for Resource {}

#[derive(Debug, PartialEq, Clone)]
pub enum Resources {
    NetConn(net::ConnType),
    Abstract(abstract_resource::AbstractResource),
}

impl Hash for Resources {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Resources::NetConn(conn) => conn.hash(state),
            Resources::Abstract(abstract_res) => abstract_res.hash(state),
        };
    }
}
