//! CTE（陷阱/上下文扩展）层：Context、Event、陷阱入口与派发。

use crate::EXIT_MAGIC;

/// Context 总大小（35 × 4 = 140 字节），分配时对齐到 16 → 144 字节。
pub const CONTEXT_SIZE: usize = 144;

/// 默认陷阱处理的退出码：guest 未安装 handler（未调 trap_init）时，
/// 陷阱被默认 handler 捕获后以该码 halt。
pub const DEFAULT_TRAP_EXIT: i32 = 0x10;

/// 默认陷阱信息：记录最近一次 trap 的 mcause / mepc，供调试者（CLI `x`）读取。
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TrapInfo {
    pub cause: u32,
    pub pc: u32,
}

/// 默认陷阱处理写入的信息（no_std 无 printf，经全局变量暴露）。
pub static mut DEFAULT_TRAP_INFO: TrapInfo = TrapInfo { cause: 0, pc: 0 };

/// 陷阱上下文：32 GPR + mcause + mstatus + mepc。
///
/// `#[repr(C)]` 保证字段顺序与汇编布局一致：
/// - `gpr[0]`（x0）恒 0，不用
/// - `gpr[2]`（x2/sp）在陷阱入口存原始 sp（恢复时从 gpr[2] 读回）
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
        // M_EI（外部中断）：rremu 无此中断源，显式归 Error，避免静默兜底
        (true, 11, _) => Event::Error,
        (false, 11, a7) if a7 == u32::MAX => Event::Yield,
        (false, 11, a7) if a7 == EXIT_MAGIC => Event::Null,
        (false, 11, _) => Event::Syscall,
        // ECALL_FROM_U：U-mode ecall，语义与 syscall 相同
        (false, 8, _) => Event::Syscall,
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
    // 未安装 handler：记录 mcause/mepc 后以 DEFAULT_TRAP_EXIT 终止。
    // 用 volatile 写，避免 release+LTO 下因无程序内读者而被死存储消除。
    unsafe {
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!(DEFAULT_TRAP_INFO),
            TrapInfo { cause: (*ctx).mcause, pc: (*ctx).mepc },
        );
    }
    crate::halt(DEFAULT_TRAP_EXIT)
}

// 陷阱入口汇编。
//
// 布局：Context 在栈上，35×4=140 字节，分配 144（16 对齐）。
// slot 2（offset 8）存原始 sp，恢复时从 gpr[2] 读回（支持上下文切换）。
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
    // 从 gpr[2] 恢复 sp（支持上下文切换）
    "lw sp, 8(sp)",
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

    #[test]
    fn trap_info_layout_and_default_exit() {
        assert_eq!(DEFAULT_TRAP_EXIT, 0x10);
        let info = TrapInfo { cause: 0, pc: 0 };
        assert_eq!(std::mem::size_of::<TrapInfo>(), 8);
        let base = &info as *const _ as *const u8;
        assert_eq!(&info.cause as *const _ as usize - base as usize, 0);
        assert_eq!(&info.pc as *const _ as usize - base as usize, 4);
    }

    fn mk(mcause: u32, a7: u32) -> Context {
        let mut ctx = Context { gpr: [0; 32], mcause, mstatus: 0, mepc: 0 };
        ctx.gpr[17] = a7;
        ctx
    }

    #[test]
    fn decode_event_full_cause_matrix() {
        // 表驱动：锁定全部 mcause → Event 映射，避免遗漏 cause 静默落入 Error。
        let cases: &[(u32, u32, Event)] = &[
            // 中断（bit31=1）
            (1 << 31 | 7, 0, Event::IrqTimer),
            (1 << 31 | 3, 0, Event::IrqSoftware),
            (1 << 31 | 11, 0, Event::Error), // M_EI：rremu 无外部中断设备
            // ecall 变体
            (11, u32::MAX, Event::Yield),
            (11, EXIT_MAGIC, Event::Null),
            (11, 42, Event::Syscall),
            (8, 0, Event::Syscall), // ECALL_FROM_U：语义同 syscall
            // 异常
            (2, 0, Event::IllInsn),
            (3, 0, Event::Breakpoint),
            (0, 0, Event::Misalign),
            (4, 0, Event::Misalign),
            (6, 0, Event::Misalign),
            (5, 0, Event::LoadFault),
            (7, 0, Event::StoreFault),
            (1, 0, Event::Error), // INSTR_ACCESS_FAULT：取指访问异常
            (99, 0, Event::Error),
            (1 << 31 | 99, 0, Event::Error),
        ];
        for &(mcause, a7, want) in cases {
            assert_eq!(decode_event(&mk(mcause, a7)), want, "mcause={mcause:#x} a7={a7:#x}");
        }
    }
}
