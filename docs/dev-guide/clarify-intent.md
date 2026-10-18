# 澄清意图命令行

「澄清意图」的命令行与字段约定。正式实现落在 `apps/qtfounder` 的 `src/cli`，命令 `qtfounder`；实验室版在 lab 仓 `src/cli/`，命令 `qtfounder-lab`，本篇与接口参考都按实验室版写。实现状态与关键设计在 quanttide-report-of-founder 的 `work/clarify-intent.md`。逐参数的接口契约（入参、JSON 出入、错误码）见 `src/cli/docs/api-references/`。

两个入口：名词 `qtfounder-lab code intent` 看意图，动词 `qtfounder-lab code clarify` 澄清一次。

## 入口一（名词）：intent

| 调用 | 干什么 |
|------|--------|
| `qtfounder-lab code intent` | 列出全部意图：任务一句话、澄清次数、重复 ≥2 次的差（规则候选） |
| `qtfounder-lab code intent --id <id>` | 一件事的详情：它下面的全部澄清记录与提示词 |

## 入口二（动词）：clarify

| 调用 | 干什么 |
|------|--------|
| `qtfounder-lab code clarify` | 澄清一次：问「原以为它知道什么 / 它实际做了什么」，给一段可粘回对话的话，挂到最近一件事上 |
| `qtfounder-lab code clarify --task "<一句话>"` | 新开一件事再澄清 |
| `qtfounder-lab code clarify --stdin` | 模型用：整条 JSON 走 stdin，字段名与输出一致 |
| `qtfounder-lab code clarify --revise <clr_id>` | 修正一条已记录的澄清：原地改两问、重拼提示词，不新增记录 |
| `qtfounder-lab code clarify --rule "<文本>"` | 把重复出现的差固化成一条规则，追加进 `rules.md` |

阶段不是命令。开始前、过程中、结束时都敲同一句 `qtfounder-lab code clarify`，工具按「有没有可挂的事」决定新开还是续记。

## 字段

intent（一件事）：`task`（一句话，要干什么）、`at`（自动记）。

澄清记录（挂在 intent 下）：`before`（我原以为它知道 / 会做什么）、`actual`（它实际做了什么，也是归类键）、`prompt`（可直接粘回对话的一段话）、`at`（自动记）。

没有第五个字段——撑起全部功能的就是这几个，不加。

## 用法

```bash
# 人用：进流程，逐问作答，挂到最近一件事上
qtfounder-lab code clarify

# 新开一件事
qtfounder-lab code clarify --task "给这段代码写单元测试"

# 模型用：一次给全
echo '{"task":"给这段代码写单元测试","before":"它知道项目用 pytest、fixture 在 conftest","actual":"给了五段解释加伪代码","prompt":"刚才你的回答偏在铺陈展开……"}' \
  | qtfounder-lab code clarify --stdin

# 看有哪些事、哪些差重复了
qtfounder-lab code intent
qtfounder-lab code intent --id int_01J9

# 立规则
qtfounder-lab code clarify --rule "先结论后细节，除非我明确要展开"
```

`--json` 给机器：字段只增不删，错误也走 JSON、写 stderr。重复提交用 sha256(before + actual) 去重，返回已有记录并标 `deduplicated`，退出码 0——模型重试安全。

退出码三档：0 成功（含去重）、2 用法错、3 未找到（`--id` 对应的事不存在）。

## HTTP

名词入口对应读，动词入口对应写：

| 方法 | 路径 | 对哪个入口 |
|------|------|-----------|
| GET | `/code/intents` | `intent` |
| GET | `/code/intents/{id}` | `intent --id` |
| POST | `/code/clarify` | `clarify` |
| POST | `/code/clarify/rules` | `clarify --rule` |

## 实现落点（实验室版，已落地）

| 文件 | 职责 |
|------|------|
| `src/cli/src/main.rs` | clap derive 定义（`work <intent\|clarify>`）、错误 JSON、退出码三档 |
| `src/cli/src/store.rs` | 单文件 JSON 存储、id/时间生成、归一化与去重键、rules.md 追加 |
| `src/cli/src/code.rs` | 两个入口的行为：列表/详情、交互两问、--stdin、去重、立规则 |
| `src/cli/tests/code.rs` | 集成测试（assert_cmd + tempfile），覆盖表见 `src/cli/REPORT.md` |

门禁（只对本 crate，全绿才算完）：

```bash
cargo fmt --check
cargo clippy --all-targets --offline -- -D warnings
cargo test --offline
```

实际输出样式：

```text
$ qtfounder-lab code intent
int_1SAZ  给这段代码写单元测试  3 次澄清
  候选：先讲了两大段背景才给答案  ×2

$ qtfounder-lab code clarify          # 交互：逐行问两问，模板提示词当场生成
你原本以为它知道 / 会做什么？
> <你的答案>
它实际做了什么？
> <你的答案>

我原以为「…」。
它实际做了「…」。
请按我原以为的重做一遍，把这两条当约束。

已记入 int_1SAZ / clr_VTAV

$ qtfounder-lab code clarify --rule "先结论后细节，除非我明确要展开"
已追加规则到 data/rules.md
- 先结论后细节，除非我明确要展开
已处理 2 条重复差：clr_VTAV, clr_9S2V
```

## 落地顺序

1. 先定 HTTP 契约（上面四个路径）；
2. server + SQLite 起一版，跑通「澄清一次 → 看意图 → 候选浮出 → 立规则」一条链（CLI 版已用单文件 JSON 跑通同一条链，不等 server）；
3. CLI 做薄客户端，`--json` 直接透传；
4. skill 里写死 `clarify` 这一条命令，跑两周，看两个数：prompt 粘回率、规则点头率。

第一版不做：向量检索（先用关键词打分）、多用户、同步、MCP 适配。

## 存储

当前实现（实验室版 CLI）：单文件 JSON，两个文件——`clarify.json` 是账本、`rules.md` 是规则。数据目录按构建形态选：开发构建（`cargo run` / `cargo test`）落代码包内的 `data/`，开发者直接看；发布构建遵循 XDG，落 `$XDG_DATA_HOME/qtfounder-lab`，未设则 `~/.local/share/qtfounder-lab`；`QTFOUNDER_LAB_DATA` 优先级最高，测试与临时切换用它。不引 SQLite——离线环境没有 bundled sqlite，单文件 JSON 够这两个入口用；`src/cli/.gitignore` 已忽略 `data/`。

后续 server 侧的两表 + SQLite 方案保持原设计；数据落点是否改为第二大脑 `data/context/<集>/intention/` 的 Markdown，见 `quanttide-report-of-founder` 的 `code/clarify-intent.md`。
