# 澄清意图命令行

「澄清意图」的命令行与字段约定。实现落在 `apps/qtfounder` 的 `src/cli`；方向层（为什么做、定了什么、没定什么）在 quanttide-roadmap-of-founder 的 `work/clarify-intent.md`，本篇只讲怎么敲、敲完得到什么。逐参数的接口契约（入参、JSON 出入、错误码）见 `src/cli/docs/api-references/`。

两个入口：名词 `qtfounder work intent` 看意图，动词 `qtfounder work clarify` 澄清一次。

## 入口一（名词）：intent

| 调用 | 干什么 |
|------|--------|
| `qtfounder work intent` | 列出全部意图：任务一句话、澄清次数、重复 ≥2 次的差（规则候选） |
| `qtfounder work intent --id <id>` | 一件事的详情：它下面的全部澄清记录与提示词 |

## 入口二（动词）：clarify

| 调用 | 干什么 |
|------|--------|
| `qtfounder work clarify` | 澄清一次：问「原以为它知道什么 / 它实际做了什么」，给一段可粘回对话的话，挂到最近一件事上 |
| `qtfounder work clarify --task "<一句话>"` | 新开一件事再澄清 |
| `qtfounder work clarify --stdin` | 模型用：整条 JSON 走 stdin，字段名与输出一致 |
| `qtfounder work clarify --rule "<文本>"` | 把重复出现的差固化成一条规则，追加进 `rules.md` |

阶段不是命令。开始前、过程中、结束时都敲同一句 `qtfounder work clarify`，工具按「有没有可挂的事」决定新开还是续记。

## 字段

intent（一件事）：`task`（一句话，要干什么）、`at`（自动记）。

澄清记录（挂在 intent 下）：`before`（我原以为它知道 / 会做什么）、`actual`（它实际做了什么，也是归类键）、`prompt`（可直接粘回对话的一段话）、`at`（自动记）。

没有第五个字段——撑起全部功能的就是这几个，不加。

## 用法

```bash
# 人用：进流程，逐问作答，挂到最近一件事上
qtfounder work clarify

# 新开一件事
qtfounder work clarify --task "给这段代码写单元测试"

# 模型用：一次给全
echo '{"task":"给这段代码写单元测试","before":"它知道项目用 pytest、fixture 在 conftest","actual":"给了五段解释加伪代码","prompt":"刚才你的回答偏在铺陈展开……"}' \
  | qtfounder work clarify --stdin

# 看有哪些事、哪些差重复了
qtfounder work intent
qtfounder work intent --id int_01J9

# 立规则
qtfounder work clarify --rule "先结论后细节，除非我明确要展开"
```

`--json` 给机器：字段只增不删，错误也走 JSON、写 stderr。重复提交用 sha256(before + actual) 去重，返回已有记录并标 `deduplicated`，退出码 0——模型重试安全。

退出码三档：0 成功（含去重）、2 用法错、3 未找到（`--id` 对应的事不存在）。

## HTTP

名词入口对应读，动词入口对应写：

| 方法 | 路径 | 对哪个入口 |
|------|------|-----------|
| GET | `/work/intents` | `intent` |
| GET | `/work/intents/{id}` | `intent --id` |
| POST | `/work/clarify` | `clarify` |
| POST | `/work/clarify/rules` | `clarify --rule` |

## 落地顺序

1. 先定 HTTP 契约（上面四个路径）；
2. server + SQLite 起一版，跑通「澄清一次 → 看意图 → 候选浮出 → 立规则」一条链；
3. CLI 做薄客户端，`--json` 直接透传；
4. skill 里写死 `clarify` 这一条命令，跑两周，看两个数：prompt 粘回率、规则点头率。

第一版不做：向量检索（先用关键词打分）、多用户、同步、MCP 适配。

## 存储

初版按 server 侧 SQLite 单库设计，intent 与澄清记录两张表 + `rules.md`。数据落点是否改为第二大脑 `data/context/<集>/intention/` 的 Markdown，见 roadmap 仓那份的「待决策」。
