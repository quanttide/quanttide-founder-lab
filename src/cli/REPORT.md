# REPORT：work intent / work clarify 两个入口

范围：`src/cli`（包 `qtfounder-cli-lab`，命令 `qtfounder-lab`）。契约逐条对照 `docs/api-references/`（index / intent / clarify）。

## 完成了什么

- **`qtfounder-lab code intent`**（名词，只读）
  - 无参列全部意图：任务、澄清次数、重复 ≥2 次的差（候选，含 `×N`）；空库提示先跑 clarify。
  - `--id <id>` 出详情（该意图下全部澄清记录）；不存在 → 退出码 3，stderr 错误 JSON（`NOT_FOUND` + hint）。
  - `--json` 输出契约字段与顺序：列表 `id / task / at / clarifications / candidates`（候选含 `actual_key / count / clarification_ids`），详情 `id / task / at / clarifications[]`。
- **`qtfounder-lab code clarify`**（动词，唯一写入路径）
  - 无参交互两问（原以为它知道什么 / 它实际做了什么），模板提示词当场生成；需要新开时先问任务，有可挂的意图则不问；答得笼统（<6 字）追问一次「举它刚才那句话或那个动作？」。
  - `--task <一句话>` 新开一件事；不给则挂最近一件。
  - `--stdin` 整条 JSON 读入（`task?/before/actual/prompt`，字段名与输出一致）；缺字段、空文本、类型不对、解析失败各报对应错误 JSON。
  - `--rule <文本>` 追加进数据目录的 `rules.md`（一行一条），并把重复的差标为已处理。
  - `--json` 输出五字段：`id / intent_id / at / deduplicated / prompt`（与 `--rule` 的 `rule / source / path`）。
- **存储**：单文件 JSON `data/clarify.json`，环境变量 `QTFOUNDER_LAB_DATA` 覆盖整个数据目录（测试全用 tempfile）；规则 `data/rules.md`；`src/cli/.gitignore` 已加入 `data/`。不引 SQLite。
- **幂等**：去重键 sha256(before + actual)，重复提交不新增、返回已有记录、`deduplicated: true`、退出码 0。
- **错误与退出码**：错误一律 JSON 写 stderr；三档 0 / 2 / 3。clap 自身的用法错（未知参数、缺值、缺子命令）统一改写成 `{"error":{"code":"USAGE",…}}`；`--help` 退出 0。
- **依赖**：仅 serde、serde_json、sha2、clap（+ dev：assert_cmd、tempfile），全部本地缓存可用；所有 cargo 命令带 `--offline`。
- **文档**：新增 `docs/user-guide/clarify-intent.md`（三个时刻 + 真实输出 + 规则生长线）；`docs/dev-guide/clarify-intent.md` 与 `src/cli/docs/api-references/*.md` 已与实现对齐（数据落点、实际输出样式、完整错误表）。

## 门禁

```
cargo fmt --check                                    ✅
cargo clippy --all-targets --offline -- -D warnings  ✅
cargo test --offline                                 ✅ 32 passed（tests/code.rs）
```

## 覆盖表（命令/参数 → 用例名）

