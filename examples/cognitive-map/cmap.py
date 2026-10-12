#!/usr/bin/env python3
"""
cmap —— 「认知地图」命令行推理工具

用法示例：
    cmap stats                                  # 概览统计
    cmap kinds                                  # 各类对象数量
    cmap out 范围失控                            # 正向：范围失控指向谁
    cmap out 范围失控 --kind responds_to         # 正向（限定态射）
    cmap in  缩小窗口                            # 反向：谁指向缩小窗口
    cmap trace 范围失控 responds_to              # 单跳推理
    cmap chain 战略失败                          # 触发点→情绪→方法→价值 逐段展开
    cmap validate                               # 完整性校验
    cmap check "缩小窗口"                        # 校验一个名字是否存在于地图
    cmap path 战略失败 积累感                    # 找一条连接两对象的路径
    cmap export --format cytoscape -o web/graph.json
    cmap export --format json -o map.json
    cmap list --kind method                     # 按类型列出对象
"""

from __future__ import annotations

import argparse
import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "lib"))
from cognitive_map import CognitiveMap, MORPHISM_SIGNATURE  # noqa: E402

DEFAULT_MAP = os.path.join(os.path.dirname(os.path.abspath(__file__)), "cognitive-map.yaml")

KIND_CN = {
    "trigger": "触发点", "emotion": "情绪", "method": "方法",
    "expression": "表达", "value": "价值", "anti": "排除形态",
}
MORPHISM_CN = {
    "evokes": "触发", "responds_to": "应对", "serves": "服务",
    "excludes": "排除", "manifests": "外显",
}


def _load(args) -> CognitiveMap:
    return CognitiveMap.from_file(args.map)


def cmd_stats(cm: CognitiveMap, args):
    print(f"《{cm.meta.get('title', '认知地图')}》  schema={cm.meta.get('schema', '?')}")
    print(f"  对象总数 : {len(cm.objects)}")
    print(f"  关系总数 : {len(cm.relations)}")
    print("  按类型   :", "  ".join(f"{KIND_CN.get(k, k)}={v}" for k, v in cm.kinds().items()))


def cmd_kinds(cm: CognitiveMap, args):
    for k, v in cm.kinds().items():
        print(f"{k:12s} {KIND_CN.get(k, '')}  ×{v}")


def cmd_list(cm: CognitiveMap, args):
    kind = args.kind
    for o in cm.objects.values():
        if kind and o.kind != kind:
            continue
        extra = f"  [{o.group}]" if o.group else ""
        note = f"  ({o.note})" if o.note else ""
        print(f"· {o.name}  <{KIND_CN.get(o.kind, o.kind)}>{extra}{note}")


def cmd_out(cm: CognitiveMap, args):
    rels = cm.out(args.name, args.kind)
    if not rels:
        print(f"`{args.name}` 没有匹配的出边。")
        return
    for r in rels:
        print(f"{r.src}  -{r.kind}->  {r.dst}   [{KIND_CN.get(cm.objects[r.dst].kind, '?')}]")


def cmd_in(cm: CognitiveMap, args):
    rels = cm.in_(args.name, args.kind)
    if not rels:
        print(f"没有指向 `{args.name}` 的入边。")
        return
    for r in rels:
        print(f"{r.src}  -{r.kind}->  {r.dst}")


def cmd_trace(cm: CognitiveMap, args):
    res = cm.trace(args.name, args.kind)
    print(f"① 起点 : {res['from']}  <{KIND_CN.get(cm.objects[res['from']].kind, '?')}>")
    print(f"② 态射 : {res['kind']}（{MORPHISM_CN.get(res['kind'], '')}）  签名 {res['signature']}")
    if not res["targets"]:
        print("③ 结果 : （无）")
        return
    print("③ 结果 :")
    for t in res["targets"]:
        print(f"     └─ {t['to']}  <{KIND_CN.get(t['kind'], t['kind'])}>")


def cmd_chain(cm: CognitiveMap, args):
    res = cm.chain_from(args.name)
    print(f"认知链展开（起点：{res['start']}）")
    for leg in res["legs"]:
        arrow = f"-{leg['kind']}->"
        if leg["targets"]:
            print(f"  {arrow:16s} " + "、".join(leg["targets"]))
        else:
            print(f"  {arrow:16s} —")


def cmd_validate(cm: CognitiveMap, args):
    diags = cm.validate()
    errors = [d for d in diags if d.level == "error"]
    warns = [d for d in diags if d.level == "warning"]
    if not diags:
        print("✓ 校验通过：无悬空引用、无重复定义、态射签名一致。")
        return
    for d in errors + warns:
        tag = "✗ 错误" if d.level == "error" else "! 提示"
        where = f"  ({d.where})" if d.where else ""
        print(f"{tag} [{d.code}] {d.message}{where}")
    print(f"\n合计：{len(errors)} 个错误，{len(warns)} 个提示。")


