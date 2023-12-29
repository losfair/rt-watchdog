# rt-watchdog

[![crates.io](https://img.shields.io/crates/v/rt-watchdog.svg)](https://crates.io/crates/rt-watchdog)

Real-time userspace watchdog for Rust. Currently supported platforms are x86-64
Linux and AArch64 Linux.

- The watchdog thread runs with the `SCHED_DEADLINE` scheduler, guaranteeing the
  highest real-time priority.
- Code and data pages used by the watchdog thread are `mlock`-ed.

If a tick is missed, the current process is immediately aborted.

An option is provided to fall back to a simple, non-realtime implementation on
unsupported platforms.

## Usage

```rust
use std::sync::atomic::Ordering;
use std::time::Duration;

let ctx = rt_watchdog::start_watchdog(
  rt_watchdog::Strategy::RealtimeOrFallback,
  Duration::from_millis(100),
);

loop {
  ctx.counter.fetch_add(1, Ordering::SeqCst);
  std::thread::sleep(Duration::from_millis(90));
}
```

The process must have the `CAP_SYS_NICE` capability to use the deadline
scheduler. Add the capability to an executable with:

```
sudo setcap "cap_sys_nice=eip" ./myapp
```
