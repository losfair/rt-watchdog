.section .text
.align 4096
.globl rt_watchdog_thread_entry
rt_watchdog_thread_entry:
  // save context addr to r14
  mov r14, rdi

  // start fence: wait until fence becomes 1 using futex (202)
1:
  mov eax, [r14 + 8]
  cmp eax, 1
  je 2f

  lea rdi, [r14 + 8] // uaddr
  xor rsi, rsi // op = FUTEX_WAIT
  xor rdx, rdx // val
  xor r10, r10 // utime
  xor r8, r8 // uaddr2
  xor r9, r9 // val3
  mov rax, 202
  syscall

  cmp rax, 0 # wakeup
  je 1b

  cmp rax, -11 # EAGAIN
  je 1b

  cmp rax, -4 # EINTR
  je 1b

  // fail
  ud2

2:
  // thread start ack: atomically write value 2 to fence
  mov rax, 2
  xchg [r14 + 8], eax

  // futex notify
  lea rdi, [r14 + 8] // uaddr
  mov rsi, 1 // op = FUTEX_WAKE
  mov rdx, 1 // val = 1 (num_waiters)
  xor r10, r10 // utime
  xor r8, r8 // uaddr2
  xor r9, r9 // val3
  mov rax, 202
  syscall

  // if return value is below 0, abort process
  cmp rax, 0
  jb 3f

  // local counter init: 0
  xor r12, r12

  // load remote counter address to r13
  lea r13, [r14 + 0]
1:
  // issue syscall sched_yield (24)
  mov rax, 24
  syscall

  // atomically load the 64-bit remote counter
  mov rax, [r13]

  // if rax is equal to r12, counter did not advance in the previous period - abort the process
  cmp rax, r12
  je 3f

  // save rax to r12
  mov r12, rax

  // loop
  jmp 1b

3:
  int 3
  ud2

.align 4096, 0x90
