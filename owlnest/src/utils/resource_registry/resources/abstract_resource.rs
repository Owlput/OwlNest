use super::net::ConnType;

#[derive(Debug, Hash, PartialEq, Clone)]
pub enum AbstractResource {
    ConnectedPort { type_of: PortType, remote: ConnType },
}

#[derive(Debug, Hash, PartialEq, Clone)]
pub enum PortType {
    Desktop,
    Mobile,
    Webpage,
    Unknown,
}
