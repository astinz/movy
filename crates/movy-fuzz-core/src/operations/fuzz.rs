use libafl::{feedbacks::ExitKindLogic, observers::StdMapObserver};

pub const CODE_OBSERVER_NAME: &str = "code_observer";

pub struct OkFeedback;

impl ExitKindLogic for OkFeedback {
    const NAME: std::borrow::Cow<'static, str> = std::borrow::Cow::Borrowed("OkFeedback");
    fn check_exit_kind(kind: &libafl::executors::ExitKind) -> Result<bool, libafl::Error> {
        Ok(matches!(kind, libafl::executors::ExitKind::Ok))
    }
}

pub fn code_observer() -> StdMapObserver<'static, u8, false> {
    StdMapObserver::owned(CODE_OBSERVER_NAME, vec![0u8; 16384])
}
