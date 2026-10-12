# profile-index 推理工具（cmap）

把《认知地图》——「对象 + 态射」形式化的 profile 索引——当作一张**有向多重图**来查询、校验与可视化。

## 文件结构

```
profile-index/
├── cognitive-map.yaml    # 认知地图数据（42 对象 / 43 关系，schema: profile-index/1）
├── cmap.py               # CLI 入口
├── lib/cognitive_map.py  # 推理引擎（可 import 复用）
├── tests/tests.py        # 验证题目：16 道断言题 + 10 道场景题
└── web/index.html        # 交互式可视化（数据已内联，离线可用）
```

## 快速开始

```bash
python3 cmap.py stats                 # 概览：40 对象 / 26 关系
python3 cmap.py out 范围失控           # 正向查询
python3 cmap.py in  缩小窗口           # 反向查询
python3 cmap.py chain 战略失败         # 认知链逐段展开
python3 cmap.py validate              # 完整性校验
python3 tests/tests.py                # 跑全部验证题目
# 可视化：直接用浏览器打开 web/index.html
```

## 态射签名（结构规则）

引擎按原文件的类型约束编码了每种态射的合法端点，违者报错：

| 态射 | 语义 | 起点 kind | 终点 kind |
|---|---|---|---|
| `evokes` | 触发点 → 情绪 | trigger | emotion |
| `responds_to` | 情境 → 方法 | trigger / emotion | method |
| `serves` | 方法 → 价值 | method | value |
| `excludes` | 价值 → 排除的形态 | value | anti |
| `manifests` | 表达 → 价值（本次扩展） | expression | value |

> `manifests` 为 2026-10-05 新增：索引中 expression 这一类此前**没有任何出边**，
> 任何"表达服务于某价值"的接线都会被签名校验拦下。为其引入独立态射，
> 而非把 expression 硬塞进 method 的 `serves`，以保持各 kind 语义清晰。

## 修订记录

### 2026-10-05 「封闭 / 疏离 / 修剪上下文」专题

| 动作 | 内容 |
|---|---|
| 新增对象 | `疏离感的封闭`(trigger)、`与自己保持连接`(value) |
| 补断链 | `修剪上下文 -serves→ 与自己保持连接`（此前 serves 为空） |
| 补断头情绪 | `信心的底色 -responds_to→ 回到虚构世界` |
| 修复孤立节点 | 10 → 0：methods / expressions / values 各按签名内合法接法接入 |
| 新增态射 | `manifests`（expression → value） |
| 关系数 | 26 → 43 |
| 校验 | `cmap validate`：0 错误 / 0 孤立节点 |

**推断可追溯**：所有新增关系均带 `inference: inferred` 与 `note` 说明依据，
可与原档事实（无该字段者）区分。

## 验证题目（节选自 tests/tests.py 的场景题）

| # | 问题 | 工具的回答 |
|---|---|---|
| Q1 | 范围失控时，会掉进什么情绪？该用什么方法？ | 情绪 = 规模焦虑；方法 = 缩小窗口 |
| Q2 | 「缩小窗口」被哪些情境召唤？（反向） | 范围失控、规模焦虑 |
| Q3 | 规模焦虑有哪几种应对？ | 缩小窗口、去建设平台 |
| Q4 | 从「范围失控」往下走，能走到哪个被排除的形态？ | 规模焦虑 → 缩小窗口/去建设平台 → 积累感 → 消耗型活动 |
| Q5 | 哪条价值线规定「个人化表达不能硬塞进组织流程」？ | 标准化才配走公司流程 |
| Q6 | 「刻意引导」的触发情境是情绪还是触发点？ | 两者都有：AI 变成纯粹执行器(trigger)、失控与重新掌控(emotion) |
| Q7 | 哪些方法服务于「合规是刚性边界」？ | 找合规来套 |
| Q8 | 地图里哪些节点是孤立的？ | 共 10 个（如 三本小说的同构倾向、利用人性恶赚钱不能长久…） |
| Q9 | 「信心的底色」有应对方法吗？ | 无——这是地图的一处缺口 |
| Q10 | 「战略失败」与「消耗型活动」连通吗？ | 不连通：该线止于「信心的底色」，未接入方法层 |

断言题另覆盖：数量与类型分布校验、多出边节点、悬空引用检测（构造坏图验证 `DANGLING_DST`）、
签名违例检测（构造 `trigger -serves-> value` 验证 `SIGNATURE_SRC`）、Cytoscape 导出完整性。

## validate 能查出什么

- `DANGLING_SRC / DANGLING_DST`：关系端点未在 objects 中声明
- `DUP_OBJECT`：对象重复定义
- `BAD_MORPHISM`：未知态射类型
- `SIGNATURE_SRC / SIGNATURE_DST`：端点类型违反态射签名
- `ORPHAN`（warning）：孤立节点——本地图检出 10 个，即原文件中「尚未接入认知网络」的条目

## 设计边界（最小可用）

- 只做**单跳**查询与逐段展开，不做自动多跳寻路（`chain` 每段之间不自动跳转）；
- `path` 仅为连通性排查用的 BFS，不作为语义推理；
- 「什么时候归 profile」的判据不在本工具职责内——它只回答**结构与连接**层面的问题。

## API 速览

```python
from lib.cognitive_map import CognitiveMap
cm = CognitiveMap.from_file("cognitive-map.yaml")
cm.out_names("范围失控", "responds_to")   # ['缩小窗口']
cm.in_names("缩小窗口", "responds_to")    # ['范围失控', '规模焦虑']
cm.chain_from("范围失控")                 # {'legs': [{kind: evokes, ...}, ...]}
cm.validate()                            # -> list[Diagnostic]
cm.to_cytoscape()                        # -> 网页用的 nodes/edges
```
