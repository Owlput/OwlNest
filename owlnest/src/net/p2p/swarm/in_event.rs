use super::*;

#[derive(Debug)]
pub struct InEvent {
    op: swarm_op::Op,
    callback: oneshot::Sender<swarm_op::OpResult>,
}
impl InEvent {
    pub fn new(op: swarm_op::Op, callback: oneshot::Sender<swarm_op::OpResult>) -> Self {
        InEvent { op, callback }
    }
    pub fn into_inner(self) -> (swarm_op::Op, oneshot::Sender<swarm_op::OpResult>) {
        (self.op, self.callback)
    }
}



#[derive(Debug)]
pub enum BehaviourOp {
    Messaging(messaging::Op),
    Tethering(tethering::Op),
    Kad(kad::Op)
}

#[derive(Debug)]
pub enum BehaviourOpResult {
    Messaging(messaging::OpResult),
    Tethering(Result<tethering::HandleOk,tethering::HandleError>),
    Kad(kad::OpResult)
}
impl TryInto<messaging::OpResult> for BehaviourOpResult{
    type Error = ();
    fn try_into(self) -> Result<messaging::OpResult, Self::Error> {
        match self{
            Self::Messaging(result)=>Ok(result),
            _=>Err(())
        }
    }
}
impl TryInto<Result<tethering::HandleOk,tethering::HandleError>> for BehaviourOpResult{
    type Error = ();
    fn try_into(self) -> Result<Result<tethering::HandleOk,tethering::HandleError>, Self::Error> {
        match self{
            Self::Tethering(result)=>Ok(result),
            _=>Err(())
        }
    }
}
impl TryInto<kad::OpResult> for BehaviourOpResult{
    type Error = ();
    fn try_into(self) -> Result<kad::OpResult, Self::Error> {
        match self{
            Self::Kad(result)=>Ok(result),
            _=>Err(())
        }
    }
}



#[derive(Debug)]
pub enum BehaviourInEvent {
    Messaging(messaging::InEvent),
    Tethering(tethering::InEvent),
    Kad(kad::InEvent)
}
