# rremu-am 状态归属与安全条件

## 1. 当前全局状态

| 状态 | 位置 | 类型 | 写入者 | 读取者 |
|---|---|---|---|---|
| `DEFAULT_TRAP_INFO` | `state.rs` | `static mut TrapInfo` | 默认 M/S handler | 调试器、测试、外部观察 |
| `M_HANDLER` | `state.rs` | `static mut Option<Handler>` | `trap_init` | `__am_irq_handle` |
| `S_HANDLER` | `state.rs` | `static mut Option<SHandler>` | `trap_init_s` | `__am_s_irq_handle` |
| `BOOT_A0` | `state.rs` | `static mut u32` | `_start` | `boot_hartid` |
| `BOOT_A1` | `state.rs` | `static mut u32` | `_start` | `boot_dtb` |
| `EXIT_MAGIC` | `lib.rs` | `const u32` | 无 | halt / CPU 协议 |
| `DEFAULT_TRAP_EXIT` | `cte/context.rs` | `const i32` | 无 | 默认 handler |

## 2. 单 hart 假设

当前实现只支持单 hart：

- 一个 `HANDLER`
- 一个 `S_HANDLER`
- 一个 `DEFAULT_TRAP_INFO`
- 一组 boot 参数

因此：

- 所有静态状态都是全局单例。
- trap handler 不得并发运行。
- 不支持 SMP 或 per-hart handler。
- 当前不支持多 hart；多 hart 需要 per-hart 状态表。

## 3. 安全条件

### HANDLER / S_HANDLER

安全条件：

1. `trap_init*` 必须在对应 trap 可能发生前调用。
2. 写入后不得在运行期无同步地并发修改。
3. handler 必须具有 `'static` 生命周期。
4. 读取 `HANDLER` 后调用的函数必须在 trap 上下文中可重入安全。

当前行为：

```text
HANDLER 未设置 → default_handler
S_HANDLER 未设置 → default_s_handler
```

### DEFAULT_TRAP_INFO

安全条件：

1. 只由默认 handler 在 trap 上下文写入。
2. 读取者必须接受“可能正在写入”的并发风险；当前单 hart 下由 trap 暂停保证。
3. 写入 MUST 使用 volatile。
4. 不得用于普通控制流，只用于调试观测。

### BOOT_A0 / BOOT_A1

安全条件：

1. 只由 `_start` 在 BSS 清零后写入。
2. guest 运行后视为只读。
3. 读取使用 volatile，避免优化删除。
4. 不支持运行期修改启动参数。

## 4. 状态集中化

所有 `static mut` 只在 `state.rs` 中声明和访问；其它模块通过窄访问器读写：

```rust
pub(crate) fn install_m_handler(handler: Handler)
pub(crate) fn install_s_handler(handler: SHandler)
pub(crate) fn m_handler() -> Option<Handler>
pub(crate) fn s_handler() -> Option<SHandler>
pub(crate) fn write_default_trap_info(cause: u32, pc: u32)
pub(crate) fn read_boot_hartid() -> u32
pub(crate) fn read_boot_dtb() -> u32
```

## 5. 状态访问约束

- `trap_init*` 必须在对应 trap 可能发生前调用。
- handler 必须是 `'static` 函数指针。
- 默认 handler 在未注册时使用；注册后覆盖。
- `DEFAULT_TRAP_INFO` 只由默认 handler 写入，读取用于调试观测。
- `BOOT_A0/BOOT_A1` 只在 `_start` 清 BSS 后写入，运行期只读。
- 单 hart 模型下不引入锁；不提供 SMP 并发保证。

## 6. 实现约束

- 不改变 Context/SContext/TrapInfo 布局。
- 不改变链接器可见符号：
  - `BOOT_A0`
  - `BOOT_A1`
  - `DEFAULT_TRAP_INFO`
  - `__am_asm_trap`
  - `__am_asm_strap`
  - `__am_irq_handle`
  - `__am_s_irq_handle`
- 不改变默认 trap 退出码。
- 不引入锁；当前单 hart 不提供并发保证。
- 不增加新的全局状态。

## 7. 验证

验证项：

```text
默认 handler 记录 cause/pc
trap_init 覆盖默认 handler
trap_init_s 覆盖默认 S handler
boot_hartid/boot_dtb 返回启动值
enter_supervisor 正常进入 S
halt 退出码不变
Context/SContext/TrapInfo 大小和偏移不变
```