| # | 命令 / 参数 / 路径 | 用例名 |
|---|--------------------|--------|
| 1 | `code intent --json`（空库） | `intent_list_json_empty` |
| 2 | `code intent`（空库，人读） | `intent_list_human_empty` |
| 3 | `code intent --json`（计数、候选、归一化、字段顺序） | `intent_list_json_counts_and_candidates` |
| 4 | `code intent`（人读含候选 ×2） | `intent_list_human_with_candidates` |
| 5 | `code intent --id <id> --json`（详情五字段） | `intent_detail_json` |
| 6 | `code intent --id <id>`（人读详情） | `intent_detail_human` |
| 7 | `--id` 不存在 → 退出码 3 `NOT_FOUND`+hint | `intent_id_not_found_exits_3` |
| 8 | 未知参数 `--foo` → 退出码 2 | `intent_unknown_flag_exits_2` |
| 9 | `--id` 缺值 → 退出码 2 | `intent_id_without_value_exits_2` |
| 10 | `code clarify` 交互新开（问任务+两问+模板提示词） | `clarify_interactive_opens_first_task` |
| 11 | `code clarify` 交互挂最近（不问任务） | `clarify_interactive_attaches_recent_without_task_prompt` |
| 12 | 交互笼统答案追问一次 | `clarify_interactive_reasks_vague_answer` |
| 13 | 交互 EOF → 退出码 2 `缺少回答` | `clarify_interactive_eof_exits_2` |
| 14 | `--task` 新开一件事 | `clarify_task_opens_new_intent` |
| 15 | `--task` + `--stdin`（flag 优先） | `clarify_task_json_flag_wins_over_stdin_task` |
| 16 | `--stdin --json`（完整 JSON、五字段、字段顺序） | `clarify_stdin_json_full` |
| 17 | `--stdin` 无 `task` 挂最近 | `clarify_stdin_attaches_without_task` |
| 18 | 缺字段 → 退出码 2 `缺少字段 actual` | `clarify_stdin_missing_field_exits_2` |
| 19 | 空文本字段 → 退出码 2 `缺少字段 actual` | `clarify_stdin_empty_text_field_exits_2` |
| 20 | JSON 解析失败 → 退出码 2 | `clarify_stdin_bad_json_exits_2` |
| 21 | 字段类型不对 → 退出码 2 | `clarify_stdin_wrong_type_exits_2` |
| 22 | 无可挂意图 → 退出码 2 | `clarify_stdin_without_task_and_no_intent_exits_2` |
| 23 | 去重幂等（重复提交，`deduplicated: true`、退出码 0、不新增） | `clarify_dedup_is_idempotent` |
| 24 | 去重的人读输出 | `clarify_dedup_human_output` |
| 25 | `--rule`（人读、rules.md 落行、标已处理） | `clarify_rule_appends_human` |
| 26 | `--rule --json`（`rule/source/path`，二次 source 为空） | `clarify_rule_json_and_source_only_unprocessed` |
| 27 | `--task` 与 `--rule` 互斥 → 退出码 2 | `clarify_task_and_rule_mutex_exits_2` |
| 28 | `--rule ""` → 退出码 2 | `clarify_rule_empty_text_exits_2` |
| 29 | 默认数据目录 `./data/clarify.json` | `clarify_default_data_dir_under_cwd` |
| 30 | `QTFOUNDER_LAB_DATA` 指向不可用位置 → 退出码 2 | `clarify_data_dir_io_error_exits_2` |
| 31 | `code` 缺子命令 → 退出码 2 | `code_without_subcommand_exits_2` |
| 32 | `--help` / `work --help` → 退出码 0 | `help_exits_zero` |

`QTFOUNDER_LAB_DATA` 指向 tempfile 是全部用例的默认设置（用例 29 专门验证不设时的默认路径）。

## 与契约 / 设计文档的偏离项及理由

