//! Assertion and step accounting.
//!
//! Every assertion goes through [`check!`] or [`check_eq!`] and every wait
//! through the browser driver, so the runner can print how many checks ran.
//! Steps carry the case IDs from issue #62 (S1, I3, M15, B6, ...). The
//! runner fails a case that did not reach every one of its listed steps.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

static CHECKS: AtomicUsize = AtomicUsize::new(0);
static STEPS: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn count() {
    CHECKS.fetch_add(1, Ordering::SeqCst);
}

pub fn total() -> usize {
    CHECKS.load(Ordering::SeqCst)
}

/// Record that a case reached a numbered step.
pub fn step(id: &str, what: &str) {
    println!("  [{id}] {what}");
    let mut steps = STEPS.lock().unwrap_or_else(|e| e.into_inner());
    if !steps.iter().any(|s| s == id) {
        steps.push(id.to_string());
    }
}

/// Take the steps recorded since the last call.
pub fn take_steps() -> Vec<String> {
    std::mem::take(&mut *STEPS.lock().unwrap_or_else(|e| e.into_inner()))
}

/// The listed step IDs that a run did not reach.
pub fn missing<'a>(expected: &[&'a str], reached: &[String]) -> Vec<&'a str> {
    expected
        .iter()
        .copied()
        .filter(|id| !reached.iter().any(|r| r == id))
        .collect()
}

#[macro_export]
macro_rules! check {
    ($cond:expr, $($msg:tt)+) => {{
        $crate::check::count();
        let passed: bool = $cond;
        if !passed {
            anyhow::bail!($($msg)+);
        }
    }};
}

#[macro_export]
macro_rules! check_eq {
    ($left:expr, $right:expr, $what:expr) => {{
        $crate::check::count();
        let (left, right) = (&$left, &$right);
        if left != right {
            anyhow::bail!("{}: expected {:?}, got {:?}", $what, right, left);
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_missing_steps() {
        let reached = vec!["S1".to_string(), "S3".to_string()];
        assert_eq!(
            missing(&["S1", "S2", "S3", "S4"], &reached),
            vec!["S2", "S4"]
        );
        assert!(missing(&["S1"], &reached).is_empty());
    }

    fn passing() -> anyhow::Result<()> {
        check!(["a"].len() == 1, "length");
        check_eq!("a", "a", "text");
        Ok(())
    }

    fn failing() -> anyhow::Result<()> {
        check_eq!(1, 2, "number");
        Ok(())
    }

    #[test]
    fn macros_count_and_fail() {
        let before = total();
        passing().unwrap();
        assert!(total() >= before + 2);
        let error = failing().unwrap_err().to_string();
        assert_eq!(error, "number: expected 2, got 1");
    }
}
