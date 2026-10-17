#![no_std]
//! rremu-am：RISC-V 抽象机器层（HAL）。
//!
//! 对应 abstract-machine，提供 no_std RISC-V 运行时、陷阱处理与设备抽象 API。
//! 仅支持 RV32IMA + C（riscv32imac-unknown-none-elf 目标）。

pub mod arch;
pub mod cte;
pub mod ioe;
pub mod trm;

pub use cte::{Context, Event, Handler, SContext, SHandler, trap_init, trap_init_s, DEFAULT_TRAP_EXIT, DEFAULT_TRAP_INFO};
pub use ioe::{clint_get_time, clint_set_msip, clint_set_timercmp, uart_getc};
pub use trm::{boot_dtb, boot_hartid, enter_supervisor, halt, pmp_allow_all, putc};

/// 宿主退出协议魔术数：M-mode ecall 且 a7 == EXIT_MAGIC 时模拟器以 a0 为退出码终止。
/// 与 rremu-cpu 的 EXIT_MAGIC 保持一致。
pub const EXIT_MAGIC: u32 = 0xCAFE_0000;

/// 已知退出码的语义描述（供离线工具 rremu-report / 调试者使用）。
///
/// 未识别的 code 返回 `None`（可能由 guest 自定义）。
pub fn exit_code_desc(code: i32) -> Option<&'static str> {
    match code {
        0 => Some("success"),
        1 => Some("generic failure"),
        DEFAULT_TRAP_EXIT => Some("unhandled trap (default handler)"),
        _ => None,
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_desc_known_codes() {
        assert_eq!(exit_code_desc(0), Some("success"));
        assert_eq!(exit_code_desc(1), Some("generic failure"));
        assert_eq!(exit_code_desc(DEFAULT_TRAP_EXIT), Some("unhandled trap (default handler)"));
        assert_eq!(exit_code_desc(42), None);
    }
}
