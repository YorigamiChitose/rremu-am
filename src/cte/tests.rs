use crate::EXIT_MAGIC;
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
    fn s_context_layout() {
        let ctx = SContext { gpr: [0; 32], scause: 0, sstatus: 0, sepc: 0 };
        let base = &ctx as *const _ as *const u8;
        assert_eq!(&ctx.gpr[0] as *const _ as usize - base as usize, 0);
        assert_eq!(&ctx.gpr[17] as *const _ as usize - base as usize, 68);
        assert_eq!(&ctx.gpr[31] as *const _ as usize - base as usize, 124);
        assert_eq!(&ctx.scause as *const _ as usize - base as usize, 128);
        assert_eq!(&ctx.sstatus as *const _ as usize - base as usize, 132);
        assert_eq!(&ctx.sepc as *const _ as usize - base as usize, 136);
        assert_eq!(std::mem::size_of::<SContext>(), 140);
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
            (1 << 31 | 5, 0, Event::IrqTimer),
            (1 << 31 | 3, 0, Event::IrqSoftware),
            (1 << 31 | 1, 0, Event::IrqSoftware),
            (1 << 31 | 11, 0, Event::IrqExternal),
            (1 << 31 | 9, 0, Event::IrqExternal),
            // ecall 变体
            (11, u32::MAX, Event::Yield),
            (11, EXIT_MAGIC, Event::Null),
            (11, 42, Event::Syscall),
            (8, 0, Event::Syscall), // ECALL_FROM_U：语义同 syscall
            (9, 0, Event::Syscall), // ECALL_FROM_S：语义同 syscall
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

    #[test]
    fn decode_event_s_uses_scause() {
        let mut ctx = SContext { gpr: [0; 32], scause: 9, sstatus: 0, sepc: 0 };
        ctx.gpr[17] = 0;
        assert_eq!(decode_event_s(&ctx), Event::Syscall);
        ctx.scause = 2;
        assert_eq!(decode_event_s(&ctx), Event::IllInsn);
        ctx.scause = 1 << 31 | 5;
        assert_eq!(decode_event_s(&ctx), Event::IrqTimer);
        ctx.scause = 1 << 31 | 9;
        assert_eq!(decode_event_s(&ctx), Event::IrqExternal);
    }
