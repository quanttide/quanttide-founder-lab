# clarify（动词入口）

澄清一次。唯一的写入路径：记录一次「不对劲」，并当场给出一段能粘回对话的话。

```text
qtfounder-lab product clarify [--task <一句话>] [--stdin] [--rule <文本>] [--json]
```

| 参数 | 必填 | 说明 |
|------|------|------|
| `--task <一句话>` | 否 | 新开一件事；不给则挂到最近一次需求上 |
| `--stdin` | 否 | 整条 JSON 从标准输入读入，字段名与 `--json` 输出一致 |
| `--revise <clr_id>` | 否 | 修正一条已记录的澄清：原地改 `before` / `actual` 并重拼提示词，不新增记录 |
| `--rule <文本>` | 否 | 把候选固化成一条规则，追加进 `rules.md` |
| `--json` | 否 | 输出机器可读 JSON |

阶段不是命令。开始前、过程中、结束时都敲同一句 `product clarify`：工具有可挂的需求就续记，没有就当场新开并问任务。用户不需要知道阶段名。

## 澄清一次

**交互（人用）**：两问由工具逐行问，`>` 后是你敲的答案；模板提示词由两问当场生成（答得笼统会先追问一句「举它刚才那句话或那个动作？」）：

```text
$ qtfounder-lab product clarify
任务：给这段代码写单元测试            ← 仅在需要新开时问（给了 --task 则不问）
你原本以为它知道 / 会做什么？
> 项目用 pytest，fixture 在 conftest
它实际做了什么？
> 给了五段解释加伪代码

我原以为「项目用 pytest，fixture 在 conftest」。
它实际做了「给了五段解释加伪代码」。
请按我原以为的重做一遍，把这两条当约束。

已记入 req_01J9 / clr_01JB
```

重复提交同一对 before/actual 时不去重新记录，人读输出改为 `已存在 req_01J9 / clr_01JA（重复提交，未新增记录）`。交互中任一问读不到回答（EOF 或空行）报 `缺少回答`，退出码 2。

**`--stdin`（模型用）**：整条记录一次给全，字段名与输出一致。

```bash
echo '{"task":"给这段代码写单元测试","before":"它知道项目用 pytest、fixture 在 conftest","actual":"给了五段解释加伪代码","prompt":"刚才你的回答偏在铺陈展开，我需要的是能直接跑的代码……"}' \
  | qtfounder-lab product clarify --stdin
```

| 输入字段 | 必填 | 说明 |
|------|------|------|
| `task` | 否 | 省略则挂到最近一次需求；不给且没有可挂的需求时报错 |
| `before` | 是 | 我原以为它知道 / 会做什么 |
| `actual` | 是 | 它实际做了什么（归类键） |
| `prompt` | 否 | 可直接粘回对话的一段话；省略或空文本时按两问拼模板 |

字段缺失或空文本都报 `缺少字段 <名>`；值不是文本报 `字段 <名> 必须是文本`；`--task` 与 JSON 里的 `task` 同时给时以 `--task` 为准。

**`--json` 输出**：

```json
{
  "id": "clr_01JB",
  "requirement_id": "req_01J9",
  "at": "2026-10-18T09:12:03Z",
  "deduplicated": false,
  "prompt": "刚才你的回答偏在铺陈展开，我需要的是能直接跑的代码……"
}
```

`deduplicated` 为 true 表示这条之前就记过（去重键 sha256(before + actual)），本次没有新增记录，退出码仍为 0。没给 `--json` 时人读输出是模板提示词 + `已记入 <requirement_id> / <id>`（去重时为 `已存在 …（重复提交，未新增记录）`）。

`--stdin` 前没有需求且 JSON 不含 `task` 时报 `没有可挂的需求，缺少 task`，退出码 2。

## 修正一条记录

记歪了——字段里带了「我以为」这类框架词，或者写错了一行——用 `--revise <clr_id>` 原地改，不新增记录：

