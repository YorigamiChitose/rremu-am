//! CTE（陷阱/上下文扩展）层：Context、Event、陷阱入口与派发。

use crate::EXIT_MAGIC;

/// Context 总大小（35 × 4 = 140 字节），分配时对齐到 16 → 144 字节。
pub const CONTEXT_SIZE: usize = 144;

/// 陷阱上下文：32 GPR + mcause + mstatus + mepc。
///
/// `#[repr(C)]` 保证字段顺序与汇编布局一致：
/// - `gpr[0]`（x0）恒 0，不用
/// - `gpr[2]`（x2/sp）在陷阱入口存原始 sp（addi 算术恢复）
/// - `gpr[17]`（a7）用于 Event 解码
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Context {
    pub gpr: [u32; 32],
    pub mcause: u32,
    pub mstatus: u32,
    pub mepc: u32,
}

/// 事件类型（11 变体）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum Event {
    /// 不应到达（ecall+a7=EXIT_MAGIC 被 CPU 拦截前退出）。
    Null,
    /// 机器定时器中断（mcause bit31=1, code=7）。
    IrqTimer,
    /// 机器软件中断（mcause bit31=1, code=3）。
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
    /// 其他未识别。
    Error,
}

/// 事件处理函数签名：返回 Context 指针以支持上下文切换。
pub type Handler = extern "C" fn(Event, *mut Context) -> *mut Context;

/// 从 mcause + a7 解码 Event。
pub fn decode_event(ctx: &Context) -> Event {
    let is_interrupt = (ctx.mcause >> 31) & 1 == 1;
    let cause = ctx.mcause & 0x7fff_ffff;
    let a7 = ctx.gpr[17]; // a7 = x17

    match (is_interrupt, cause, a7) {
        (true, 7, _) => Event::IrqTimer,
        (true, 3, _) => Event::IrqSoftware,
        (false, 11, a7) if a7 == u32::MAX => Event::Yield,
        (false, 11, a7) if a7 == EXIT_MAGIC => Event::Null,
        (false, 11, _) => Event::Syscall,
        (false, 2, _) => Event::IllInsn,
        (false, 3, _) => Event::Breakpoint,
        (false, 0 | 4 | 6, _) => Event::Misalign,
        (false, 5, _) => Event::LoadFault,
        (false, 7, _) => Event::StoreFault,
        _ => Event::Error,
    }
}

/// 全局处理函数指针（trap_init 设置）。
static mut HANDLER: Option<Handler> = None;

