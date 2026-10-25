//! HAL 全局状态的唯一集中点。
//!
//! 当前模型假设单 hart；所有静态状态都在本模块内，并通过窄访问器读写。
//! 其他模块不得直接声明或访问 `static mut`。

use crate::cte::{Handler, SHandler, TrapInfo};

/// 默认陷阱处理写入的信息（no_std 无 printf，经全局变量暴露）。
pub static mut DEFAULT_TRAP_INFO: TrapInfo = TrapInfo { cause: 0, pc: 0 };

static mut M_HANDLER: Option<Handler> = None;
static mut S_HANDLER: Option<SHandler> = None;

/// 启动参数捕获：`_start` 在清 BSS 后保存 a0(hartid) / a1(dtb)。
#[cfg(target_arch = "riscv32")]
#[unsafe(no_mangle)]
pub(crate) static mut BOOT_A0: u32 = 0;
#[cfg(target_arch = "riscv32")]
#[unsafe(no_mangle)]
pub(crate) static mut BOOT_A1: u32 = 0;

#[cfg(target_arch = "riscv32")]
pub(crate) fn install_m_handler(handler: Handler) {
    unsafe { M_HANDLER = Some(handler) };
}

#[cfg(target_arch = "riscv32")]
pub(crate) fn install_s_handler(handler: SHandler) {
    unsafe { S_HANDLER = Some(handler) };
}

pub(crate) fn m_handler() -> Option<Handler> {
    unsafe { M_HANDLER }
}

pub(crate) fn s_handler() -> Option<SHandler> {
    unsafe { S_HANDLER }
}

pub(crate) fn write_default_trap_info(cause: u32, pc: u32) {
    unsafe {
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!(DEFAULT_TRAP_INFO),
            TrapInfo { cause, pc },
        );
    }
}

/// 启动时的 hartid（qemu-virt: a0）。
#[cfg(target_arch = "riscv32")]
pub(crate) fn read_boot_hartid() -> u32 {
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!(BOOT_A0)) }
}

/// 启动时的 DTB 地址（qemu-virt: a1；未加载 DTB 时为 0）。
#[cfg(target_arch = "riscv32")]
pub(crate) fn read_boot_dtb() -> u32 {
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!(BOOT_A1)) }
}
