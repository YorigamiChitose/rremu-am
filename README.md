# rremu-am

rremu 模拟器的 **RISC-V 抽象机器层（HAL）**，`no_std` `rlib` 库 crate。
对应 abstract-machine 的设计理念，为 guest 程序提供最小运行时、陷阱处理与设备抽象 API。
仅支持 RV32IMA + C（`riscv32imac-unknown-none-elf` 目标）。

本仓库为 Cargo workspace，包含两个成员：

| 成员 | 说明 |
|---|---|
| `rremu-am`（根，lib） | 上述 HAL 库（零依赖、`no_std`） |
| `rremu-report`（`report/`，bin） | host 侧工具：解析 rremu 的 dump JSON + ELF，输出可读崩溃报告 |

`rremu-report` 复用本库的 `exit_code_desc`（退出码语义）与 rremu 侧的
`rremu-isa`/`rremu-decode`（反汇编），把结构化 dump 翻译成人可读文本：

```sh
cd report && cargo run --release -- <dump.json> [--elf <path>]
```

## 分层

| 层 | 模块 | 内容 |
|---|---|---|
| TRM（图灵机） | `trm` | `_start` 启动（设栈、清 BSS、保存 a0/a1、调用 `rust_main`）、`halt(code)`、`putc`、`boot_hartid()` / `boot_dtb()`、`enter_supervisor(entry, a0, a1)` |
| IOE（I/O 扩展） | `ioe` | `uart_getc`、`clint_get_time` / `clint_set_timercmp` / `clint_set_msip` |
| CTE（陷阱/上下文） | `cte` | `Context`（M：32 GPR + mcause/mstatus/mepc）、`SContext`（S：scause/sstatus/sepc）、`Event`（11 变体）、`trap_init(handler)` / `trap_init_s(handler)`、`__am_asm_trap` / `__am_asm_strap`、默认 handler（`DEFAULT_TRAP_EXIT` / `DEFAULT_TRAP_INFO`） |

`arch` 模块承载地址映射常量（RAM / UART / CLINT / PLIC / FB）。它是 rremu 侧
`rremu-config` 默认值的**人工镜像**（本仓库为独立 `no_std` 仓库，不依赖它）；
两边一致性由 `rremu-test` 的护栏测试 `config_defaults_match_am_arch` 锁定——
任何地址漂移会在 CI 立即暴露。

## 使用约定

- guest 入口为 `#[unsafe(no_mangle)] pub extern "C" fn rust_main() -> !`
- 链接 `linker.ld`：RAM 基址 `0x8000_0000`，`_start` 位于 `.text.init` 段首
- **退出协议**：M-mode `ecall` 且 `a7 == EXIT_MAGIC (0xCAFE_0000)`，模拟器以 `a0` 为退出码终止（`halt(code)` 即此封装）
- **事件解码**：陷阱入口保存完整上下文后调用 `__am_irq_handle`，由 `decode_event` 从 `mcause` + `a7` 解码出 `Event` 交给用户 handler；handler 返回 `*mut Context` 支持上下文切换
- **默认错误处理**：`_start` 启动时默认安装 `mtvec` 指向陷阱入口。若 guest 未调 `trap_init`，陷阱由默认 handler 捕获——记录 `mcause`/`mepc` 到 `DEFAULT_TRAP_INFO` 后 `halt(DEFAULT_TRAP_EXIT = 0x10)`。guest 调 `trap_init(handler)` 可覆盖此行为
- **启动参数**：`_start` 在清 BSS 后保存 a0(hartid)/a1(dtb)，guest 可经
  `boot_hartid()` / `boot_dtb()` 读取
- **进入 S-mode**：`enter_supervisor(entry, a0, a1)` 设置 `mstatus.MPP=S`、`mepc`
  与参数后执行 `mret`，不返回
- **S-mode 陷阱**：`trap_init_s(handler)` 安装 `stvec` 指向 `__am_asm_strap`；
  handler 接收 `SContext`（`scause/sstatus/sepc`），返回后经 `sret` 恢复
- 非 RISC-V 目标下 API 退化为空实现（供 host 侧编译/单测）

## 测试

```sh
cargo test   # host 侧纯单元测试（Context 布局、Event 解码）
```
