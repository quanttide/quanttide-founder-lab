#!/usr/bin/env python3
"""
tests.py —— 「认知地图」推理工具的验证题目

分两部分：
  A. 断言题（assert）—— 用 pytest / 直接运行都可，验证引擎行为正确。
  B. 场景题（quiz）  —— 把「人问工具」的问句跑一遍，打印工具给出的答案，
                        用来直观展示这个工具能回答什么。

直接运行：python tests/tests.py
"""

from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "lib"))
from cognitive_map import CognitiveMap  # noqa: E402

MAP = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                   "cognitive-map.yaml")

cm = CognitiveMap.from_file(MAP)


# ---------------------------------------------------------------------------
# A. 断言题
# ---------------------------------------------------------------------------

def test_object_count():
    assert len(cm.objects) == 42, "对象应有 42 个（40 原始 + 2 新增）"


def test_relation_count():
    assert len(cm.relations) == 43, "关系应有 43 条（26 原始 + 17 新增）"


def test_kind_partition():
    k = cm.kinds()
    assert k == {"trigger": 8, "emotion": 6, "method": 15,
                 "expression": 3, "value": 7, "anti": 3}


def test_forward_query():
    # 范围失控 → 规模焦虑(evokes) 且 → 缩小窗口(responds_to)
    assert "规模焦虑" in cm.out_names("范围失控", "evokes")
    assert "缩小窗口" in cm.out_names("范围失控", "responds_to")


def test_backward_query():
    # 谁指向「缩小窗口」？范围失控 与 规模焦虑
    assert set(cm.in_names("缩小窗口", "responds_to")) == {"范围失控", "规模焦虑"}


def test_multi_out():
    # 规模焦虑有两条出边（缩小窗口 / 去建设平台）
    assert set(cm.out_names("规模焦虑", "responds_to")) == {"缩小窗口", "去建设平台"}


def test_chain_no_auto_multi_hop():
    # 链断在缺失的一环就停：战略失败 → 信心的底色 → 回到虚构世界 → 与自己保持连接 → 消耗型活动
    # 注：这是逐段展开，每段只走一跳；段间不自动寻路（若某段无目标则后续段为空）
    res = cm.chain_from("战略失败")
    assert res["legs"][0]["targets"] == ["信心的底色"]
    assert res["legs"][1]["targets"] == ["回到虚构世界"]


def test_chain_full_path():
    # 范围失控 -evokes-> 规模焦虑 -responds_to-> 缩小窗口 -serves-> 积累感 -excludes-> 消耗型活动
    res = cm.chain_from("范围失控")
    legs = {leg["kind"]: leg["targets"] for leg in res["legs"]}
    assert "规模焦虑" in legs["evokes"]
    assert set(legs["responds_to"]) == {"缩小窗口", "去建设平台"}
    assert "积累感" in legs["serves"]
    assert "消耗型活动" in legs["excludes"]


def test_validate_no_errors():
    # 该地图结构自洽：无悬空引用、无签名冲突
    errors = [d for d in cm.validate() if d.level == "error"]
    assert errors == [], f"不应有结构错误，却有：{errors}"


def test_orphans_resolved():
    # 2026-10-05 修复后：不再有孤立节点
    orphans = [d.message.split("`")[1] for d in cm.validate() if d.code == "ORPHAN"]
    assert orphans == [], f"孤立节点应已全部接线，却仍有：{orphans}"


def test_new_chain_from_conversation():
    # 本次对话蒸馏的新链：封闭 → 疏离 → 修剪上下文 → 与自己保持连接 → 消耗型活动
    res = cm.chain_from("疏离感的封闭")
    legs = {leg["kind"]: leg["targets"] for leg in res["legs"]}
    assert legs["evokes"] == ["疏离"]
    assert legs["responds_to"] == ["修剪上下文"]
    assert legs["serves"] == ["与自己保持连接"]
    assert legs["excludes"] == ["消耗型活动"]


def test_manifests_morphism():
    # 新增态射 manifests：expression → value
    assert "与自己保持连接" in cm.out_names("三本小说的同构倾向", "manifests")


def test_inference_provenance():
    # 每个新增关系都带 inference 标注，可与原档事实区分
    raw = __import__("yaml").safe_load(open(MAP, encoding="utf-8"))
    tagged = [r for r in raw["relations"] if r.get("inference")]
    assert len(tagged) == 17, f"应有 17 条带推断标注的新关系，实际 {len(tagged)}"
    assert all(r["inference"] in ("derived", "inferred") for r in tagged)


