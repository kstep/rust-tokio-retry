use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures::future::{err, ok};
use std::iter::Take;
use tokio::runtime::Runtime;
use tokio_retry::{Retry, RetryIf};

#[test]
fn attempts_just_once() {
    use std::iter::empty;
    let mut runtime = Runtime::new().unwrap();
    let counter = Arc::new(AtomicUsize::new(0));
    let cloned_counter = counter.clone();
    let future = Retry::spawn(empty(), move || {
        cloned_counter.fetch_add(1, Ordering::SeqCst);
        err::<(), u64>(42)
    });

    let res = runtime.block_on(future);

    assert_eq!(res, Err(42));
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn attempts_until_max_retries_exceeded() {
    use tokio_retry::strategy::FixedInterval;
    let s = FixedInterval::from_millis(100).take(2);
    let mut runtime = Runtime::new().unwrap();
    let counter = Arc::new(AtomicUsize::new(0));
    let cloned_counter = counter.clone();
    let future = Retry::spawn(s, move || {
        cloned_counter.fetch_add(1, Ordering::SeqCst);
        err::<(), u64>(42)
    });
    let res = runtime.block_on(future);

    assert_eq!(res, Err(42));
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

#[test]
fn attempts_until_success() {
    use tokio_retry::strategy::FixedInterval;
    let s = FixedInterval::from_millis(100);
    let mut runtime = Runtime::new().unwrap();
    let counter = Arc::new(AtomicUsize::new(0));
    let cloned_counter = counter.clone();
    let future = Retry::spawn(s, move || {
        let previous = cloned_counter.fetch_add(1, Ordering::SeqCst);
        if previous < 3 {
            err::<(), u64>(42)
        } else {
            ok::<(), u64>(())
        }
    });
    let res = runtime.block_on(future);

    assert_eq!(res, Ok(()));
    assert_eq!(counter.load(Ordering::SeqCst), 4);
}

#[test]
fn compatible_with_tokio_core() {
    use tokio_retry::strategy::FixedInterval;
    let s = FixedInterval::from_millis(100);
    let mut rt = Runtime::new().unwrap();
    let counter = Arc::new(AtomicUsize::new(0));
    let cloned_counter = counter.clone();
    let future = Retry::spawn(s, move || {
        let previous = cloned_counter.fetch_add(1, Ordering::SeqCst);
        if previous < 3 {
            err::<(), u64>(42)
        } else {
            ok::<(), u64>(())
        }
    });
    let res = rt.block_on(future);

    assert_eq!(res, Ok(()));
    assert_eq!(counter.load(Ordering::SeqCst), 4);
}

#[test]
fn attempts_retry_only_if_given_condition_is_true() {
    use tokio_retry::strategy::FixedInterval;
    let s = FixedInterval::from_millis(100).take(5);
    let mut runtime = Runtime::new().unwrap();
    let counter = Arc::new(AtomicUsize::new(0));
    let cloned_counter = counter.clone();
    let future: RetryIf<Take<FixedInterval>, _, fn(&Result<(), usize>) -> bool> = RetryIf::spawn(
        s,
        move || {
            let previous = cloned_counter.fetch_add(1, Ordering::SeqCst);
            err::<(), usize>(previous + 1)
        },
        |e: &Result<(), usize>| *e.as_ref().err().unwrap() < 3,
    );
    let res = runtime.block_on(future);

    assert_eq!(res, Err(3));
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}
