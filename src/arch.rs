//! 架构常量与地址映射（与 rremu 模拟器约定严格对齐）。

/// RAM 基址。
pub const RAM_BASE: u32 = 0x8000_0000;
/// 默认 RAM 大小（16 MiB）。
pub const RAM_SIZE: u32 = 16 * 1024 * 1024;
/// 栈顶地址（RAM 末尾）。
pub const STACK_TOP: u32 = RAM_BASE + RAM_SIZE;

/// UART (NS16550A 子集) 基址。
pub const UART_BASE: u32 = 0x1000_0000;
/// CLINT 基址。
pub const CLINT_BASE: u32 = 0x0200_0000;
/// Framebuffer 基址。
pub const FB_BASE: u32 = 0x2000_0000;
/// Framebuffer 宽度。
pub const FB_WIDTH: u32 = 640;
/// Framebuffer 高度。
pub const FB_HEIGHT: u32 = 480;

/// UART 寄存器偏移。
pub mod uart {
    use super::UART_BASE;
    /// 发送 holding（写）/ 接收 data（读），offset 0。
    pub const THR: u32 = UART_BASE;
    /// 接收 data（读），offset 0（与 THR 同址）。
    pub const RDR: u32 = UART_BASE;
    /// 线状态寄存器，offset 5。
    pub const LSR: u32 = UART_BASE + 5;
    /// LSR Data Ready 位。
    pub const LSR_DR: u8 = 0x01;
}

/// CLINT 寄存器偏移。
pub mod clint {
    use super::CLINT_BASE;
    /// msip，offset 0x0。
    pub const MSIP: u32 = CLINT_BASE;
    /// mtimecmp，offset 0x4000。
    pub const MTIMECMP: u32 = CLINT_BASE + 0x4000;
    /// mtime，offset 0xBFF8。
    pub const MTIME: u32 = CLINT_BASE + 0xBFF8;
}
