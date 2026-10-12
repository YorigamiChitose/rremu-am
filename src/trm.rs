//! TRM（图灵机）层：启动入口与基本 I/O。

#[cfg(target_arch = "riscv32")]
use crate::EXIT_MAGIC;
#[cfg(target_arch = "riscv32")]
use crate::arch;

/// 以 `code` 为退出码终止模拟（宿主陷阱协议）。
///
/// 实现：M-mode `ecall`，a7 = EXIT_MAGIC，a0 = code。
#[cfg(target_arch = "riscv32")]
pub fn halt(code: i32) -> ! {
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a0") code as u32,
            in("a7") EXIT_MAGIC,
            options(noreturn),
        );
    }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn halt(code: i32) -> ! {
    panic!("halt({code}) called on non-RISC-V target");
}

/// 向 UART THR 写一字节（非阻塞输出）。
#[cfg(target_arch = "riscv32")]
pub fn putc(ch: u8) {
    unsafe {
        core::arch::asm!(
            "sb {ch}, 0({addr})",
            ch = in(reg) ch,
            addr = in(reg) arch::uart::THR,
        );
    }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn putc(_ch: u8) {}

// 启动入口：设栈、清 BSS、调用 rust_main。
///
/// 放在 .text.init 段（linker.ld 保证 _start 排在 .text 最前）。
#[cfg(target_arch = "riscv32")]
core::arch::global_asm!(
    ".section .text.init",
    ".global _start",
    "_start:",
    "la sp, _stack_top",
    // 清 BSS（_bss_start ~ _bss_end 由 linker.ld 定义）
    "la t0, _bss_start",
    "la t1, _bss_end",
    "1:",
    "bgeu t0, t1, 2f",
    "sw zero, 0(t0)",
    "addi t0, t0, 4",
    "j 1b",
    "2:",
    "call rust_main",
    // rust_main 应不返回（-> !）；兜底死循环
    "3:",
    "j 3b",
);
