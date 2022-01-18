pub mod database;
pub mod filelog;
pub mod net;

pub enum DebugAdapter {
    Console,
    FileLog(filelog::FilelogAdapter),
    NetStream,
    Database(database::DatabaseAdapter),
}
impl DebugAdapter {
    pub fn send(&self, msg: &String) -> Result<(), String> {
        match self {
            DebugAdapter::Console => match println!("{}", msg) {
                _ => Ok(()),
            },
            DebugAdapter::FileLog(_adapter) => Err("Unimplemented".into()),
            DebugAdapter::NetStream => Err("Unimplemented".into()),
            DebugAdapter::Database(_adapter) => Err("Unimplemented".into()),
        }
    }
}
impl core::fmt::Display for DebugAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebugAdapter::Console => write!(f, "Console Logging"),
            DebugAdapter::FileLog(_) => write!(f, "File Logging"),
            DebugAdapter::NetStream => write!(f, "Network Debug Streaming"),
            DebugAdapter::Database(_) => write!(f, "Database Recording"),
        }
    }
}
