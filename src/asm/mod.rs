#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
std::arch::global_asm!(include_str!("x86_64-linux.S"));

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
pub const PLATFORM_SUPPORTED: bool = true;

#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
std::arch::global_asm!(include_str!("aarch64-linux.S"));

#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
pub const PLATFORM_SUPPORTED: bool = true;

#[cfg(not(any(
  all(target_arch = "x86_64", target_os = "linux"),
  all(target_arch = "aarch64", target_os = "linux")
)))]
#[no_mangle]
extern "C" fn rt_watchdog_thread_entry() {
  unimplemented!()
}

#[cfg(not(any(
  all(target_arch = "x86_64", target_os = "linux"),
  all(target_arch = "aarch64", target_os = "linux")
)))]
pub const PLATFORM_SUPPORTED: bool = false;
