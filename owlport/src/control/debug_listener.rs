pub mod adapters;

pub struct DebugListener {
    adapters: Vec<adapters::DebugAdapter>,
}
impl DebugListener {
    pub fn new(adapters: Vec<adapters::DebugAdapter>) -> Self {
        DebugListener {
            adapters,
        }
    }
    pub fn push(self, msg: &'static str){
            for adapter in self.adapters{
                match adapter.send(msg){
                    Ok(_)=>{}
                    Err(_)=>{println!("Unimplemented:{}",adapter)}
                };
            }
    }
}