def test_dangling_detection():
    # 构造一个含悬空引用的地图，验证能被检出
    bad = CognitiveMap.from_dict({
        "objects": [{"name": "A", "kind": "trigger"}],
        "relations": [{"from": "A", "kind": "evokes", "to": "不存在的情绪"}],
    })
    codes = {d.code for d in bad.validate()}
    assert "DANGLING_DST" in codes


def test_signature_violation_detection():
    # 构造一条签名错误的边：trigger -serves-> value（serves 起点应为 method）
    bad = CognitiveMap.from_dict({
        "objects": [
            {"name": "T", "kind": "trigger"},
            {"name": "V", "kind": "value"},
        ],
        "relations": [{"from": "T", "kind": "serves", "to": "V"}],
    })
    codes = {d.code for d in bad.validate()}
    assert "SIGNATURE_SRC" in codes


def test_export_cytoscape():
    g = cm.to_cytoscape()
    assert len(g["nodes"]) == 42
    assert len(g["edges"]) == 43
    assert all("color" in n["data"] for n in g["nodes"])


# ---------------------------------------------------------------------------
# B. 场景题：把「人问工具」的问句列出来，直接打印答案
# ---------------------------------------------------------------------------

def quiz():
    print("\n" + "=" * 62)
    print("场景题：这些问句，工具分别怎么答")
    print("=" * 62)

    print("\nQ1. 「我范围失控时，会掉进什么情绪？该用什么方法？」")
    print("    工具答：情绪 =", "、".join(cm.out_names("范围失控", "evokes")),
          "；方法 =", "、".join(cm.out_names("范围失控", "responds_to")))

    print("\nQ2. 「缩小窗口这个方法，是被哪些情境召唤出来的？」（反向）")
    print("    工具答：", "、".join(cm.in_names("缩小窗口", "responds_to")))

    print("\nQ3. 「规模焦虑有哪几种应对方式？」")
    print("    工具答：", "、".join(cm.out_names("规模焦虑", "responds_to")))

    print("\nQ4. 「从『范围失控』这条线往下走，一直能走到哪个被排除的形态？」")
    res = cm.chain_from("范围失控")
    for leg in res["legs"]:
        print(f"    -{leg['kind']:12s}-> " + ("、".join(leg["targets"]) or "—"))

    print("\nQ5. 「哪条价值线规定了『个人化表达不能硬塞进组织流程』？」（反向归因）")
    print("    工具答：", "、".join(cm.in_names("个人化表达硬塞进组织流程", "excludes")))

    print("\nQ6. 「『刻意引导』这个方法的触发情境，只是情绪，还是也有触发点？」")
    src = cm.in_names("刻意引导", "responds_to")
    print("    工具答：", "、".join(src), "→",
          "、".join(cm.objects[s].kind for s in src))

    print("\nQ7. 「哪些方法是为『合规是刚性边界』服务的？」")
    print("    工具答：", "、".join(cm.in_names("合规是刚性边界", "serves")))

    print("\nQ8. 「地图里还有孤立、没接入关系的节点吗？」")
    orphans = [d.message.split("`")[1] for d in cm.validate() if d.code == "ORPHAN"]
    print("    工具答：", "、".join(orphans) if orphans else "无——2026-10-05 已全部接线")

    print("\nQ9. 「『信心的底色』这个情绪有对应的应对方法吗？」")
    print("    工具答：", "、".join(cm.out_names("信心的底色", "responds_to")) or "（无）")

    print("\nQ10. 「战略失败 和 消耗型活动 之间连通吗？给出路径。」")
    from collections import deque
    start, goal = "战略失败", "消耗型活动"
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
    if goal in prev:
        chain, node = [], goal
        while prev[node]:
            p, k = prev[node]
            chain.append(f"{p} -{k}-> {node}")
            node = p
        print("    工具答：✓ 连通 →", " ; ".join(reversed(chain)))
    else:
        print("    工具答：✗ 不连通（战略失败这条线止于『信心的底色』，未接入方法层）")


def run_all():
    tests = [(n, f) for n, f in sorted(globals().items())
             if n.startswith("test_") and callable(f)]
    passed = 0
    for name, fn in tests:
        try:
            fn()
            print(f"✓ {name}")
            passed += 1
        except AssertionError as e:
            print(f"✗ {name}  → {e}")
    print(f"\n断言题：{passed}/{len(tests)} 通过")
    quiz()


if __name__ == "__main__":
    run_all()
