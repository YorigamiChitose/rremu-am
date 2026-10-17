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

/// 启动参数捕获：`_start` 在清 BSS 后保存 a0(hartid) / a1(dtb)，供 guest 读取。
#[cfg(target_arch = "riscv32")]
#[unsafe(no_mangle)]
static mut BOOT_A0: u32 = 0;
#[cfg(target_arch = "riscv32")]
#[unsafe(no_mangle)]
static mut BOOT_A1: u32 = 0;

/// 启动时的 hartid（qemu-virt: a0）。
#[cfg(target_arch = "riscv32")]
pub fn boot_hartid() -> u32 {
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!(BOOT_A0)) }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn boot_hartid() -> u32 {
    0
}

/// 启动时的 DTB 地址（qemu-virt: a1；未加载 DTB 时为 0）。
#[cfg(target_arch = "riscv32")]
pub fn boot_dtb() -> u32 {
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!(BOOT_A1)) }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn boot_dtb() -> u32 {
    0
}

/// 配置 PMP0 为覆盖 4GiB 的 NAPOT 区间，允许 S/U 读写执行所有物理地址。
///
/// 仅用于测试/教学环境；真实 OS 应配置最小权限 PMP。
#[cfg(target_arch = "riscv32")]
pub fn pmp_allow_all() {
    unsafe {
        core::arch::asm!(
            "li t0, 0x1fffffff",
            "csrw pmpaddr0, t0",
            "li t0, 0x1f",
            "csrw pmpcfg0, t0",
            out("t0") _,
        );
    }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn pmp_allow_all() {}

/// 从 M-mode 进入 S-mode：设置 MPP=S、mepc 与 a0/a1 后执行 `mret`，不返回。
#[cfg(target_arch = "riscv32")]
pub fn enter_supervisor(entry: u32, arg0: u32, arg1: u32) -> ! {
    unsafe {
        core::arch::asm!(
            "csrr t0, mstatus",
            "li t1, {mpp_mask}",
            "not t1, t1",
            "and t0, t0, t1",
            "li t1, {mpp_s}",
            "or t0, t0, t1",
            "csrw mstatus, t0",
            "csrw mepc, {entry}",
            "mv a0, {arg0}",
            "mv a1, {arg1}",
            "mret",
            mpp_mask = const (0b11u32 << 11),
            mpp_s = const (1u32 << 11),
            entry = in(reg) entry,
            arg0 = in(reg) arg0,
            arg1 = in(reg) arg1,
            clobber_abi("C"),
            options(noreturn),
        );
    }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn enter_supervisor(_entry: u32, _arg0: u32, _arg1: u32) -> ! {
    panic!("enter_supervisor called on non-RISC-V target");
}

// 启动入口：设栈、清 BSS、调用 rust_main。
//
// 放在 .text.init 段（linker.ld 保证 _start 排在 .text 最前）。
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
    // 保存 boot a0/a1（清 BSS 之后，避免被清零）
    "la t0, BOOT_A0",
    "sw a0, 0(t0)",
    "la t0, BOOT_A1",
    "sw a1, 0(t0)",
    // 安装默认陷阱入口（BIOS 式兜底；guest 可经 trap_init 覆盖 handler）
    "la t0, __am_asm_trap",
    "csrw mtvec, t0",
    "call rust_main",
    // rust_main 应不返回（-> !）；兜底死循环
    "3:",
    "j 3b",
);
