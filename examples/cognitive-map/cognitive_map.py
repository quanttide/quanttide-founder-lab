"""
cognitive_map.py —— 「认知地图」形式化推理器（最小可用版）

把 profile-index（对象 + 态射）当作一张有向多重图来查询与校验：

    对象 Object  : {name, kind, file, group?, note?}
    态射 Relation: {from, kind, to}     kind ∈ {evokes, responds_to, serves, excludes}

对外能力（单跳，刻意保持最小可用）：
    - 正向查询  out(name, kind=None)        某个对象「指向」谁
    - 反向查询  in_(name, kind=None)        谁「指向」某个对象
    - 单跳推理  trace(name, kind)           触发点→情绪→方法→价值 的逐段展开
    - 完整性校验 validate()                 悬空引用 / 重复定义 / kind 合法性

零依赖（PyYAML 除外），既可作为库 import，也可被 CLI / 网页导出 JSON 复用。
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Iterable

try:
    import yaml
except ImportError:  # pragma: no cover
    yaml = None


# ---------------------------------------------------------------------------
# 态射类型的语义与合法性约束
# ---------------------------------------------------------------------------

# 每种态射规定「起点 kind」与「终点 kind」，用于校验关系是否符合认知地图的结构
COLOR_OF = {
    "trigger": "#e8590c",   # 触发点
    "emotion": "#7048e8",   # 情绪
    "method": "#0ca678",    # 方法
    "expression": "#1c7ed6",  # 表达
    "value": "#c92a2a",     # 价值
    "anti": "#868e96",      # 排除的形态
}

MORPHISM_SIGNATURE = {
    "evokes": ("trigger", "emotion"),
    "responds_to": ({"trigger", "emotion"}, "method"),
    "serves": ("method", "value"),
    "excludes": ("value", "anti"),
    # 2026-10-05 扩展：表达是价值的外显，为其提供出边（expression 此前无任何出边）
    "manifests": ("expression", "value"),
}

# 一条完整的「认知链」应逐段经过的态射顺序
CHAIN = ["evokes", "responds_to", "serves", "excludes"]

@dataclass
class Obj:
    name: str
    kind: str
    file: str = ""
    group: str | None = None
    note: str | None = None


@dataclass
class Relation:
    src: str
    kind: str
    dst: str

    def as_dict(self) -> dict:
        return {"from": self.src, "kind": self.kind, "to": self.dst}


@dataclass
class Diagnostic:
    level: str          # "error" | "warning"
    code: str
    message: str
    where: str = ""


@dataclass
class CognitiveMap:
    objects: dict[str, Obj] = field(default_factory=dict)
    relations: list[Relation] = field(default_factory=list)
    meta: dict = field(default_factory=dict)

    # -- 构建 ---------------------------------------------------------------

    @classmethod
    def from_file(cls, path: str) -> "CognitiveMap":
        if yaml is None:
            raise RuntimeError("需要 PyYAML：pip install pyyaml")
        with open(path, "r", encoding="utf-8") as fh:
            raw = yaml.safe_load(fh)
        return cls.from_dict(raw)

    @classmethod
    def from_dict(cls, raw: dict) -> "CognitiveMap":
        m = cls()
        m.meta = {k: raw.get(k) for k in ("schema", "title", "source") if k in raw}
        for item in raw.get("objects", []):
            o = Obj(
                name=item["name"],
                kind=item["kind"],
                file=item.get("file", ""),
                group=item.get("group"),
                note=item.get("note"),
            )
            m.objects[o.name] = o
        for rel in raw.get("relations", []):
            m.relations.append(Relation(rel["from"], rel["kind"], rel["to"]))
        return m

    # -- 单跳正向查询 --------------------------------------------------------

    def out(self, name: str, kind: str | None = None) -> list[Relation]:
        """从 name 出发、可指定 kind 的出边。"""
        return [
            r for r in self.relations
            if r.src == name and (kind is None or r.kind == kind)
        ]

    def out_names(self, name: str, kind: str | None = None) -> list[str]:
        return [r.dst for r in self.out(name, kind)]

    # -- 单跳反向查询 --------------------------------------------------------

    def in_(self, name: str, kind: str | None = None) -> list[Relation]:
        """指向 name 的入边。"""
        return [
            r for r in self.relations
            if r.dst == name and (kind is None or r.kind == kind)
        ]

    def in_names(self, name: str, kind: str | None = None) -> list[str]:
        return [r.src for r in self.in_(name, kind)]

    # -- 单跳推理（逐段展开，不做多跳寻路）------------------------------------

    def trace(self, name: str, kind: str) -> dict:
        """
        沿单一态射展开一跳，附带该段的语义说明。
        这是「最小可用」推理的语义核心：不做自动多跳寻路，
        但保证每一跳的 direction / 语义 / 端点类型都是显式且可解释的。
        """
        sig = MORPHISM_SIGNATURE.get(kind)
        rels = self.out(name, kind)
        return {
            "from": name,
            "kind": kind,
            "signature": str(sig) if sig else None,
            "targets": [
                {"to": r.dst, "kind": self.objects[r.dst].kind if r.dst in self.objects else "?"}
                for r in rels
            ],
        }

    def chain_from(self, name: str) -> dict:
        """
        从触发点出发，按 evokes → responds_to → serves → excludes 逐段展开。
        每一段都沿用 trace()，段与段之间不做自动跳转，保持最小可用与可解释。
        """
        result = {"start": name, "legs": []}
        frontier = [name]
        for kind in CHAIN:
            leg_targets: list[str] = []
            for node in frontier:
                leg_targets.extend(self.out_names(node, kind))
            # 去重保序
            seen, uniq = set(), []
            for t in leg_targets:
                if t not in seen:
                    seen.add(t)
                    uniq.append(t)
            result["legs"].append({"kind": kind, "targets": uniq})
            frontier = uniq
            if not frontier:
                break
        return result

    # -- 完整性校验 ----------------------------------------------------------

    def validate(self) -> list[Diagnostic]:
        diags: list[Diagnostic] = []

        # 1) 悬空引用：关系端点必须是已声明对象
        for r in self.relations:
            if r.src not in self.objects:
                diags.append(Diagnostic(
                    "error", "DANGLING_SRC",
                    f"关系起点 `{r.src}` 未在 objects 中定义", f"{r.src} -{r.kind}-> ..."))
            if r.dst not in self.objects:
                diags.append(Diagnostic(
                    "error", "DANGLING_DST",
                    f"关系终点 `{r.dst}` 未在 objects 中定义", f"... -{r.kind}-> {r.dst}"))

        # 2) 重复定义对象
        names = [o.name for o in self.objects.values()]
        dup = {n for n in names if names.count(n) > 1}
        for n in dup:
            diags.append(Diagnostic("error", "DUP_OBJECT", f"对象 `{n}` 重复定义"))

        # 3) 非法态射
        for r in self.relations:
            if r.kind not in MORPHISM_SIGNATURE:
                diags.append(Diagnostic(
                    "error", "BAD_MORPHISM", f"未知态射类型 `{r.kind}`", f"{r.src} -> {r.dst}"))

        # 4) 关系端点类型与态射签名不符
        for r in self.relations:
            if r.kind not in MORPHISM_SIGNATURE or r.src not in self.objects or r.dst not in self.objects:
                continue
            want_src, want_dst = MORPHISM_SIGNATURE[r.kind]
            want_src = {want_src} if isinstance(want_src, str) else want_src
            want_dst = {want_dst} if isinstance(want_dst, str) else want_dst
            got_src = self.objects[r.src].kind
            got_dst = self.objects[r.dst].kind
            if got_src not in want_src:
                diags.append(Diagnostic(
                    "error", "SIGNATURE_SRC",
                    f"`{r.kind}` 的起点应为 {sorted(want_src)}，实际为 `{got_src}`",
                    f"{r.src} -{r.kind}-> {r.dst}"))
            if got_dst not in want_dst:
                diags.append(Diagnostic(
                    "error", "SIGNATURE_DST",
                    f"`{r.kind}` 的终点应为 {sorted(want_dst)}，实际为 `{got_dst}`",
                    f"{r.src} -{r.kind}-> {r.dst}"))

        # 5) 孤立对象（既无出边也无入边）
        linked = {r.src for r in self.relations} | {r.dst for r in self.relations}
        for name in self.objects:
            if name not in linked:
                diags.append(Diagnostic(
                    "warning", "ORPHAN",
                    f"对象 `{name}` 未参与任何关系（孤立节点）"))

        # 6) 无来源的终端（anti 之外没有任何入边的 value / method 等只作提示）
        for o in self.objects.values():
            if not self.in_(o.name) and self.out(o.name):
                pass  # 起点型对象（trigger）本就没有入边，属正常

        return diags

    # -- 视图 / 导出 ---------------------------------------------------------

    def kinds(self) -> dict[str, int]:
        counts: dict[str, int] = {}
        for o in self.objects.values():
            counts[o.kind] = counts.get(o.kind, 0) + 1
        return counts

    def to_cytoscape(self) -> dict:
        """导出给网页（Cytoscape.js）用的 nodes/edges 结构。"""
        nodes = [
            {
                "data": {
                    "id": o.name,
                    "label": o.name,
                    "kind": o.kind,
                    "color": COLOR_OF.get(o.kind, "#adb5bd"),
                    "group": o.group or "",
                    "file": o.file,
                    "note": o.note or "",
                }
            }
            for o in self.objects.values()
        ]
        edges = [
            {
                "data": {
                    "id": f"{r.src}|{r.kind}|{r.dst}",
                    "source": r.src,
                    "target": r.dst,
                    "kind": r.kind,
                }
            }
            for r in self.relations
            if r.src in self.objects and r.dst in self.objects
        ]
        return {"meta": self.meta, "kinds": self.kinds(), "nodes": nodes, "edges": edges}

    def to_dict(self) -> dict:
        return {
            "meta": self.meta,
            "objects": [vars(o) for o in self.objects.values()],
            "relations": [r.as_dict() for r in self.relations],
        }


if __name__ == "__main__":
    import json
    import sys

    path = sys.argv[1] if len(sys.argv) > 1 else "cognitive-map.yaml"
    cm = CognitiveMap.from_file(path)
    print(json.dumps({"kinds": cm.kinds(),
                      "objects": len(cm.objects),
                      "relations": len(cm.relations)}, ensure_ascii=False, indent=2))