def cmd_check(cm: CognitiveMap, args):
    name = args.name
    if name in cm.objects:
        o = cm.objects[name]
        print(f"✓ `{name}` 存在于地图中：<{KIND_CN.get(o.kind, o.kind)}>  源文件 {o.file}")
        outs = cm.out(name)
        ins = cm.in_(name)
        print(f"  出边 {len(outs)} 条，入边 {len(ins)} 条")
    else:
        print(f"✗ `{name}` 不在 objects 中。")
        # 给出近似候选
        cand = [n for n in cm.objects if name[:2] in n or n[:2] in name]
        if cand:
            print("  是否想找：" + "、".join(cand[:5]))


def cmd_path(cm: CognitiveMap, args):
    """BFS 找一条从 start 到 goal 的路径（不限定态射顺序，仅用于连通性排查）。"""
    from collections import deque
    start, goal = args.start, args.goal
    if start not in cm.objects or goal not in cm.objects:
        print("起点或终点不在地图中。")
        return
    prev = {start: None}
    q = deque([start])
    while q:
        cur = q.popleft()
        if cur == goal:
            break
        for r in cm.out(cur):
            if r.dst not in prev:
                prev[r.dst] = (cur, r.kind)
                q.append(r.dst)
    if goal not in prev:
        print(f"✗ 从 `{start}` 无法到达 `{goal}`（在忽略态射方向约束下也不连通）。")
        return
    chain = []
    node = goal
    while prev[node] is not None:
        p, k = prev[node]
        chain.append((p, k, node))
        node = p
    chain.reverse()
    print(f"✓ 找到路径（{len(chain)} 跳）：")
    for p, k, n in chain:
        print(f"  {p}  -{k}->  {n}")


def cmd_export(cm: CognitiveMap, args):
    data = cm.to_cytoscape() if args.format == "cytoscape" else cm.to_dict()
    out = json.dumps(data, ensure_ascii=False, indent=2)
    if args.output:
        with open(args.output, "w", encoding="utf-8") as fh:
            fh.write(out)
        print(f"已导出 {args.format} → {args.output}")
    else:
        print(out)


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="cmap", description="「认知地图」形式化推理工具")
    p.add_argument("--map", default=DEFAULT_MAP, help="认知地图 YAML 路径")
    sub = p.add_subparsers(dest="cmd", required=True)

    sub.add_parser("stats", help="概览统计").set_defaults(func=cmd_stats)
    sub.add_parser("kinds", help="按类型统计").set_defaults(func=cmd_kinds)
    sub.add_parser("validate", help="完整性校验").set_defaults(func=cmd_validate)

    sp = sub.add_parser("list", help="列出对象")
    sp.add_argument("--kind", choices=list(KIND_CN), help="限定类型")
    sp.set_defaults(func=cmd_list)

    sp = sub.add_parser("out", help="正向查询：X 指向谁")
    sp.add_argument("name")
    sp.add_argument("--kind", choices=list(MORPHISM_SIGNATURE))
    sp.set_defaults(func=cmd_out)

    sp = sub.add_parser("in", help="反向查询：谁指向 X")
    sp.add_argument("name")
    sp.add_argument("--kind", choices=list(MORPHISM_SIGNATURE))
    sp.set_defaults(func=cmd_in)

    sp = sub.add_parser("trace", help="单跳推理")
    sp.add_argument("name")
    sp.add_argument("kind", choices=list(MORPHISM_SIGNATURE))
    sp.set_defaults(func=cmd_trace)

    sp = sub.add_parser("chain", help="从触发点逐段展开认知链")
    sp.add_argument("name")
    sp.set_defaults(func=cmd_chain)

    sp = sub.add_parser("check", help="校验某名字是否存在")
    sp.add_argument("name")
    sp.set_defaults(func=cmd_check)

    sp = sub.add_parser("path", help="找一条连通路径")
    sp.add_argument("start")
    sp.add_argument("goal")
    sp.set_defaults(func=cmd_path)

    sp = sub.add_parser("export", help="导出 JSON")
    sp.add_argument("--format", choices=["cytoscape", "json"], default="cytoscape")
    sp.add_argument("-o", "--output")
    sp.set_defaults(func=cmd_export)

    return p


def main(argv=None):
    args = build_parser().parse_args(argv)
    cm = _load(args)
    args.func(cm, args)


if __name__ == "__main__":
    main()
