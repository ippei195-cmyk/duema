"""効果文から作った手順（JSON）を、学習エンジンの Op の並び（src/textfx_progs.rs）に変換する。
使い方: python3 textfx/compile.py progs.json > src/textfx_progs.rs
progs.json = [{"id":"dm22ex1-037","ev":"spell","side":0,"steps":[...]}, ...]
手順の形は対戦卓の「処理の変更」（STEP_SPEC）と同じ。エンジンで実行できない手順（note・cast など）が
1つでもある能力は変換せず、標準エラーに理由を出す（あいまいなまま動かさない）。"""
import json, re, sys, pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
cards_rs = (ROOT / 'src' / 'cards.rs').read_text(encoding='utf-8')
IDS = re.findall(r'CardDef \{ id: "([^"]+)"', cards_rs)
VARIANTS = [v.strip() for v in re.sub(r'//[^\n]*', '', re.search(r'pub enum C \{([^}]*)\}', cards_rs).group(1)).replace('\n', ' ').split(',') if v.strip()]
assert len(IDS) == len(VARIANTS), (len(IDS), len(VARIANTS))
ID2C = dict(zip(IDS, VARIANTS))

CIV = {'光': 'L', '水': 'W', '闇': 'D', '火': 'F', '自然': 'N'}
RACE = {'ドラゴン': 'R_DRAGON', 'ジョーカーズ': 'R_JOKERS', '生徒会組': 'R_SEITOKAI', 'アビス': 'R_ABYSS', 'コマンド': 'R_COMMAND', 'ジャイアント': 'R_GIANT', 'マジック': 'R_MAGIC', 'オラクル': 'R_ORACLE'}
KW = {'ブロッカー': 'K_BLOCKER', 'スピードアタッカー': 'K_SA', 'マッハファイター': 'K_MACH', 'S・トリガー': 'K_STRIG', 'G・ストライク': 'K_GSTRIKE', 'ジャストダイバー': 'K_JUSTDIVER', 'EXライフ': 'K_EXLIFE', 'W・ブレイカー': 'K_WB', 'T・ブレイカー': 'K_TB'}
KIND = {'カード': 0, 'クリーチャー': 1, '呪文': 2, 'フィールド': 4, 'G城': 4, 'タマシード': 8, 'エレメント': 1 | 4 | 8}
ZONE = {'MY_HAND': ('Me', 'Hand'), 'MY_BATTLE': ('Me', 'Battle'), 'MY_MANA': ('Me', 'Mana'), 'MY_GRAVE': ('Me', 'Grave'), 'MY_SHIELD': ('Me', 'Shield'),
        'OPP_HAND': ('Opp', 'Hand'), 'OPP_BATTLE': ('Opp', 'Battle'), 'OPP_MANA': ('Opp', 'Mana'), 'OPP_GRAVE': ('Opp', 'Grave'), 'OPP_SHIELD': ('Opp', 'Shield')}
CZ = {'mana': 'Mana', 'hand': 'Hand', 'grave': 'Grave', 'battle': 'Battle', 'shield': 'Shield', 'deck': 'Deck'}
TO = {'hand': 'Hand', 'mana': 'Mana', 'manaTapped': 'ManaTapped', 'grave': 'Grave', 'shield': 'Shield', 'deckTop': 'DeckTop', 'deckBottom': 'DeckBottom', 'battle': 'Battle'}
NVARS = 6


class Fail(Exception):
    pass


def who(x):
    return 'Opp' if x == 'opp' else 'Me'


def filt(f):
    if not f:
        return 'ANY'
    out = {}
    for k, v in f.items():
        if k == 'type':
            if v not in KIND: raise Fail(f'種類 {v}')
            if KIND[v]: out['kinds'] = str(KIND[v])
        elif k == 'civ':
            cs = [c for c in re.split(r'[/・、]', v) if c]
            if any(c not in CIV for c in cs): raise Fail(f'文明 {v}')
            out['civ'] = ' | '.join(CIV[c] for c in cs)
        elif k == 'race':
            if v not in RACE: raise Fail(f'種族 {v}')
            out['race'] = RACE[v]
        elif k == 'text':
            if v not in KW: raise Fail(f'能力 {v}')
            out['kw'] = KW[v]
        elif k == 'minCost': out['min_cost'] = str(int(v))
        elif k == 'maxCost': out['max_cost'] = str(int(v))
        elif k == 'minPower': out['min_power'] = str(int(v))
        elif k == 'maxPower': out['max_power'] = str(int(v))
        elif k == 'nonEvolution':
            if v: out['non_evo'] = 'true'
        elif k == 'tapped': out['tapped'] = '1' if v else '2'
        elif k == 'mono':
            if v: out['mono'] = 'true'
        elif k == 'name':
            raise Fail('名前の条件')
        else:
            raise Fail(f'条件 {k}')
    return 'Filt { ' + ', '.join(f'{k}: {v}' for k, v in out.items()) + ', ..ANY }'