/// 安装事件处理函数并设 mtvec 指向陷阱入口。
#[cfg(target_arch = "riscv32")]
pub fn trap_init(handler: Handler) {
    unsafe { HANDLER = Some(handler) };
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

/// 陷阱派发入口（由汇编调用）。
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __am_irq_handle(ctx: *mut Context) -> *mut Context {
    let event = decode_event(unsafe { &*ctx });
    let handler = unsafe { HANDLER }.unwrap_or(default_handler);
    handler(event, ctx)
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
extern "C" fn default_handler(_ev: Event, ctx: *mut Context) -> *mut Context {
    ctx
}

// 陷阱入口汇编。
//
// 布局：Context 在栈上，35×4=140 字节，分配 144（16 对齐）。
// slot 2（offset 8）存原始 sp，由 addi 算术恢复。
// x5（t0）先存再用作 scratch；恢复时最后恢复。
#[cfg(target_arch = "riscv32")]
core::arch::global_asm!(
    ".section .text.trap",
    ".align 2",
    ".global __am_asm_trap",
    "__am_asm_trap:",
    // 分配 Context（144 字节，16 对齐）
    "addi sp, sp, -144",
    // 存 x1（ra）到 slot 1
    "sw ra, 4(sp)",
    // 存 x3-x5（gp/tp/t0），t0 须先存再作 scratch
    "sw gp, 12(sp)",
    "sw tp, 16(sp)",
    "sw t0, 20(sp)",
    // 存 x6-x31
    "sw t1, 24(sp)",
    "sw t2, 28(sp)",
    "sw s0, 32(sp)",
    "sw s1, 36(sp)",
    "sw a0, 40(sp)",
    "sw a1, 44(sp)",
    "sw a2, 48(sp)",
    "sw a3, 52(sp)",
    "sw a4, 56(sp)",
    "sw a5, 60(sp)",
    "sw a6, 64(sp)",
    "sw a7, 68(sp)",
    "sw s2, 72(sp)",
    "sw s3, 76(sp)",
    "sw s4, 80(sp)",
    "sw s5, 84(sp)",
    "sw s6, 88(sp)",
    "sw s7, 92(sp)",
    "sw s8, 96(sp)",
    "sw s9, 100(sp)",
    "sw s10, 104(sp)",
    "sw s11, 108(sp)",
    "sw t3, 112(sp)",
    "sw t4, 116(sp)",
    "sw t5, 120(sp)",
    "sw t6, 124(sp)",
    // 存原始 sp 到 slot 2
    "addi t0, sp, 144",
    "sw t0, 8(sp)",
    // 存 CSR
    "csrr t0, mcause",
    "sw t0, 128(sp)",
    "csrr t0, mstatus",
    "sw t0, 132(sp)",
    "csrr t0, mepc",
    "sw t0, 136(sp)",
    // 调用 __am_irq_handle(sp)
    "mv a0, sp",
    "call __am_irq_handle",
    // 切到返回的 Context
    "mv sp, a0",
    // 恢复 CSR
    "lw t0, 132(sp)",
    "csrw mstatus, t0",
    "lw t0, 136(sp)",
    "csrw mepc, t0",
    "lw t0, 128(sp)",
    "csrw mcause, t0",
    // 恢复 x1, x3-x4
    "lw ra, 4(sp)",
    "lw gp, 12(sp)",
    "lw tp, 16(sp)",
    // 恢复 x6-x31（跳过 x5/t0）
    "lw t1, 24(sp)",
    "lw t2, 28(sp)",
    "lw s0, 32(sp)",
    "lw s1, 36(sp)",
    "lw a0, 40(sp)",
    "lw a1, 44(sp)",
    "lw a2, 48(sp)",
    "lw a3, 52(sp)",
    "lw a4, 56(sp)",
    "lw a5, 60(sp)",
    "lw a6, 64(sp)",
    "lw a7, 68(sp)",
    "lw s2, 72(sp)",
    "lw s3, 76(sp)",
    "lw s4, 80(sp)",
    "lw s5, 84(sp)",
    "lw s6, 88(sp)",
    "lw s7, 92(sp)",
    "lw s8, 96(sp)",
    "lw s9, 100(sp)",
    "lw s10, 104(sp)",
    "lw s11, 108(sp)",
    "lw t3, 112(sp)",
    "lw t4, 116(sp)",
    "lw t5, 120(sp)",
    "lw t6, 124(sp)",
    // 最后恢复 t0
    "lw t0, 20(sp)",
    // 算术恢复 sp
    "addi sp, sp, 144",
    "mret",
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_layout() {
        let ctx = Context { gpr: [0; 32], mcause: 0, mstatus: 0, mepc: 0 };
        let base = &ctx as *const _ as *const u8;
        assert_eq!(&ctx.gpr[0] as *const _ as usize - base as usize, 0);
        assert_eq!(&ctx.gpr[1] as *const _ as usize - base as usize, 4);
        assert_eq!(&ctx.gpr[2] as *const _ as usize - base as usize, 8);
        assert_eq!(&ctx.gpr[17] as *const _ as usize - base as usize, 68);
        assert_eq!(&ctx.gpr[31] as *const _ as usize - base as usize, 124);
        assert_eq!(&ctx.mcause as *const _ as usize - base as usize, 128);
        assert_eq!(&ctx.mstatus as *const _ as usize - base as usize, 132);
        assert_eq!(&ctx.mepc as *const _ as usize - base as usize, 136);
        assert_eq!(std::mem::size_of::<Context>(), 140);
    }

    fn mk(mcause: u32, a7: u32) -> Context {
        let mut ctx = Context { gpr: [0; 32], mcause, mstatus: 0, mepc: 0 };
        ctx.gpr[17] = a7;
        ctx
    }

    #[test]
    fn decode_interrupts() {
        assert_eq!(decode_event(&mk(1 << 31 | 7, 0)), Event::IrqTimer);
        assert_eq!(decode_event(&mk(1 << 31 | 3, 0)), Event::IrqSoftware);
    }

    #[test]
    fn decode_ecall_variants() {
        assert_eq!(decode_event(&mk(11, u32::MAX)), Event::Yield);
        assert_eq!(decode_event(&mk(11, EXIT_MAGIC)), Event::Null);
        assert_eq!(decode_event(&mk(11, 42)), Event::Syscall);
    }

    #[test]
    fn decode_exceptions() {
        assert_eq!(decode_event(&mk(2, 0)), Event::IllInsn);
        assert_eq!(decode_event(&mk(3, 0)), Event::Breakpoint);
        assert_eq!(decode_event(&mk(0, 0)), Event::Misalign);
        assert_eq!(decode_event(&mk(4, 0)), Event::Misalign);
        assert_eq!(decode_event(&mk(6, 0)), Event::Misalign);
        assert_eq!(decode_event(&mk(5, 0)), Event::LoadFault);
        assert_eq!(decode_event(&mk(7, 0)), Event::StoreFault);
    }

    #[test]
    fn decode_unknown() {
        assert_eq!(decode_event(&mk(99, 0)), Event::Error);
        assert_eq!(decode_event(&mk(1 << 31 | 99, 0)), Event::Error);
    }
}
