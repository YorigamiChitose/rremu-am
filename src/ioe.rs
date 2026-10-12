//! IOE（I/O 扩展）层：UART 读取与 CLINT 定时器。

#[cfg(target_arch = "riscv32")]
use crate::arch;

/// 从 UART 读取一字节（若 Data Ready）。
#[cfg(target_arch = "riscv32")]
pub fn uart_getc() -> Option<u8> {
    let lsr: u8;
    unsafe {
        core::arch::asm!(
            "lbu {lsr}, 0({addr})",
            lsr = out(reg) lsr,
            addr = in(reg) arch::uart::LSR,
        );
    }
    if lsr & arch::uart::LSR_DR == 0 {
        return None;
    }
    let ch: u8;
    unsafe {
        core::arch::asm!(
            "lbu {ch}, 0({addr})",
            ch = out(reg) ch,
            addr = in(reg) arch::uart::RDR,
        );
    }
    Some(ch)
}

#[cfg(not(target_arch = "riscv32"))]
pub fn uart_getc() -> Option<u8> {
    None
}

/// 设置 CLINT mtimecmp（64 位）。
#[cfg(target_arch = "riscv32")]
pub fn clint_set_timercmp(val: u64) {
    let lo = val as u32;
    let hi = (val >> 32) as u32;
    unsafe {
        core::arch::asm!(
            "sw {lo}, 0({addr})",
            "sw {hi}, 4({addr})",
            lo = in(reg) lo,
            hi = in(reg) hi,
            addr = in(reg) arch::clint::MTIMECMP,
        );
    }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn clint_set_timercmp(_val: u64) {}

/// 读取 CLINT mtime（64 位）。
///
/// rremu 为单线程模拟器，直接读 lo/hi 即可（无并发翻转风险）。
#[cfg(target_arch = "riscv32")]
pub fn clint_get_time() -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe {
        core::arch::asm!(
            "lw {lo}, 0({addr})",
            "lw {hi}, 4({addr})",
            lo = out(reg) lo,
            hi = out(reg) hi,
            addr = in(reg) arch::clint::MTIME,
        );
    }
    (lo as u64) | ((hi as u64) << 32)
}

#[cfg(not(target_arch = "riscv32"))]
pub fn clint_get_time() -> u64 {
    0
}

/// 设置 CLINT msip（软件中断）。
#[cfg(target_arch = "riscv32")]
pub fn clint_set_msip(val: u32) {
    unsafe {
        core::arch::asm!(
            "sw {val}, 0({addr})",
            val = in(reg) val,
            addr = in(reg) arch::clint::MSIP,
        );
    }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn clint_set_msip(_val: u32) {}