class Comp:
    def __init__(self):
        self.ops = []; self.vars = {}

    def var(self, name, new=False):
        name = name.lstrip('$')
        if name not in self.vars:
            if not new: raise Fail(f'変数 ${name} が先に作られていない')
            if len(self.vars) >= NVARS: raise Fail('変数が多すぎる')
            self.vars[name] = len(self.vars)
        return self.vars[name]

    def tmp(self):
        return self.var(f'_t{len(self.vars)}', True)

    def src(self, r):
        if isinstance(r, list): raise Fail('カードidの指定（対局ごとに違う）')
        if not isinstance(r, str) or not r.startswith('$'): raise Fail(f'参照 {r}')
        k = r[1:]
        if k == 'SELF': return 'Src::SelfCard'
        if k in ZONE:
            w, z = ZONE[k]; return f'Src::Zone(Who::{w}, Zone::{z})'
        return f'Src::Var({self.var(k)})'

    def ids_var(self, r):
        """move などの対象を変数にそろえる"""
        if isinstance(r, str) and (r[1:] in ZONE or r == '$SELF'):
            t = self.tmp(); self.emit(f'Filter {{ from: {self.src(r)}, f: ANY, var: {t} }}'); return t
        if isinstance(r, list) and len(r) == 1: return self.ids_var(r[0])
        if isinstance(r, str) and r.startswith('$'): return self.var(r[1:])
        raise Fail(f'対象 {r}')

    def emit(self, op):
        self.ops.append(op); return len(self.ops) - 1

    def patch(self, i, op):
        self.ops[i] = op

    def steps(self, steps):
        for st in steps or []:
            self.step(st)

    def step(self, st):
        d = st.get('do')
        n = lambda k='n', dv=1: int(st.get(k, dv) if st.get(k) is not None else dv)
        if d == 'optional':
            i = self.emit(None); self.steps(st.get('steps')); self.patch(i, f'Optional {{ skip: {len(self.ops)} }}')
        elif d == 'mode':
            if int(st.get('times', 1)) != 1: raise Fail('2つ以上選ぶモード')
            i = self.emit(None); starts = []; jumps = []
            for o in st.get('options', []):
                starts.append(len(self.ops)); self.steps(o.get('steps')); jumps.append(self.emit(None))
            for j in jumps: self.patch(j, f'Jump({len(self.ops)})')
            self.patch(i, f'Mode {{ targets: &[{", ".join(map(str, starts))}] }}')
        elif d == 'look':
            self.emit(f'Look {{ who: Who::{who(st.get("who"))}, n: {n()}, var: {self.var(st.get("as", "L"), True)} }}')
        elif d == 'choose':
            frm = self.src(st.get('from', st.get('cands')))
            if st.get('random'):
                self.emit(f'Random {{ from: {frm}, f: {filt(st.get("filter"))}, n: {int(st.get("max", 1))}, var: {self.var(st.get("as", "X"), True)} }}'); return
            mx = int(st.get('max', 1) if st.get('max') is not None else 1); mn = int(st.get('min', 0) or 0)
            self.emit(f'Choose {{ from: {frm}, f: {filt(st.get("filter"))}, min: {min(mn, 99)}, max: {min(mx, 99)}, var: {self.var(st.get("as", "X"), True)}, by: Who::{who(st.get("by"))} }}')
        elif d == 'filter':
            self.emit(f'Filter {{ from: {self.src(st.get("from", st.get("ids")))}, f: {filt(st.get("filter"))}, var: {self.var(st.get("as", "X"), True)} }}')
        elif d == 'move':
            to = st.get('to')
            if to not in TO: raise Fail(f'移動先 {to}')
            v = self.ids_var(st.get('ids'))
            ex = 255
            if st.get('except'):
                e = st['except']; e = e if isinstance(e, list) else [e]
                if len(e) != 1: raise Fail('除くものが2つ以上')
                ex = self.ids_var(e[0])
            self.emit(f'Move {{ var: {v}, except: {ex}, to: To::{TO[to]} }}')
        elif d == 'put':
            if st.get('destroyAtEnd'): raise Fail('ターンの終わりに破壊')
            self.emit(f'Move {{ var: {self.ids_var(st.get("ids"))}, except: 255, to: To::Battle }}')
        elif d == 'destroy':
            self.emit(f'Destroy {{ var: {self.ids_var(st.get("ids"))} }}')
        elif d == 'draw':
            self.emit(f'Draw {{ who: Who::{who(st.get("who"))}, n: {n()} }}')
        elif d == 'topToMana':
            self.emit(f'TopToMana {{ who: Who::{who(st.get("who"))}, n: {n()}, tapped: {"false" if st.get("tapped") is False else "true"} }}')
        elif d == 'topToShield':
            self.emit(f'TopToShield {{ who: Who::{who(st.get("who"))}, n: {n()} }}')
        elif d == 'shuffle':
            pass  # 近似：エンジンの山札は見ていない順番なので何もしない
        elif d in ('tap', 'untap', 'freeze', 'noattack'):
            name = {'tap': 'Tap', 'untap': 'Untap', 'freeze': 'Freeze', 'noattack': 'NoAttack'}[d]
            self.emit(f'{name} {{ var: {self.ids_var(st.get("ids"))} }}')
        elif d == 'power':
            self.emit(f'Power {{ var: {self.ids_var(st.get("ids"))}, delta: {int(st.get("delta", 0))} }}')
        elif d == 'if':
            conds = st.get('conds') or []
            if not conds: raise Fail('条件が空')
            idx = []
            for cd in conds:
                if cd.get('zone', 'mana') not in CZ: raise Fail(f'条件の場所 {cd.get("zone")}')
                op = {'>=': 'Ge', '<=': 'Le', '==': 'Eq'}.get(cd.get('op', '>='))
                if not op: raise Fail(f'比べ方 {cd.get("op")}')
                idx.append((self.emit(None), cd, op))
            self.steps(st.get('steps'))
            j = self.emit(None) if st.get('else') else None
            els = len(self.ops)
            for i, cd, op in idx:
                self.patch(i, f'If {{ who: Who::{who(cd.get("who"))}, zone: Zone::{CZ[cd.get("zone", "mana")]}, f: {filt(cd.get("filter"))}, cmp: Cmp::{op}, n: {int(cd.get("n", 0))}, els: {els} }}')
            if j is not None:
                self.steps(st.get('else')); self.patch(j, f'Jump({len(self.ops)})')
        elif d == 'win':
            self.emit('Win')
        elif d == 'lose':
            self.emit(f'Lose {{ who: Who::{who(st.get("who"))} }}')
        else:
            raise Fail(f'エンジンで実行できない手順 {d}')


