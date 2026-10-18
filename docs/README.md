# rremu-am 核心文档

> 状态：核心审查阶段文档，尚未伴随代码重构。
>
> 范围：`trm`、`cte`、`ioe`、`arch`。
>
> 不包含：Linux、`platforms/`、第三层测试扩展。

## 文档索引

| 文档 | 内容 |
|---|---|
| [`architecture.md`](architecture.md) | TRM / CTE / IOE 分层与模块边界 |
| [`abi-contracts.md`](abi-contracts.md) | Context、SContext、TrapInfo、入口与 handler 契约 |
| [`state-ownership.md`](state-ownership.md) | 静态状态、单 hart 假设与安全条件 |
| [`../../CORE-REVIEW-AND-REFACTOR-PLAN.md`](../../CORE-REVIEW-AND-REFACTOR-PLAN.md) | 全项目规划 |

## 规范词

- **MUST**：ABI 或运行时必须满足。
- **MUST NOT**：禁止行为。
- **SHOULD**：推荐；偏离需说明。
- **MAY**：允许选择。

## 状态

```text
S 阶段前基线：9f2c0ec
当前分支基线：main
```

当前文档描述现有 ABI；标记 `Target` 的内容属于后续重构。
