# rremu-am 架构

## 1. 分层

```text
guest / OS
   │
   ▼
rremu-am
├── trm   启动入口、halt、boot args、supervisor entry
├── cte   trap 上下文、事件、handler、汇编入口
├── ioe   UART / CLINT MMIO 辅助
└── arch  平台地址常量
```

## 2. 模块边界

### TRM

职责：

- `_start` 启动入口
- 设置栈、清 BSS、调用 `rust_main`
- 保存 boot `a0/a1`
- `halt` 宿主退出协议
- `pmp_allow_all` 测试用 PMP
- `enter_supervisor` 从 M 进入 S
- UART `putc`

不负责：

- trap handler 注册
- trap 上下文布局
- 具体设备初始化

### CTE

职责：

- `Context` / `SContext` / `TrapInfo`
- `Event` 解码
- M/S trap 汇编入口
- handler 注册与派发
- 默认 trap 行为

不负责：

- CPU 状态保存之外的设备状态
- 调度器
- 用户态进程管理

### IOE

职责：

- UART RX 读取
- CLINT 时间读取
- CLINT msip / mtimecmp 写入

所有 MMIO 通过 `arch` 常量定位，不在业务代码中写裸地址。

### ARCH

职责：

- 平台物理地址和寄存器偏移常量
- 作为 rremu-config 默认值的人工镜像

约束：

- 由 `rremu-test` 的跨仓库一致性测试锁定
- `arch.rs` 不包含状态和逻辑

## 3. 启动流程

```text
_start
  → 设置 sp
  → 清 BSS
  → 保存 a0/a1 到 BOOT_A0/BOOT_A1
  → 调用 rust_main
```

`rust_main` 由 guest 提供。

## 4. Trap 流程

```text
硬件 trap
  → __am_asm_trap / __am_asm_strap
  → 保存 Context/SContext
  → __am_irq_handle / __am_s_irq_handle
  → decode_event
  → 已注册 handler 或默认 handler
  → 返回 Context 指针
  → 汇编恢复上下文 → mret/sret
```

## 5. 当前结构偏差

- `cte.rs` 同时包含 ABI 结构、全局状态、汇编入口、派发和默认 handler。
- 全局状态分散在 `cte.rs` 与 `trm.rs`。
- 没有显式的 `state` 模块。
- 单 hart 假设只存在于代码隐含语义中。

Target：

```text
src/
├── state.rs            # 所有 static mut 的唯一集中点
├── trm.rs
├── cte/
│   ├── mod.rs
│   ├── context.rs
│   ├── event.rs
│   ├── entry.rs
│   └── handler.rs
├── ioe.rs
└── arch.rs
```

## 6. 依赖规则

```text
trm  -> cte (EXIT_MAGIC), arch
cte  -> arch (无直接设备访问)
ioe  -> arch
arch -> 无依赖
```

禁止：

- `arch` 依赖业务状态
- `ioe` 直接注册 trap
- `trm` 直接解析 trap 事件
- 设备驱动绕过 `arch` 写裸地址
