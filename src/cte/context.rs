//! Trap 上下文与 ABI 布局。

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

/// S-mode 陷阱上下文：32 GPR + scause + sstatus + sepc。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SContext {
    pub gpr: [u32; 32],
    pub scause: u32,
    pub sstatus: u32,
    pub sepc: u32,
}
