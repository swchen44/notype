#[path = "../../desktop/src-tauri/src/native_input.rs"]
mod native_input;
use notype_core::{DeliveryAdapter, DeliveryOutcome, DeliveryRequest};
#[test]
fn persisted_or_invented_target_token_never_becomes_current_foreground() {
    let input = native_input::NativeInput::default();
    let mut delivery = input.deliver(DeliveryRequest {
        session_id: "nonsecret-fixture".into(),
        text: "must not insert".into(),
        original_target: "invented-from-previous-launch".into(),
    });
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    assert_eq!(
        delivery.as_mut().poll(&mut context),
        std::task::Poll::Ready(DeliveryOutcome::Unavailable)
    );
}
