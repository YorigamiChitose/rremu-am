#![no_std]
//! rremu-am：RISC-V 抽象机器层（HAL）。
//!
//! 对应 abstract-machine，提供 no_std RISC-V 运行时、陷阱处理与设备抽象 API。
//! 仅支持 RV32IMA + C（riscv32imac-unknown-none-elf 目标）。

pub mod arch;
pub mod cte;
pub mod ioe;
pub mod trm;

pub use cte::{Context, Event, Handler, trap_init};
pub use ioe::{clint_get_time, clint_set_msip, clint_set_timercmp, uart_getc};
pub use trm::{halt, putc};

/// 宿主退出协议魔术数：M-mode ecall 且 a7 == EXIT_MAGIC 时模拟器以 a0 为退出码终止。
/// 与 rremu-cpu 的 EXIT_MAGIC 保持一致。
pub const EXIT_MAGIC: u32 = 0xCAFE_0000;

#[cfg(test)]
extern crate std;
