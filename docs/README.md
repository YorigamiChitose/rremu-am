# rremu-am 核心文档

本目录描述 rremu-am 的分层、ABI 和静态状态归属。

## 文档索引

| 文档 | 内容 |
|---|---|
| [`architecture.md`](architecture.md) | TRM / CTE / IOE 分层与模块边界 |
| [`abi-contracts.md`](abi-contracts.md) | Context、SContext、TrapInfo、入口与 handler 契约 |
| [`state-ownership.md`](state-ownership.md) | 静态状态、单 hart 假设与安全条件 |

## 规范词

- **MUST**：ABI 或运行时必须满足。
- **MUST NOT**：禁止行为。
- **SHOULD**：推荐；偏离需说明。
- **MAY**：允许选择。
