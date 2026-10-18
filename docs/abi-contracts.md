# rremu-am ABI 与 Handler 契约

## 1. Context

```rust
#[repr(C)]
pub struct Context {
    pub gpr: [u32; 32], // offset 0..127
    pub mcause: u32,    // offset 128
    pub mstatus: u32,   // offset 132
    pub mepc: u32,      // offset 136
}
```

布局：

```text
总字段大小：35 * 4 = 140 字节
实际分配：144 字节，16 字节对齐
```

约束：

- MUST 保持 `repr(C)` 和字段顺序。
- `gpr[0]` 不使用；x0 恒 0。
- `gpr[2]` 在 trap 入口保存原始 `sp`；恢复时从该槽读回。
- `gpr[17]` 是 `a7`，用于 Event 解码。
- 汇编入口分配的 144 字节中，最后 4 字节为对齐填充。

## 2. SContext

```rust
#[repr(C)]
pub struct SContext {
    pub gpr: [u32; 32],
    pub scause: u32,
    pub sstatus: u32,
    pub sepc: u32,
}
```

与 `Context` 同布局，仅 trap CSR 不同。

## 3. TrapInfo

```rust
#[repr(C)]
pub struct TrapInfo {
    pub cause: u32, // offset 0
    pub pc: u32,    // offset 4
}
```

用途：

- 默认 handler 记录最近一次 trap。
- 调试器/测试通过固定地址读取。
- 写入必须使用 volatile，防止 LTO 删除。

## 4. Handler 类型

```rust
pub type Handler  = extern "C" fn(Event, *mut Context) -> *mut Context;
pub type SHandler = extern "C" fn(Event, *mut SContext) -> *mut SContext;
```

契约：

- 参数 `ctx` 指向 trap 保存的完整上下文。
- 返回值 MUST 是有效的 Context/SContext 指针。
- 返回不同指针可用于上下文切换。
- handler 不得返回已失效的栈上指针。
- handler 可以修改寄存器、mepc/sepc、mstatus/sstatus 以实现返回/切换。
- 默认 M handler 记录 TrapInfo 后 `halt(DEFAULT_TRAP_EXIT)`。
- 默认 S handler 记录 TrapInfo 后自旋。

## 5. 注册接口

```rust
trap_init(handler: Handler)
trap_init_s(handler: SHandler)
```

契约：

- `trap_init` 设置 M handler 并把 `mtvec` 指向 `__am_asm_trap`。
- `trap_init_s` 设置 S handler 并把 `stvec` 指向 `__am_asm_strap`。
- 注册状态是全局的；当前模型假设单 hart。
- MUST 在可能触发对应 trap 之前注册。
- 重复注册会覆盖旧 handler。
- handler 指针必须指向 `'static` 代码。

Target：

- 注册状态移入 `state.rs`，对外仍保留这两个 API。
- 增加显式单 hart 文档和安全条件。

## 6. 汇编入口

```text
__am_asm_trap
__am_asm_strap
```

约束：

- 使用 `.section .text.trap`。
- 分配 144 字节 Context/SContext。
- 保存 x1、x3-x31；x0 不保存。
- x5（t0）先保存，再作为 scratch。
- `gpr[2]` 保存 trap 前 `sp`。
- 调用 `__am_irq_handle` / `__am_s_irq_handle`。
- 使用返回的 Context 指针恢复现场。
- 恢复后执行 `mret` / `sret`。

任何布局变化都会破坏 ABI；修改前必须更新本文档和布局测试。

## 7. Event 映射

`decode_event` 使用：

```text
mcause/scause 的 bit31 判断中断
code = cause & 0x7fff_ffff
a7 = gpr[17]
```

当前映射：

| 条件 | Event |
|---|---|
| interrupt, code 7 或 5 | `IrqTimer` |
| interrupt, code 3 或 1 | `IrqSoftware` |
| interrupt, code 11 或 9 | `IrqExternal` |
| exception, code 11, a7=u32::MAX | `Yield` |
| exception, code 11, a7=EXIT_MAGIC | `Null` |
| exception, code 11 | `Syscall` |
| exception, code 8 | `Syscall` |
| exception, code 9 | `Syscall` |
| exception, code 2 | `IllInsn` |
| exception, code 3 | `Breakpoint` |
| exception, code 0/4/6 | `Misalign` |
| exception, code 5 | `LoadFault` |
| exception, code 7 | `StoreFault` |
| exception, code 1 | `Error` |
| 其他 | `Error` |

`Event::Null` 当前不会正常到达：CPU 在 `ecall + EXIT_MAGIC` 时直接宿主退出。

## 8. 退出协议

```rust
halt(code)
```

实现：

```text
M-mode ecall
a0 = code
a7 = EXIT_MAGIC = 0xCAFE_0000
```

契约：

- 只能从 M-mode 发起。
- CPU/Machine 看到该 ecall 后以 `a0` 作为宿主退出码。
- guest 不得依赖 ecall 返回。
- `DEFAULT_TRAP_EXIT = 0x10` 表示未处理 trap。

## 9. 启动参数

```text
a0 = hartid
a1 = DTB 物理地址（无 DTB 时为 0）
```

启动后由 `_start` 保存到 `BOOT_A0/BOOT_A1`，guest 通过：

```rust
boot_hartid() -> u32
boot_dtb() -> u32
```

读取。

约束：

- 必须在清 BSS 之前保存。
- 读取不能修改值。
- 当前单 hart；`boot_hartid` 返回启动 hart。

## 10. 权限与模式切换

### `enter_supervisor(entry, arg0, arg1)`

- 设置 `mstatus.MPP = S`
- 写 `mepc = entry`
- 写 `a0 = arg0`、`a1 = arg1`
- 执行 `mret`
- 不返回

### `pmp_allow_all()`

- 设置 PMP0 为覆盖 4GiB 的 NAPOT
- 允许 S/U R/W/X
- 仅用于测试/教学

契约：

- PMP 必须在进入 S/U 前配置。
- `enter_supervisor` 不负责 PMP 配置。

## 11. 兼容性

以下变更视为 ABI 破坏：

- Context/SContext/TrapInfo 字段顺序或大小
- 汇编保存/恢复的寄存器集合
- `EXIT_MAGIC`
- `DEFAULT_TRAP_EXIT`
- handler 签名
- boot a0/a1 约定
- mcause/scause 到 Event 的映射

修改这些内容必须：

1. 更新本文档
2. 更新布局测试
3. 更新 rremu-test 或明确暂缓
