"""Make web/sim_local.html from sim/simulator.html: load the bridge first and apply a few small patches.

Run again whenever you replace sim/simulator.html with a newer version of the simulator.
"""
import sys

from .config import SIM_OUT, SIM_SRC

PATCHES = [
    # (old, new, how many times it must appear)
    ("<script>", '<script src="/bridge.js"></script>\n<script>', 1),
    ("render(); await sleep(350);", "render(); await sleep(window.DMAI_FAST?0:350);", 1),
    ("steps < 14", "steps < (window.DMAI_MAXSTEPS||14)", 1),
    ("steps<14", "steps<(window.DMAI_MAXSTEPS||14)", 1),
    # extra card scripts and rule fixes (web/patches.js) run after the simulator's script
    ("</body>", '<script src="/patches.js"></script>\n</body>', 1),
    # 「ブロックできない」効果と、超魂Xで与えられるブロッカー（patches.js の DMAI_NOBLOCK / DMAI_SOULBLOCK）
    ("const isBlocker = c => isCreature(side(c)) && /ブロッカー/.test(txt(c)) && !/「ブロッカー」を持つ/.test(txt(c));",
     "const isBlocker = c => !window.DMAI_NOBLOCK?.(c) && ((isCreature(side(c)) && /ブロッカー/.test(txt(c)) && !/「ブロッカー」を持つ/.test(txt(c))) || !!window.DMAI_SOULBLOCK?.(c));", 1),
    # 「ブロックされない」攻撃クリーチャー
    ("const blockers = inBattle(q, x => isBlocker(x) && !x.tapped && x.uid!==target);",
     "const blockers = window.DMAI_UNBLOCKABLE?.(A) ? [] : inBattle(q, x => isBlocker(x) && !x.tapped && x.uid!==target);", 1),
    # ブレイクしたシールドを手札のかわりに山札の下へ（ボルメテウス・リバース・ドラゴン）と、その後のドロー
    ("else { place(u,'hand'); broken.push(u); }", "else if (window.DMAI_BREAK_TO_BOTTOM?.(A, u)) {} else { place(u,'hand'); broken.push(u); }", 1),
    ("log(`${PN[q]}のシールドを${toBreak.length}つブレイク`, p);", "log(`${PN[q]}のシールドを${toBreak.length}つブレイク`, p); window.DMAI_AFTER_BREAK?.(A);", 1),
    # ツインパクトの呪文面にあるG・ストライク
    ("const gs = /\\{GS\\}/.test(DB[c.cid].sides[0].text);", "const gs = DB[c.cid].sides.some(s => /\\{GS\\}/.test(s.text));", 1),
    # ハイパーモード面（patches.js で分けた2面目）はS・トリガーの確認に出さない
    ("const st = sd.map((s,i)=>({s,i})).filter(({s}) => ", "const st = sd.map((s,i)=>({s,i})).filter(({s}) => !s.hyper && ", 1),
]


def build():
    if not SIM_SRC.exists():
        sys.exit(f"シミュレーターが見つかりません: {SIM_SRC}")
    html = SIM_SRC.read_text(encoding="utf-8")
    for old, new, n in PATCHES:
        found = html.count(old)
        if found != n:
            sys.exit(f"シミュレーターの形が想定と違います（'{old}' が {found} 箇所）。bridge の更新が必要です。")
        html = html.replace(old, new)
    SIM_OUT.parent.mkdir(parents=True, exist_ok=True)
    SIM_OUT.write_text(html, encoding="utf-8")
    print(f"作成しました: {SIM_OUT}")


if __name__ == "__main__":
    build()