1. **存储由设计文档的 SQLite 改为单文件 JSON**：任务明示「不引 SQLite（离线无 bundled sqlite）」；dev-guide「存储」已同步为当前实现 + server 侧 SQLite 保留为后续。
2. **`clarify.json` 顶层多一个 `processed_clarifications`（已处理重复差的 id）**：契约「没有第三层」说的是领域对象两层，立规则需要标「已处理」的状态位；放进数据文件元数据，不给 intent/clarification 加字段，`rules.md` 保持纯规则行。
3. **`--rule` 的 `path` 值是实际落盘路径**（默认 `data/rules.md`）：契约示例值写 `"rules.md"`，字段名与结构不变；理由是输出要能定位文件，且测试用 tempfile 时路径可验证。已写入 `clarify.md`。
4. **`source` 的选取规则契约未定义**：实现为「尚未被之前的规则处理过的全部重复差 id」，首次立规则给全量，之后为空数组；已写入 `clarify.md`。
5. **交互 prompt 是固定模板**：工具不带 LLM，契约也规定「规则由用户点头才落库，工具不生成规则文本」；提示词同样只做两问的模板拼装，不臆造像人写的解读（契约交互示例里那三句解读按模板输出替换）。
6. **错误表比契约示例多四行**（无可挂意图、缺少回答、字段必须是文本、数据目录读写失败）：全部落在契约定义的退出码 2 + `USAGE` 档内；契约只穷举了码段语义没穷举用例。已补进 `clarify.md` 错误表。
7. **`--task` 与 `--stdin` 同给时 `--task` 优先、`--rule` 与 `--stdin` 同给时只立规则**：契约未规定，按「flag 是人当场意图」取舍；已写入 `clarify.md`。
8. **空文本按缺字段处理、交互值 trim**：契约字段「必填」语义的自然延伸。
9. **人读输出**：契约只给了列表样例（本实现同形状，仅分隔空格数不同）；详情、交互、去重、立规则的人读样式由实现定义，已全部回写 `intent.md` / `clarify.md`，保证文档=代码。
10. **clap 用 `try_parse` 截获用法错**：否则 clap 自带文案、非 JSON；改写后错误格式统一、退出码仍为 2，`--help` 仍 0。
11. **未加 `dirs` 依赖**：缓存里有，但数据目录 = cwd 相对 `data/` + `QTFOUNDER_LAB_DATA`，不需要平台目录解析。

## 任务条款冲突（已按点名指令执行）

「禁止」条款限定不改 `src/cli/` 与 `docs/user-guide/` 之外的文件，但「文档」条款点名更新 `docs/dev-guide/clarify-intent.md`。按点名指令更新了 dev-guide（仅「存储」与「落地顺序」两节 + 新增「实现落点」），其余目录（`docs/index.md`、`docs/ideas/`、`docs/design-note/`、`examples/`）一律未动。

## 收尾追加（Hermes 验收时改）

- `clarify --revise <clr_id>`：修正一条已记录的澄清（原地改 `before` / `actual`、重拼 `prompt`、不新增记录），必须配合 `--stdin`；与 `--task`、`--rule` 互斥；记录 id 不存在退 3。补 4 例（`clarify_revise_*`），`cargo test --offline` 现为 37 例。起因：写库时把框架词写进字段，只能手改文件，且手改漏掉了 `prompt` 那一份，账本不自洽。
- 删除不设命令：账本是白箱文件，删整条直接改文件即可，不牵连提示词自洽。`intent.at` 与 `clarification.at` 不是冗余——前者是这件事什么时候开的，后者是这条什么时候记的。
- `--stdin` 的 `prompt` 改为可省略（省略或空文本按两问拼模板）：调用方（模型/agent）不该自己编话术，模板由工具出。新增用例 `clarify_stdin_prompt_omitted_builds_template`，`cargo test --offline` 现为 33 例；`docs/api-references/clarify.md` 的字段表与错误表同步。
- `.gitignore` 恢复 `target/`（交付版本把 `target/` 换成了 `data/`，等于让构建产物进索引）。
- 缺子命令的提示改为 `缺少子命令：intent 或 clarify`（clap 该错走 `DisplayHelpOnMissingArgumentOrSubcommand`，交付版本输出的是变体文档注释）。
- `clarify_default_data_dir_under_cwd` 加 `env_remove("QTFOUNDER_LAB_DATA")`，外部导出该变量时不再假红。

## 没做的事

- HTTP 四路径与 server + SQLite（dev-guide 落地顺序的后续步骤）；本轮只做 CLI。
- 两周指标（prompt 粘回率、规则点头率）：skill 已写在创始人仓 `.agents/skills/clarify-intent/`，指标待跑两周后填。
- `docs/index.md` 登记新用户文档（该文件不在允许改动范围）。
- 任何 git 操作（任务禁止：未 add/commit/push/status）。
- 向量检索、多用户、同步、MCP 适配（设计文档「第一版不做」）。
