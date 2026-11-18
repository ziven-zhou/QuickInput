use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ConfirmAction {
    #[default]
    Paste,
    PasteAndEnter,
}

impl ConfirmAction {
    pub(crate) fn validate(self, enabled: bool) -> Result<(), String> {
        if self == Self::PasteAndEnter && !enabled {
            return Err("请先在设置中开启粘贴并发送快捷键".into());
        }
        Ok(())
    }
}

// Shared by both paste entry points so a second attempt cannot replace the clipboard
// while the first target is still consuming it. Drops also release synchronous failures.
pub(crate) struct ConfirmationLease(Arc<AtomicBool>);

impl ConfirmationLease {
    pub(crate) fn acquire(busy: Arc<AtomicBool>) -> Result<Self, String> {
        busy.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "上一次粘贴尚未结束，请稍后再试".to_string())?;
        Ok(Self(busy))
    }
}

impl Drop for ConfirmationLease {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[derive(Debug)]
pub(crate) struct ConfirmationFailure {
    // True means paste events were posted, not that the target consumed the text.
    pub(crate) pasted: bool,
    pub(crate) message: String,
}

pub(crate) trait InputAutomation {
    fn frontmost_pid(&self) -> i32;
    fn keys_released(&self) -> bool;
    fn paste(&mut self) -> Result<(), String>;
    fn enter(&mut self) -> Result<(), String>;
    fn wait(&mut self, millis: u64);
}

fn wait_for_keys(
    input: &mut impl InputAutomation,
    pasted: bool,
) -> Result<(), ConfirmationFailure> {
    for _ in 0..100 {
        if input.keys_released() {
            return Ok(());
        }
        input.wait(20);
    }
    Err(ConfirmationFailure {
        pasted,
        message: "快捷键尚未松开，已停止模拟按键".into(),
    })
}

pub(crate) fn perform_confirmation(
    action: ConfirmAction,
    target_pid: i32,
    input: &mut impl InputAutomation,
) -> Result<(), ConfirmationFailure> {
    if target_pid <= 0 {
        return Err(ConfirmationFailure {
            pasted: false,
            message: "未找到目标应用".into(),
        });
    }
    wait_for_keys(input, false)?;
    if input.frontmost_pid() != target_pid {
        return Err(ConfirmationFailure {
            pasted: false,
            message: "目标应用焦点已变化，已取消粘贴".into(),
        });
    }
    input.paste().map_err(|message| ConfirmationFailure {
        pasted: false,
        message,
    })?;
    // Preserve the existing 80+400 ms clipboard consumption window before Enter.
    input.wait(480);
    if action == ConfirmAction::PasteAndEnter {
        wait_for_keys(input, true)?;
        if input.frontmost_pid() != target_pid {
            return Err(ConfirmationFailure {
                pasted: true,
                message: "目标应用焦点已变化，已跳过回车".into(),
            });
        }
        input.enter().map_err(|message| ConfirmationFailure {
            pasted: true,
            message,
        })?;
        // Let the Return events settle before a keep-open window can take focus.
        input.wait(80);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    #[derive(Default)]
    struct FakeInput {
        pids: RefCell<VecDeque<i32>>,
        calls: Vec<&'static str>,
        paste_error: bool,
        enter_error: bool,
        held: bool,
        released: RefCell<VecDeque<bool>>,
    }
    impl InputAutomation for FakeInput {
        fn frontmost_pid(&self) -> i32 {
            self.pids.borrow_mut().pop_front().unwrap_or(42)
        }
        fn keys_released(&self) -> bool {
            self.released.borrow_mut().pop_front().unwrap_or(!self.held)
        }
        fn paste(&mut self) -> Result<(), String> {
            self.calls.push("paste");
            if self.paste_error {
                Err("paste failed".into())
            } else {
                Ok(())
            }
        }
        fn enter(&mut self) -> Result<(), String> {
            self.calls.push("enter");
            if self.enter_error {
                Err("enter failed".into())
            } else {
                Ok(())
            }
        }
        fn wait(&mut self, _: u64) {
            self.calls.push("wait");
        }
    }
    #[test]
    fn default_action_and_disabled_setting_are_safe() {
        assert_eq!(ConfirmAction::default(), ConfirmAction::Paste);
        assert!(ConfirmAction::Paste.validate(false).is_ok());
        assert!(ConfirmAction::PasteAndEnter.validate(false).is_err());
        assert!(ConfirmAction::PasteAndEnter.validate(true).is_ok());
        assert!(serde_json::from_str::<ConfirmAction>("\"unknown\"").is_err());
    }
    #[test]
    fn plain_confirmation_never_enters() {
        let mut input = FakeInput::default();
        assert!(perform_confirmation(ConfirmAction::Paste, 42, &mut input).is_ok());
        assert_eq!(input.calls, ["paste", "wait"]);
    }
    #[test]
    fn sending_waits_after_paste_and_enters_once() {
        let mut input = FakeInput::default();
        assert!(perform_confirmation(ConfirmAction::PasteAndEnter, 42, &mut input).is_ok());
        assert_eq!(input.calls, ["paste", "wait", "enter", "wait"]);
    }
    #[test]
    fn invalid_or_changed_target_never_receives_input() {
        for target in [0, -1, 99] {
            let mut input = FakeInput::default();
            let error =
                perform_confirmation(ConfirmAction::PasteAndEnter, target, &mut input).unwrap_err();
            assert!(!error.pasted);
            assert!(input.calls.is_empty());
        }
    }
    #[test]
    fn focus_change_after_paste_skips_enter_and_is_not_retryable() {
        let mut input = FakeInput {
            pids: RefCell::new(VecDeque::from([42, 99])),
            ..Default::default()
        };
        let error = perform_confirmation(ConfirmAction::PasteAndEnter, 42, &mut input).unwrap_err();
        assert!(error.pasted);
        assert_eq!(input.calls, ["paste", "wait"]);
    }
    #[test]
    fn paste_failure_never_sends() {
        let mut input = FakeInput {
            paste_error: true,
            ..Default::default()
        };
        let error = perform_confirmation(ConfirmAction::PasteAndEnter, 42, &mut input).unwrap_err();
        assert!(!error.pasted);
        assert_eq!(input.calls, ["paste"]);
    }
    #[test]
    fn enter_failure_does_not_retry_paste() {
        let mut input = FakeInput {
            enter_error: true,
            ..Default::default()
        };
        let error = perform_confirmation(ConfirmAction::PasteAndEnter, 42, &mut input).unwrap_err();
        assert!(error.pasted);
        assert_eq!(input.calls, ["paste", "wait", "enter"]);
    }
    #[test]
    fn held_confirmation_keys_timeout_without_pasting() {
        let mut input = FakeInput {
            held: true,
            ..Default::default()
        };
        let error = perform_confirmation(ConfirmAction::PasteAndEnter, 42, &mut input).unwrap_err();
        assert!(!error.pasted);
        assert!(!input.calls.contains(&"paste"));
        assert!(!input.calls.contains(&"enter"));
    }
    #[test]
    fn released_keys_resume_without_duplicating_paste_or_enter() {
        let mut input = FakeInput {
            released: RefCell::new(VecDeque::from([false, true, false, true])),
            ..Default::default()
        };
        assert!(perform_confirmation(ConfirmAction::PasteAndEnter, 42, &mut input).is_ok());
        assert_eq!(
            input.calls,
            ["wait", "paste", "wait", "wait", "enter", "wait"]
        );
    }

    #[test]
    fn keys_held_after_paste_skip_enter_without_repeating_paste() {
        let mut input = FakeInput {
            released: RefCell::new(VecDeque::from([true])),
            held: true,
            ..Default::default()
        };
        let error = perform_confirmation(ConfirmAction::PasteAndEnter, 42, &mut input).unwrap_err();
        assert!(error.pasted);
        assert_eq!(
            input.calls.iter().filter(|call| **call == "paste").count(),
            1
        );
        assert!(!input.calls.contains(&"enter"));
    }

    #[test]
    fn focus_is_rechecked_after_waiting_for_key_release() {
        let mut input = FakeInput {
            pids: RefCell::new(VecDeque::from([99])),
            released: RefCell::new(VecDeque::from([false, true])),
            ..Default::default()
        };
        let error = perform_confirmation(ConfirmAction::PasteAndEnter, 42, &mut input).unwrap_err();
        assert!(!error.pasted);
        assert_eq!(input.calls, ["wait"]);
    }

    #[test]
    fn confirmation_lease_excludes_concurrent_attempts_and_releases_on_drop() {
        let busy = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let lease = ConfirmationLease::acquire(busy.clone()).unwrap();
        assert!(ConfirmationLease::acquire(busy.clone()).is_err());
        drop(lease);
        assert!(ConfirmationLease::acquire(busy).is_ok());
    }
}
