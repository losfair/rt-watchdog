mod asm;
mod deadline;
#[cfg(test)]
mod tests;

use std::{
  cell::UnsafeCell,
  sync::atomic::{AtomicU32, AtomicUsize, Ordering},
  time::Duration,
};

use asm::PLATFORM_SUPPORTED;
use libc::c_void;

use crate::deadline::sched_attr;

extern "C" {
  fn rt_watchdog_thread_entry(ctx: &Context) -> !;
}

#[repr(C)]
pub struct Context {
  pub counter: AtomicUsize,
  fence: AtomicU32,

  stack: Stack,
}

#[repr(C, align(16))]
struct Stack(UnsafeCell<[u8; 256]>);

unsafe impl Send for Stack {}
unsafe impl Sync for Stack {}

impl Stack {
  fn end(&self) -> *mut c_void {
    unsafe { self.0.get().offset(1) as *mut c_void }
  }
}

#[derive(Copy, Clone, Debug)]
struct DeadlineParams {
  runtime: Duration,
  period: Duration,
}

impl DeadlineParams {
  fn gen_attr(&self) -> sched_attr {
    sched_attr {
      size: std::mem::size_of::<sched_attr>() as u32,
      sched_policy: deadline::consts::SCHED_DEADLINE,
      sched_flags: 0,
      sched_nice: 0,
      sched_priority: 0,
      sched_runtime: self.runtime.as_nanos() as u64,
      sched_deadline: self.period.as_nanos() as u64,
      sched_period: self.period.as_nanos() as u64,
    }
  }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Strategy {
  RealtimeOnly,
  FallbackOnly,
  RealtimeOrFallback,
}

pub fn start_watchdog(strategy: Strategy, check_interval: Duration) -> &'static Context {
  unsafe { do_start_watchdog(strategy, check_interval) }
}

unsafe fn do_start_watchdog(strategy: Strategy, check_interval: Duration) -> &'static Context {
  let page_size = libc::sysconf(libc::_SC_PAGESIZE);
  assert!(page_size >= std::mem::size_of::<Context>() as i64);
  let page_size = page_size as usize;
  let context_page = libc::mmap(
    std::ptr::null_mut(),
    page_size,
    libc::PROT_READ | libc::PROT_WRITE,
    libc::MAP_ANONYMOUS | libc::MAP_PRIVATE,
    -1,
    0,
  );
  if context_page.is_null() {
    panic!("failed to map context page");
  }
  let ctx: &'static Context = &*(context_page as *mut Context);
  let dl_params = DeadlineParams {
    runtime: Duration::from_micros(50),
    period: check_interval,
  };

  if strategy != Strategy::FallbackOnly && detect_platform_support(dl_params) {
    let rt_watchdog_thread_entry = rt_watchdog_thread_entry as usize;
    assert!(rt_watchdog_thread_entry & (page_size - 1) == 0);

    // lock code and context data pages so that swapping does not
    // break real-time guarantees
    if libc::mlock(context_page, page_size) == 0
      && libc::mlock(rt_watchdog_thread_entry as *const _, page_size) == 0
    {
      let pid = libc::clone(
        std::mem::transmute(rt_watchdog_thread_entry),
        ctx.stack.end(),
        libc::CLONE_VM
          | libc::CLONE_FS
          | libc::CLONE_FILES
          | libc::CLONE_SIGHAND
          | libc::CLONE_THREAD
          | libc::CLONE_SYSVSEM,
        ctx as *const Context as *const c_void as *mut c_void,
      );
      if pid < 0 {
        let err = std::io::Error::last_os_error();
        panic!("failed to start watchdog thread: {:?}", err);
      }
      assert!(pid > 0);

      // set priority
      let mut attr = dl_params.gen_attr();
      if libc::syscall(
        libc::SYS_sched_setattr,
        pid,
        &mut attr as *mut sched_attr,
        0,
      ) < 0
      {
        let err = std::io::Error::last_os_error();
        panic!("failed to set rt watchdog priority: {:?}", err);
      }

      // notify the thread to start
      ctx.fence.store(1, Ordering::SeqCst);
      let ret = libc::syscall(libc::SYS_futex, &ctx.fence, libc::FUTEX_WAKE, 1, 0, 0, 0);
      assert!(ret >= 0);

      // wait for ack
      while ctx.fence.load(Ordering::SeqCst) != 2 {
        let ret = libc::syscall(libc::SYS_futex, &ctx.fence, libc::FUTEX_WAIT, 1, 0, 0, 0);
        let errno = *libc::__errno_location();
        assert!(ret == 0 || (ret < 0 && errno == libc::EAGAIN)); // glibc handles EINTR?
      }

      return ctx;
    }
  }

  if strategy == Strategy::RealtimeOnly {
    panic!("failed to start realtime watchdog on current platform");
  }

  eprintln!("falling back to non-realtime watchdog");
  std::thread::spawn(move || fallback_impl(ctx, check_interval));
  ctx
}

fn fallback_impl(ctx: &'static Context, check_interval: Duration) -> ! {
  let mut last_counter = 0usize;

  loop {
    std::thread::sleep(check_interval);
    let counter = ctx.counter.load(Ordering::SeqCst);
    if counter == last_counter {
      std::process::abort();
    }
    last_counter = counter;
  }
}

unsafe fn detect_platform_support(dl_params: DeadlineParams) -> bool {
  if !PLATFORM_SUPPORTED {
    return false;
  }

  // detect availability of SCHED_DEADLINE
  let sched_res = std::thread::spawn(move || {
    let mut dl_attr = dl_params.gen_attr();
    let mut normal_attr = sched_attr {
      size: std::mem::size_of::<sched_attr>() as u32,
      sched_policy: deadline::consts::SCHED_NORMAL,
      ..Default::default()
    };
    if libc::syscall(
      libc::SYS_sched_setattr,
      0,
      &mut dl_attr as *mut sched_attr,
      0,
    ) < 0
    {
      Err(std::io::Error::last_os_error())
    } else {
      // reset attr so that thread finalizers are executed normally
      let ret = libc::syscall(
        libc::SYS_sched_setattr,
        0,
        &mut normal_attr as *mut sched_attr,
        0,
      );
      if ret != 0 {
        std::process::abort();
      }
      Ok(())
    }
  })
  .join()
  .unwrap();
  if let Err(e) = sched_res {
    eprintln!("deadline params are not supported: {:?}", e);
    return false;
  }
  true
}
