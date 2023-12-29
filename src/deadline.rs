#[allow(dead_code)]
pub mod consts {
  pub const SCHED_NORMAL: u32 = 0;
  pub const SCHED_FIFO: u32 = 1;
  pub const SCHED_RR: u32 = 2;
  pub const SCHED_BATCH: u32 = 3;
  pub const SCHED_IDLE: u32 = 5;
  pub const SCHED_DEADLINE: u32 = 6;
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct sched_attr {
  pub size: u32,
  pub sched_policy: u32,
  pub sched_flags: u64,
  pub sched_nice: i32,
  pub sched_priority: u32,
  pub sched_runtime: u64,
  pub sched_deadline: u64,
  pub sched_period: u64,
}
