#[derive(Debug, Clone, Copy)]
pub enum ActionState<Ready, Busy> {
    Ready(Ready),
    MaybeBusy(Busy),
}
