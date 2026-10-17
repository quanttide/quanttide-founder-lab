# ROADMAP

量潮创始人实验室 CLI（qtlab-cli）。

## 目标

把实验室中反复出现的实验操作固化为命令，减少手工步骤。

## 当前状态

脚手架：入口与结构保留，暂无子命令。新功能接入：`src/` 下新增模块 → `CliCommand` 注册 → 依赖加回 `Cargo.toml`。

## 候选功能

| 功能 | 说明 |
|------|------|
| `segments` | 重建日志段清单，复用 `examples/core/segmentation.py` |
| `emotion` | 情绪日记联想检索，复用 `examples/core/retrieval.py` |
| `audit` | 跑各案例的回归测试并汇总结果 |
