//! Trap cause 到 Event 的映射。

use super::{Context, SContext};
use crate::EXIT_MAGIC;

/// 事件类型（12 变体）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum Event {
    /// 不应到达（ecall+a7=EXIT_MAGIC 被 CPU 拦截前退出）。
    Null,
    /// 机器定时器中断（mcause bit31=1, code=7）。
    IrqTimer,
    /// 软件中断（machine code=3 或 supervisor code=1）。
    IrqSoftware,
    /// AM yield（ecall, a7=u32::MAX）。
    Yield,
    /// 普通 ecall（M-mode, a7 非 EXIT_MAGIC / u32::MAX）。
    Syscall,
    /// 非法指令（mcause code=2）。
    IllInsn,
    /// 断点（mcause code=3）。
    Breakpoint,
    /// 对齐违例（mcause code=0|4|6 合并）。
    Misalign,
    /// 取数访问异常（mcause code=5）。
    LoadFault,
    /// 存数访问异常（mcause code=7）。
    StoreFault,
    /// 外部中断（machine code=11 或 supervisor code=9）。
    IrqExternal,
    /// 其他未识别。
    Error,
}

/// 事件处理函数签名：返回 Context 指针以支持上下文切换。
pub type Handler = extern "C" fn(Event, *mut Context) -> *mut Context;

/// S-mode 事件处理函数签名。
pub type SHandler = extern "C" fn(Event, *mut SContext) -> *mut SContext;

fn decode_cause(cause: u32, a7: u32) -> Event {
    let is_interrupt = (cause >> 31) & 1 == 1;
    let code = cause & 0x7fff_ffff;

    match (is_interrupt, code, a7) {
        (true, 7 | 5, _) => Event::IrqTimer,
        (true, 3 | 1, _) => Event::IrqSoftware,
        (true, 11 | 9, _) => Event::IrqExternal,
        (false, 11, a7) if a7 == u32::MAX => Event::Yield,
        (false, 11, a7) if a7 == EXIT_MAGIC => Event::Null,
        (false, 11, _) => Event::Syscall,
        // ECALL_FROM_U：U-mode ecall，语义与 syscall 相同
        (false, 8, _) => Event::Syscall,
        // ECALL_FROM_S：S-mode ecall，语义与 syscall 相同
        (false, 9, _) => Event::Syscall,
        (false, 2, _) => Event::IllInsn,
        (false, 3, _) => Event::Breakpoint,
        (false, 0 | 4 | 6, _) => Event::Misalign,
        (false, 5, _) => Event::LoadFault,
        (false, 7, _) => Event::StoreFault,
        // INSTR_ACCESS_FAULT：取指访问异常，暂归 Error
        (false, 1, _) => Event::Error,
        _ => Event::Error,
    }
}

/// 从 mcause + a7 解码 Event。
pub fn decode_event(ctx: &Context) -> Event {
    decode_cause(ctx.mcause, ctx.gpr[17])
}

/// 从 scause + a7 解码 Event。
pub fn decode_event_s(ctx: &SContext) -> Event {
    decode_cause(ctx.scause, ctx.gpr[17])
}