```bash
qtfounder-lab product clarify --revise clr_NHCX --stdin --json <<'JSON'
{"before":"只改我指定的那一段","actual":"整段重写"}
JSON
```

必须配合 `--stdin`。提示词按新的两问重拼，`id`、`at`、所属需求都不变；`--revise` 与 `--task`、`--rule` 互斥——修正不改归属，也不立规则。

**输出**：

```json
{
  "id": "clr_NHCX",
  "requirement_id": "req_1Z8G",
  "at": "2026-10-18T08:39:56Z",
  "prompt": "我原以为「只改我指定的那一段」。\n它实际做了「整段重写」。\n请按我原以为的重做一遍，把这两条当约束。"
}
```

删除不设命令：账本是白箱文件，删整条直接改文件就行，不会牵连提示词自洽（提示词跟着记录走）。

## 立规则

```bash
qtfounder-lab product clarify --rule "先结论后细节，除非我明确要展开"
```

规则由用户点头才落库，工具不生成规则文本。命令把这条规则以 `- <文本>` 一行追加进数据目录里的 `rules.md`，并把重复的差标为已处理（状态记在数据文件里，不加领域字段）。与 `--stdin` 同时给时只立规则，不读 stdin。

**人读输出**：

```text
已追加规则到 data/rules.md
- 先结论后细节，除非我明确要展开
已处理 2 条重复差：clr_01J9, clr_01JA
```

**`--json` 输出**：

```json
{
  "rule": "先结论后细节，除非我明确要展开",
  "source": ["clr_01J9", "clr_01JA"],
  "path": "data/rules.md"
}
```

| 字段 | 说明 |
|------|------|
| `source` | 本次处理的重复差（尚未被之前的规则处理过的澄清 id）；都处理过后再跑为空数组 |
| `path` | `rules.md` 的实际落盘路径（默认 `data/rules.md`，随 `QTFOUNDER_LAB_DATA` 变） |

## 错误

| 情况 | 退出码 | 输出 |
|------|--------|------|
| 缺必填字段（`before` / `actual`，空文本同） | 2 | `{"error":{"code":"USAGE","message":"缺少字段 actual"}}` |
| 字段值不是文本 | 2 | `{"error":{"code":"USAGE","message":"字段 actual 必须是文本"}}` |
| `--task` 与 `--rule` 同时给 | 2 | `{"error":{"code":"USAGE","message":"--task 与 --rule 不能同时使用"}}` |
| `--stdin` 的 JSON 解析失败 | 2 | `{"error":{"code":"USAGE","message":"JSON 解析失败：第 1 行"}}` |
| `--rule` 文本为空 | 2 | `{"error":{"code":"USAGE","message":"--rule 需要一段文本"}}` |
| `--revise` 未配合 `--stdin` | 2 | `{"error":{"code":"USAGE","message":"--revise 需要配合 --stdin 给新的 before / actual"}}` |
| `--revise` 与 `--task` 或 `--rule` 同给 | 2 | `{"error":{"code":"USAGE","message":"--task 与 --revise 不能同时使用"}}` |
| `--revise` 的记录 id 不存在 | 3 | `{"error":{"code":"NOT_FOUND","message":"clr_XXXX 不存在","hint":"用 qtfounder-lab product requirement --id <需求 id> 查看"}}` |
| 无 `--task` 且没有可挂的需求 | 2 | `{"error":{"code":"USAGE","message":"没有可挂的需求，缺少 task"}}` |
| 交互中读不到回答（EOF / 空行） | 2 | `{"error":{"code":"USAGE","message":"缺少回答"}}` |
| 数据目录读写失败（`QTFOUNDER_LAB_DATA` 指向不可写位置等） | 2 | `{"error":{"code":"USAGE","message":"无法读取 <路径>：…"}}` |

退出码三档与幂等约定见 [index.md](index.md)。
