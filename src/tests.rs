use std::{
  convert::Infallible,
  sync::{atomic::Ordering, Mutex},
  time::Duration,
};

use crate::{start_watchdog, Strategy};

unsafe fn run_forked(cb: impl FnOnce() -> Infallible) -> i32 {
  let pid = libc::fork();
  assert!(pid >= 0);
  if pid == 0 {
    #[allow(unreachable_code)]
    match cb() {}
  } else {
    let mut status = 0i32;
    let ret = libc::waitpid(pid, &mut status, 0);
    assert_eq!(ret, pid);
    status
  }
}

#[test]
fn test_watchdog_realtime_only_success() {
  unsafe {
    let status = run_forked(|| {
      let ctx = start_watchdog(Strategy::RealtimeOnly, Duration::from_millis(100));

      for _ in 0..10 {
        ctx.counter.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(50));
      }
      std::process::exit(1);
    });
    assert!(libc::WIFEXITED(status));
    assert_eq!(libc::WEXITSTATUS(status), 1);
  }
}

#[test]
fn test_watchdog_realtime_only_fail() {
  unsafe {
    let status = run_forked(|| {
      let _ctx = start_watchdog(Strategy::RealtimeOnly, Duration::from_millis(100));

      // deadlock a mutex
      let mu = Mutex::new(0);
      let _g1 = mu.lock().unwrap();
      let _g2 = mu.lock().unwrap();

      std::process::exit(1);
    });
    assert!(libc::WIFSIGNALED(status));
    assert_eq!(libc::WTERMSIG(status), 4); // SIGILL
  }
}

#[test]
fn test_watchdog_fallback_only_fail() {
  unsafe {
    let status = run_forked(|| {
      let _ctx = start_watchdog(Strategy::FallbackOnly, Duration::from_millis(100));

      // deadlock a mutex
      let mu = Mutex::new(0);
      let _g1 = mu.lock().unwrap();
      let _g2 = mu.lock().unwrap();

      std::process::exit(1);
    });
    assert!(libc::WIFSIGNALED(status));
    assert_eq!(libc::WTERMSIG(status), 6); // SIGABRT
  }
}

#[test]
fn test_watchdog_fallback_only_success() {
  unsafe {
    let status = run_forked(|| {
      let ctx = start_watchdog(Strategy::FallbackOnly, Duration::from_millis(100));

      for _ in 0..10 {
        ctx.counter.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(50));
      }
      std::process::exit(1);
    });
    assert!(libc::WIFEXITED(status));
    assert_eq!(libc::WEXITSTATUS(status), 1);
  }
}
