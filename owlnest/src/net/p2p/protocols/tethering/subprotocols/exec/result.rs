/// Operation result for sending to the peer who sends the request.
/// Won't be exposed via `tethering::OutEvent` but be sent through sender provided by the request caller after unwrapping.
#[derive(Debug,Serialize,Deserialize)]
pub enum Result{
    Messaging(messaging::OpResult,u128),
    Tethering()
}