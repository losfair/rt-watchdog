.section .text
.align 14
.globl rt_watchdog_thread_entry
rt_watchdog_thread_entry:
  // save context addr to x14
  mov x14, x0

  // start fence: wait until fence becomes 1 using futex (98)
1:
  add x0, x14, #8
  ldar w8, [x0]
  cmp w8, #1
  b.eq 2f

  // Prepare for futex syscall: FUTEX_WAIT
  add x0, x14, #8  // uaddr
  mov x1, #0      // op = FUTEX_WAIT
  mov x2, #0      // val
  mov x3, #0      // timeout (NULL)
  mov x8, #98     // syscall number for futex
  svc #0          // invoke syscall

  // Handle return values
  cmp x0, #0      // wakeup
  b.eq 1b

  cmp x0, #-11    // EAGAIN
  b.eq 1b

  cmp x0, #-4     // EINTR
  b.eq 1b

  // fail
  brk #0

2:
  // thread start ack: atomically write value 2 to fence
  mov w8, #2
  add x0, x14, #8
  stlr w8, [x0]

  // futex notify
  add x0, x14, #8  // uaddr
  mov x1, #1       // op = FUTEX_WAKE
  mov x2, #1       // val = 1 (num_waiters)
  mov x8, #98      // syscall number for futex
  svc #0           // invoke syscall

  // if return value is negative, abort process
  cmp x0, #0
  b.lt 3f

  // local counter init: 0
  eor x12, x12, x12

  // load remote counter address to x13
  add x13, x14, #0
1:
  // issue syscall sched_yield (124)
  mov x8, #124
  svc #0

  // atomically load the 64-bit remote counter
  ldar x0, [x13]

  // if x0 is equal to x12, counter did not advance - abort
  cmp x0, x12
  b.eq 3f

  // save x0 to x12
  mov x12, x0

  // loop
  b 1b

3:
  brk #0

.align 14, 0x00
