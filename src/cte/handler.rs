//! Handler 注册、派发与默认 trap 行为。

use super::{decode_event, decode_event_s, Context, Event, Handler, SContext, SHandler, DEFAULT_TRAP_EXIT};

/// 安装事件处理函数并设 mtvec 指向陷阱入口。
#[cfg(target_arch = "riscv32")]
pub fn trap_init(handler: Handler) {
    crate::state::install_m_handler(handler);
    unsafe {
        core::arch::asm!(
            "la t0, __am_asm_trap",
            "csrw mtvec, t0",
            out("t0") _,
        );
    }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn trap_init(_handler: Handler) {}

/// 安装 S-mode 事件处理函数并设 stvec 指向 S 陷阱入口。
#[cfg(target_arch = "riscv32")]
pub fn trap_init_s(handler: SHandler) {
    crate::state::install_s_handler(handler);
    unsafe {
        core::arch::asm!(
            "la t0, __am_asm_strap",
            "csrw stvec, t0",
            out("t0") _,
        );
    }
}

#[cfg(not(target_arch = "riscv32"))]
pub fn trap_init_s(_handler: SHandler) {}

/// 陷阱派发入口（由汇编调用）。
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __am_irq_handle(ctx: *mut Context) -> *mut Context {
    let event = decode_event(unsafe { &*ctx });
    let handler = crate::state::m_handler().unwrap_or(default_handler);
    handler(event, ctx)
}

/// S-mode 陷阱派发入口（由汇编调用）。
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __am_s_irq_handle(ctx: *mut SContext) -> *mut SContext {
    let event = decode_event_s(unsafe { &*ctx });
    let handler = crate::state::s_handler().unwrap_or(default_s_handler);
    handler(event, ctx)
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
extern "C" fn default_s_handler(_ev: Event, ctx: *mut SContext) -> *mut SContext {
    // 未安装 S handler：记录现场后自旋（S-mode 不能直接 halt）。
    unsafe {
        crate::state::write_default_trap_info((*ctx).scause, (*ctx).sepc);
    }
    loop {
        core::hint::spin_loop();
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
extern "C" fn default_handler(_ev: Event, ctx: *mut Context) -> *mut Context {
    // 未安装 handler：记录 mcause/mepc 后以 DEFAULT_TRAP_EXIT 终止。
    // 用 volatile 写，避免 release+LTO 下因无程序内读者而被死存储消除。
    unsafe {
        crate::state::write_default_trap_info((*ctx).mcause, (*ctx).mepc);
    }
    crate::halt(DEFAULT_TRAP_EXIT)
}

