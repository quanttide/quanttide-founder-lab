# clarify（动词入口）

澄清一次。唯一的写入路径：记录一次「不对劲」，并当场给出一段能粘回对话的话。

```text
qtfounder work clarify [--task <一句话>] [--stdin] [--rule <文本>] [--json]
```

| 参数 | 必填 | 说明 |
|------|------|------|
| `--task <一句话>` | 否 | 新开一件事；不给则挂到最近一次意图上 |
| `--stdin` | 否 | 整条 JSON 从标准输入读入，字段名与 `--json` 输出一致 |
| `--rule <文本>` | 否 | 把候选固化成一条规则，追加进 `rules.md` |
| `--json` | 否 | 输出机器可读 JSON |

阶段不是命令。开始前、过程中、结束时都敲同一句 `work clarify`：工具有可挂的意图就续记，没有就当场新开并问任务。用户不需要知道阶段名。

## 澄清一次

**交互（人用）**：

```text
$ qtfounder work clarify
任务：给这段代码写单元测试            ← 仅在需要新开时问
你原本以为它知道 / 会做什么？
> 项目用 pytest，fixture 在 conftest
它实际做了什么？
> 给了五段解释加伪代码

刚才你的回答偏在铺陈展开，我需要的是能直接跑的代码。
差异在于我默认了「结论优先、细节按需」，你按「全面覆盖」来写。
接下来请先给三行以内的结论，细节我追问再展开。

已记入 int_01J9 / clr_01JB
```

两问必须落到具体行为。答得笼统（「它就是不行」）时追问一次「举它刚才那句话或那个动作」——概括是情绪，不是信息。

**`--stdin`（模型用）**：整条记录一次给全，字段名与输出一致。

```bash
echo '{"task":"给这段代码写单元测试","before":"它知道项目用 pytest、fixture 在 conftest","actual":"给了五段解释加伪代码","prompt":"刚才你的回答偏在铺陈展开，我需要的是能直接跑的代码……"}' \
  | qtfounder work clarify --stdin
```

| 输入字段 | 必填 | 说明 |
|------|------|------|
| `task` | 否 | 省略则挂到最近一次意图 |
| `before` | 是 | 我原以为它知道 / 会做什么 |
| `actual` | 是 | 它实际做了什么（归类键） |
| `prompt` | 是 | 可直接粘回对话的一段话 |

**`--json` 输出**：

```json
{
  "id": "clr_01JB",
  "intent_id": "int_01J9",
  "at": "2026-10-18T09:12:03Z",
  "deduplicated": false,
  "prompt": "刚才你的回答偏在铺陈展开，我需要的是能直接跑的代码……"
}
```

`deduplicated` 为 true 表示这条之前就记过（去重键 sha256(before + actual)），本次没有新增记录，退出码仍为 0。

## 立规则

```bash
qtfounder work clarify --rule "先结论后细节，除非我明确要展开"
```

规则由用户点头才落库，工具不生成规则文本。命令把这条规则追加进 `rules.md`，并把重复的差标为已处理。

**`--json` 输出**：

```json
{
  "rule": "先结论后细节，除非我明确要展开",
  "source": ["clr_01J9", "clr_01JA"],
  "path": "rules.md"
}
```

## 错误

| 情况 | 退出码 | 输出 |
|------|--------|------|
| 缺必填字段（`before` / `actual` / `prompt`） | 2 | `{"error":{"code":"USAGE","message":"缺少字段 actual"}}` |
| `--task` 与 `--rule` 同时给 | 2 | `{"error":{"code":"USAGE","message":"--task 与 --rule 不能同时使用"}}` |
| `--stdin` 的 JSON 解析失败 | 2 | `{"error":{"code":"USAGE","message":"JSON 解析失败：第 1 行"}}` |
| `--rule` 文本为空 | 2 | `{"error":{"code":"USAGE","message":"--rule 需要一段文本"}}` |

退出码三档与幂等约定见 [index.md](index.md)。