EV = {'etb': 'Ev::Etb', 'attack': 'Ev::Attack', 'turnStart': 'Ev::TurnStart', 'turnEnd': 'Ev::TurnEnd', 'destroyed': 'Ev::Destroyed'}


def compile_one(p):
    c = Comp(); c.steps(p['steps']); c.emit('End')
    ev = f'Ev::Spell({int(p.get("side", 0))})' if p['ev'] == 'spell' else EV.get(p['ev'])
    if not ev: raise Fail(f'タイミング {p["ev"]}')
    if p['id'] not in ID2C: raise Fail(f'エンジンに無いカード {p["id"]}')
    return f'    (C::{ID2C[p["id"]]}, {ev}, &[{", ".join(c.ops)}]),'


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    cr = 'dmengine' if '--ext' in sys.argv else 'crate'  # --ext：テスト（クレートの外）から include! する時
    progs = json.loads(pathlib.Path(args[0]).read_text(encoding='utf-8')) if args else []
    lines = []; ng = 0
    for p in progs:
        try:
            lines.append(f'    // {p.get("name", p["id"])} {p["ev"]}\n' + compile_one(p))
        except (Fail, TypeError, ValueError, KeyError, AttributeError) as e:
            ng += 1; print(f'NG {p.get("name", p["id"])} {p["ev"]}: {e}', file=sys.stderr)
    print('// textfx/compile.py が作るファイル（手で書き換えない）。効果文から作った手順の一覧。')
    print(f'#[allow(unused_imports)]\nuse {cr}::cards::*;\nuse {cr}::textfx::*;\n\n#[allow(unused_imports)]\nuse {cr}::textfx::Op::*;\n')
    print('pub static PROGS: &[(C, Ev, &[Op])] = &[')
    print('\n'.join(lines))
    print('];')
    print(f'変換 {len(lines)} 件、変換できない {ng} 件', file=sys.stderr)


if __name__ == '__main__':
    main()
