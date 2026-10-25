//! 最小浮点测试辅助。
//!
//! 只提供 guest 端测试需要的 CSR / `mstatus.FS` 读写，不参与陷阱帧或
//! 上下文切换。FP 上下文保存由 `rremu` 的 `MachineSnapshot` 覆盖。

/// `mstatus.FS` 字段的掩码。
pub const MSTATUS_FS_MASK: u32 = 0b11 << 13;
/// `mstatus.FS = Off`。
pub const FS_OFF: u32 = 0;
/// `mstatus.FS = Initial`。
pub const FS_INITIAL: u32 = 1 << 13;
/// `mstatus.FS = Clean`。
pub const FS_CLEAN: u32 = 2 << 13;
/// `mstatus.FS = Dirty`。
pub const FS_DIRTY: u32 = 3 << 13;

/// `fflags` 位：不精确。
pub const FFLAGS_INEXACT: u32 = 1 << 0;
/// `fflags` 位：下溢。
pub const FFLAGS_UNDERFLOW: u32 = 1 << 1;
/// `fflags` 位：上溢。
pub const FFLAGS_OVERFLOW: u32 = 1 << 2;
/// `fflags` 位：除以零。
pub const FFLAGS_DIVIDE_BY_ZERO: u32 = 1 << 3;
/// `fflags` 位：非法操作。
pub const FFLAGS_INVALID: u32 = 1 << 4;
/// `fflags` 已实现位。
pub const FFLAGS_MASK: u32 = 0x1f;
/// `frm` 已实现位。
pub const FRM_MASK: u32 = 0x7;

/// 读取完整的 `mstatus`。
#[cfg(target_arch = "riscv32")]
pub fn read_mstatus() -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!(
            "csrr {value}, mstatus",
            value = out(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}

#[cfg(not(target_arch = "riscv32"))]
pub fn read_mstatus() -> u32 {
    0
}

/// 读取 `mstatus.FS`，返回已移位的字段值。
pub fn read_fs() -> u32 {
    read_mstatus() & MSTATUS_FS_MASK
}

/// 写 `mstatus.FS`，保留 `mstatus` 的其它位。
#[cfg(target_arch = "riscv32")]
pub fn write_fs(fs: u32) {
    let value = (read_mstatus() & !MSTATUS_FS_MASK) | (fs & MSTATUS_FS_MASK);
    unsafe {
        core::arch::asm!(
            "csrw mstatus, {value}",
            value = in(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn write_fs(_fs: u32) {}

/// 读取 `fcsr`。
#[cfg(target_arch = "riscv32")]
pub fn read_fcsr() -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!(
            "csrr {value}, fcsr",
            value = out(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}

#[cfg(not(target_arch = "riscv32"))]
pub fn read_fcsr() -> u32 {
    0
}

/// 写入 `fcsr`。硬件会按已实现位截断。
#[cfg(target_arch = "riscv32")]
pub fn write_fcsr(value: u32) {
    unsafe {
        core::arch::asm!(
            "csrw fcsr, {value}",
            value = in(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn write_fcsr(_value: u32) {}

/// 读取 `fflags`。
#[cfg(target_arch = "riscv32")]
pub fn read_fflags() -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!(
            "csrr {value}, fflags",
            value = out(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}

#[cfg(not(target_arch = "riscv32"))]
pub fn read_fflags() -> u32 {
    0
}

/// 写入 `fflags`。
#[cfg(target_arch = "riscv32")]
pub fn write_fflags(value: u32) {
    unsafe {
        core::arch::asm!(
            "csrw fflags, {value}",
            value = in(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn write_fflags(_value: u32) {}

/// 清除全部 `fflags`。
#[cfg(target_arch = "riscv32")]
pub fn clear_fflags() {
    unsafe {
        core::arch::asm!(
            "csrw fflags, zero",
            options(nomem, nostack, preserves_flags),
        );
    }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn clear_fflags() {}

/// 置位指定 `fflags`（sticky）。
#[cfg(target_arch = "riscv32")]
pub fn set_fflags(bits: u32) {
    unsafe {
        core::arch::asm!(
            "csrs fflags, {bits}",
            bits = in(reg) bits,
            options(nomem, nostack, preserves_flags),
        );
    }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn set_fflags(_bits: u32) {}

/// 读取 `frm`。
#[cfg(target_arch = "riscv32")]
pub fn read_frm() -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!(
            "csrr {value}, frm",
            value = out(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}

#[cfg(not(target_arch = "riscv32"))]
pub fn read_frm() -> u32 {
    0
}

/// 写入 `frm`。
#[cfg(target_arch = "riscv32")]
pub fn write_frm(value: u32) {
    unsafe {
        core::arch::asm!(
            "csrw frm, {value}",
            value = in(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn write_frm(_value: u32) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fp_helpers_host() {
        assert_eq!(MSTATUS_FS_MASK, 0x6000);
        assert_eq!(FS_INITIAL, 0x2000);
        assert_eq!(FS_CLEAN, 0x4000);
        assert_eq!(FS_DIRTY, 0x6000);
        assert_eq!(FFLAGS_MASK, 0x1f);
        assert_eq!(FRM_MASK, 0x7);

        assert_eq!(read_mstatus(), 0);
        assert_eq!(read_fs(), 0);
        write_fs(FS_DIRTY);
        assert_eq!(read_fs(), 0);

        assert_eq!(read_fcsr(), 0);
        write_fcsr(0xff);
        assert_eq!(read_fcsr(), 0);
        assert_eq!(read_fflags(), 0);
        write_fflags(FFLAGS_INEXACT);
        assert_eq!(read_fflags(), 0);
        set_fflags(FFLAGS_INVALID);
        clear_fflags();
        assert_eq!(read_frm(), 0);
        write_frm(2);
        assert_eq!(read_frm(), 0);
    }
}
