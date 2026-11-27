"""効果文 → 手順(steps) を「Python の前処理＋ローカルAI（Qwen）」で作る（2回目の方式）。

1回目（run_qwen.py）は、カードのテキスト全部を Qwen に渡して手順を丸ごと書かせたので、ほとんど間違えた。
この方式では Qwen の仕事を小さくする：
  1. Python が、カードから「出た時」「呪文」「攻撃する時」の能力だけを切り出す（キーワード能力・注釈・置換効果は外す）。
  2. 能力を1文ずつ（「その後、」「、残り」でも）区切る。「〜なら、」「〜あれば、」の条件は文から切り離す。
  3. よくある決まり文句（カードを引く・山札の上から見る・残りを山札の下に・手札を捨てさせる など）は Python が手順にする。
  4. 残った1文だけを Qwen に渡す。使える変数（$L・$X…）と文中の数を添え、答えの形は JSON スキーマで縛る。
  5. Qwen の答えを Python が確かめる（使えない手順・ない変数・文中の数が抜けている）。だめならエラーを添えて最大2回やり直し。
対局中には使わない。できた手順は、エンジンの手書き処理と同じ結果になるかをテスト（textfx_grade）で採点してから使う。

使い方: python pipeline.py cards.json out.json [--model qwen3:8b] [--rules-only]
"""
import json, re, sys, time, pathlib, urllib.request, urllib.error

HERE = pathlib.Path(__file__).parent
ARGS = [a for a in sys.argv[1:] if not a.startswith('--')]
MODEL = 'qwen3:8b'
if '--model' in sys.argv:
    MODEL = sys.argv[sys.argv.index('--model') + 1]
    ARGS = [a for a in ARGS if a != MODEL]
RULES_ONLY = '--rules-only' in sys.argv
THINK = '--think' in sys.argv  # Qwen に考えてから答えさせる（遅いが正確になるか試す）
OLLAMA = 'http://127.0.0.1:11434/api/chat'
# 採点で合格した「文 → 手順」をためた例文集。似た文の正解を Qwen に見せる（同じカードの例は使わない）
BANK = []
if '--bank' in sys.argv:
    _b = sys.argv[sys.argv.index('--bank') + 1]
    ARGS = [a for a in ARGS if a != _b]
    BANK = json.loads(pathlib.Path(_b).read_text(encoding='utf-8'))
CUR = {'id': None}


def similar(cl, k=4):
    def grams(t):
        return {t[i:i + 2] for i in range(len(t) - 1)}
    g = grams(cl)
    scored = sorted(((len(g & grams(b['clause'])) / (len(g | grams(b['clause'])) or 1), b) for b in BANK if b['id'] != CUR['id']),
                    key=lambda x: -x[0])
    return [b for sc, b in scored[:k] if sc > 0.15]

ZEN = str.maketrans('０１２３４５６７８９＋－−，', '0123456789+--,')
KEYWORDS = ['W・ブレイカー', 'T・ブレイカー', 'Q・ブレイカー', 'ブロッカー', 'マッハファイター', 'スピードアタッカー', 'ジャストダイバー', 'EXライフ',
            'S・トリガー', 'G・ストライク', 'スマッシュ・バースト', 'アビスラッシュ', 'Zラッシュ', 'スレイヤー', 'シールド・ゴー', '水晶チャージャー']
SPECIAL = ['$MY_HAND', '$MY_BATTLE', '$MY_MANA', '$MY_GRAVE', '$MY_SHIELD', '$OPP_HAND', '$OPP_BATTLE', '$OPP_MANA', '$OPP_GRAVE', '$OPP_SHIELD', '$SELF']
DOS = ['look', 'choose', 'filter', 'move', 'put', 'destroy', 'draw', 'topToMana', 'topToShield', 'shuffle', 'tap', 'untap', 'freeze', 'noattack',
       'giveSA', 'lock', 'untapMana', 'power', 'win', 'lose']
TO = ['hand', 'mana', 'manaTapped', 'grave', 'shield', 'deckTop', 'deckBottom', 'battle']
VARS = ['X', 'Y', 'Z', 'W', 'V']


# ---------------- 1. 能力の切り出し ----------------
def strip_notes(t):
    t = t.translate(ZEN)
    prev = None
    while prev != t:  # 入れ子の（…）も外す
        prev = t
        t = re.sub(r'（[^（）]*）|\([^()]*\)', '', t)
    t = re.sub(r'\{[^}]*\}', '', t)
    t = re.sub(r'\[[^\]]*\]', '', t)
    return t


def sentences(t):
    t = t.replace('\n', '。').replace('　', '').replace(' ', '')
    quoted = re.findall(r'「[^」]*」', t)  # 「スピードアタッカー」を与える などのキーワードは残す
    for i, q in enumerate(quoted):
        t = t.replace(q, f'@{i}@', 1)
    for k in KEYWORDS:
        t = t.replace(k, '。')
    for i, q in enumerate(quoted):
        t = t.replace(f'@{i}@', q)
    out = []
    for x in (x.strip('、') for x in t.split('。')):
        if not x:
            continue
        if '：' in x:  # 「逆転撃：…」などキーワード能力の説明。続く「そうしたら」「支払えなければ」の文も外す
            out.append(None); continue
        if out and out[-1] is None and re.match(r'^(そうしたら|支払えなければ)', x):
            continue
        out.append(x)
    return [x for x in out if x]


CONT = re.compile(r'^(その後、|その中から|残り|このターン、|次の(自分|相手)のターン|そうしたら|それ|その|相手はそれ|こうして|そのターン、)')
# 出た時に動くキーワード能力（注釈の中に処理が書いてある）。これがある面の「出た時」は一部しか手順にできないので手書きに任せる
ETB_KW = re.compile(r'超魂レイド|超魂X|EXライフ')
TRIG = re.compile(r'このクリーチャーが(バトルゾーンに)?出た時(または攻撃する時)?、')


def abilities(card):
    """[(ev, side, [文...], 元の文)] を返す。手順にしない能力は skipped に理由を入れる。"""
    faces = re.split(r'\n?――\n?', card['text'])
    types = [x.strip() for x in card['type'].split('/')]
    out, skipped = [], []
    for side, face in enumerate(faces):
        ty = types[side] if side < len(types) else types[-1]
        ss = sentences(strip_notes(face))
        if '呪文' in ty:
            hard = [s for s in ss if re.search(r'かわりに|唱えられない|コストを支払わずに唱え', s)]
            if hard:  # 一部だけ手順にすると効果が変わるので、呪文ごと手書きに任せる
                skipped.append((side, '手順にできない文がある呪文: ' + ' / '.join(hard)))
                continue
            body = ss
            drop = [s for s in body if s.startswith('ただし')]
            body = [s for s in body if not s.startswith('ただし')]
            if drop:
                skipped.append((side, 'ただし書きは手順にしない: ' + ' / '.join(drop)))
            if body:
                out.append(('spell', side, body))
            continue
        i = 0
        while i < len(ss):
            s = ss[i]
            m = TRIG.search(s)
            if not m or re.search(r'召喚(によって|して)', s[:m.end()]):
                if m:
                    skipped.append((side, '召喚した時だけの能力: ' + s))
                i += 1
                continue
            if ETB_KW.search(face) and 'EXライフ' not in face:
                skipped.append((side, '出た時に動くキーワード能力がある: ' + s))
                i += 1
                continue
            if '各ターンに' in s[:m.start()]:
                skipped.append((side, '各ターンに１度の能力: ' + s))
            body = [s[m.end():]]
            j = i + 1
            while j < len(ss) and CONT.match(ss[j]) and not TRIG.search(ss[j]):
                body.append(ss[j]); j += 1
            body = [b for b in body if not b.startswith('ただし')]
            out.append(('etb', side, body))
            if m.group(2):
                out.append(('attack', side, body))
            i = j
    # 両面カードで同じ能力が2回出たら1つに。同じ面・同じ時の能力が2つあれば、続けて1つの能力にする
    uniq, seen = [], set()
    for ev, side, body in out:
        key = (ev, tuple(body))
        if key in seen:
            continue
        seen.add(key)
        same = [u for u in uniq if u[0] == ev and u[1] == side]
        if same:
            same[0][2].extend(body)
        else:
            uniq.append((ev, side, list(body)))
    return uniq, skipped


def clauses(body):
    """文をさらに「その後、」「、残り」で区切る。[(文の番号, 区切った文)]"""
    out = []
    for k, s in enumerate(body):
        s = re.sub(r'^(その後|そうしたら)、', '', s)
        # 「このクリーチャーに含まれるカード1枚につき、…を1体」→ 数を $CONTAINED に（Python で書き換える）
        m = re.match(r'^このクリーチャーに含まれるカード1枚につき、(.*?)(1)(体|枚|つ)(.*)$', s)
        if m:
            s = m.group(1) + '$CONTAINED' + m.group(3) + m.group(4)
        m = re.match(r'^このクリーチャーに含まれるカードの枚数まで、自分の山札の上から(マナゾーンに置く|シールド化する)$', s)
        if m:
            s = '自分の山札の上から$CONTAINED枚を' + m.group(1)
        for part in re.split(r'、その後、|、(?=残り)', s):
            part = part.strip('、')
            if part:
                out.append((k, part))
    return out


# ---------------- 2. 決まり文句（Python で手順にする） ----------------
def parse_filter(ph):
    """「コスト3以下の」「ジョーカーズ・クリーチャー」などを filter にする。読めない語が残れば None。"""
    f, rest = {}, ph
    rules = [
        (r'相手のマナゾーンにあるカードの枚数より(大きい|多い)コストを持つ(相手の)?', lambda m: f.__setitem__('costGreaterThanMana', 'opp')),
        (r'コスト(\d+)以下の', lambda m: f.__setitem__('maxCost', int(m.group(1)))),
        (r'コスト(\d+)以上の', lambda m: f.__setitem__('minCost', int(m.group(1)))),
        (r'パワー(\d+)以下の', lambda m: f.__setitem__('maxPower', int(m.group(1)))),
        (r'パワー(\d+)以上の', lambda m: f.__setitem__('minPower', int(m.group(1)))),
        (r'、?パワーが一番小さい', lambda m: f.__setitem__('lowestPower', True)),
        (r'進化ではない', lambda m: f.__setitem__('nonEvolution', True)),
        (r'(光|水|闇|火|自然)の', lambda m: f.__setitem__('civ', m.group(1))),
        (r'(ジョーカーズ|ドラゴン|アビス|ジャイアント|生徒会組|コマンド|マジック|オラクル)・?', lambda m: f.__setitem__('race', m.group(1))),
        (r'(クリーチャー)$', lambda m: f.__setitem__('type', 'クリーチャー')),
        (r'(呪文)$', lambda m: f.__setitem__('type', '呪文')),
        (r'(カード|エレメント)$', lambda m: None),
    ]
    for pat, fn in rules:
        m = re.search(pat, rest)
        if m:
            fn(m)
            rest = rest[:m.start()] + rest[m.end():]
    rest = rest.strip('、の')
    return f if rest == '' else None


def num(x):
    return x if x == '$CONTAINED' else int(x)


# エンジンの手順で表せない言い回し。これがある文は Qwen に聞かずに、能力ごと手書きに任せる
CANNOT = re.compile(r'∞|出せない|唱えられない|その持ち主|バトルゾーンにある(?!自分|相手)|名前を|ブレイク|無視|かわりに|離れない|負けず')


class Ctx:
    def __init__(self):
        self.look = None      # 山札の上から見たカード（'$L'）
        self.picked = []      # これまでに選んだ変数（新しい順は [-1]）
        self.from_look = []   # $L から選んだ変数

    def new(self, src=None):
        v = VARS[len(self.picked)]
        self.picked.append(v)
        if src == '$L':
            self.from_look.append(v)
        return v

    def last(self):
        return '$' + self.picked[-1] if self.picked else None


def rule(cl, cx):
    """決まり文句なら手順のリスト、違えば None。"""
    m = re.fullmatch(r'カードを(\d+)枚引く', cl)
    if m:
        return [{'do': 'draw', 'n': int(m.group(1))}]
    m = re.fullmatch(r'カードを(\d+)枚まで引く', cl)
    if m:
        n = int(m.group(1))
        return [{'do': 'mode', 'q': '何枚引く？', 'options': [{'label': f'{k}枚', 'steps': [{'do': 'draw', 'n': k}] if k else []} for k in range(n + 1)]}]
    m = re.fullmatch(r'カードを(\d+)枚引いてもよい', cl)
    if m:
        return [{'do': 'optional', 'q': '引きますか？', 'steps': [{'do': 'draw', 'n': int(m.group(1))}]}]
    m = re.fullmatch(r'自分の山札の上から(\d+)枚を(見る|表向きにする)', cl)
    if m:
        cx.look = '$L'
        return [{'do': 'look', 'n': int(m.group(1)), 'as': 'L'}]
    m = re.fullmatch(r'自分の山札の上から(\d+|\$CONTAINED)枚を(タップして)?マナゾーンに(タップして)?置く', cl)
    if m:
        return [{'do': 'topToMana', 'n': num(m.group(1)), 'tapped': bool(m.group(2) or m.group(3))}]
    m = re.fullmatch(r'自分の山札の上から(\d+|\$CONTAINED)枚をシールド化する', cl)
    if m:
        return [{'do': 'topToShield', 'n': num(m.group(1))}]
    m = re.fullmatch(r'残り(の\d+枚|のカード)?を(好きな順(序|番)で)?(自分の)?山札の(一番)?下に置く', cl)
    if m and cx.look:
        st = {'do': 'move', 'ids': '$L', 'to': 'deckBottom'}
        if cx.from_look:
            st['except'] = '$' + cx.from_look[-1]
        return [st]
    m = re.fullmatch(r'相手の手札を見て(\d+)枚(まで)?選び、捨てさせる', cl)
    if m:
        n = int(m.group(1)); v = cx.new()
        return [{'do': 'choose', 'from': '$OPP_HAND', 'min': 0 if m.group(2) else n, 'max': n, 'as': v}, {'do': 'move', 'ids': '$' + v, 'to': 'grave'}]
    m = re.fullmatch(r'相手の手札を(\d+)枚見ないで選び、捨てさせる', cl)
    if m:
        v = cx.new()
        return [{'do': 'choose', 'from': '$OPP_HAND', 'random': True, 'max': int(m.group(1)), 'as': v}, {'do': 'move', 'ids': '$' + v, 'to': 'grave'}]
    m = re.fullmatch(r'自分のシールドを(\d+)つ(マナゾーンに置く|手札に加える|墓地に置く)', cl)
    if m:
        n = int(m.group(1)); v = cx.new()
        to = {'マナゾーンに置く': 'mana', '手札に加える': 'hand', '墓地に置く': 'grave'}[m.group(2)]
        return [{'do': 'choose', 'from': '$MY_SHIELD', 'min': n, 'max': n, 'as': v}, {'do': 'move', 'ids': '$' + v, 'to': to}]
    if cl == 'アンタップする':
        return [{'do': 'untap', 'ids': '$SELF'}]
    m = re.fullmatch(r'相手の、?(.*?)を(\d+|\$CONTAINED)(体|つ)選(び、(破壊する|タップする|手札に戻す|持ち主の手札に戻す)|ぶ)', cl)
    if m:
        f = parse_filter(m.group(1))
        if f is not None:
            n = num(m.group(2)); v = cx.new()
            ch = {'do': 'choose', 'from': '$OPP_BATTLE', 'min': n, 'max': n, 'as': v}
            if f:
                ch['filter'] = f
            if not m.group(5):
                return [ch]
            act = {'破壊する': {'do': 'destroy', 'ids': '$' + v}, 'タップする': {'do': 'tap', 'ids': '$' + v}}.get(m.group(5), {'do': 'move', 'ids': '$' + v, 'to': 'hand'})
            return [ch, act]
    m = re.fullmatch(r'相手の、?(.*?)をすべて(破壊する|手札に戻す)', cl)
    if m:
        f = parse_filter(m.group(1))
        if f is not None:
            v = cx.new()
            fl = {'do': 'filter', 'from': '$OPP_BATTLE', 'filter': {k: x for k, x in f.items() if k != 'type'}, 'as': v}
            return [fl, {'do': 'destroy', 'ids': '$' + v} if m.group(2) == '破壊する' else {'do': 'move', 'ids': '$' + v, 'to': 'hand'}]
    if cl in ('自分の墓地にあるカードをすべて山札に加え、シャッフルする', '自分の墓地のカードをすべて山札に加えてシャッフルする'):
        return [{'do': 'move', 'ids': '$MY_GRAVE', 'to': 'deckBottom'}, {'do': 'shuffle'}]
    m = re.fullmatch(r'その中から、?(.*?)を(\d+)(体|枚)(まで)?、?(相手に見せてから|表向きにし、)?(手札に加える|手札に加えてもよい|出す|出してもよい|マナゾーンに置く)', cl)
    if m and cx.look:
        f = parse_filter(m.group(1)) if m.group(1) else {}
        if f is not None:
            n = int(m.group(2)); v = cx.new('$L')
            ch = {'do': 'choose', 'from': '$L', 'min': 0 if (m.group(4) or 'もよい' in m.group(6)) else n, 'max': n, 'as': v}
            if f:
                ch['filter'] = f
            to = m.group(6)
            act = {'do': 'put', 'ids': '$' + v} if to.startswith('出') else {'do': 'move', 'ids': '$' + v, 'to': 'mana' if 'マナ' in to else 'hand'}
            return [ch, act]
    m = re.fullmatch(r'その中から、?(.*?)(\d+)枚を手札に加えるかマナゾーンに置(く|き)', cl)
    if m and cx.look:
        f = parse_filter(m.group(1)) if m.group(1) else {}
        if f is not None:
            n = int(m.group(2)); v = cx.new('$L')
            ch = {'do': 'choose', 'from': '$L', 'min': n, 'max': n, 'as': v}
            if f:
                ch['filter'] = f
            return [ch, {'do': 'mode', 'q': '手札かマナか', 'options': [{'label': '手札', 'steps': [{'do': 'move', 'ids': '$' + v, 'to': 'hand'}]},
                                                                     {'label': 'マナ', 'steps': [{'do': 'move', 'ids': '$' + v, 'to': 'mana'}]}]}]
    m = re.fullmatch(r'その中から、?(.*?)を好きな(枚数|数)手札に加え(る)?', cl)
    if m and cx.look:
        f = parse_filter(m.group(1))
        if f is not None:
            v = cx.new('$L')
            return [{'do': 'filter', 'from': '$L', 'filter': f, 'as': v}, {'do': 'move', 'ids': '$' + v, 'to': 'hand'}]
    return None


# ---------------- 3. Qwen への問い合わせ ----------------
SPEC = """手順(steps)の書き方。1つの文を、ゲームエンジンが実行できる手順のJSON配列にする。
- {"do":"choose","from":"$OPP_BATTLE","filter":{...},"min":1,"max":1,"as":"X","random":false,"by":"me"}  カードを選ぶ。「〜まで」「〜てもよい」は min 0。「見ないで選ぶ」は random:true。「相手は自身の〜を選び」は by:"opp"。
- {"do":"filter","from":"$OPP_BATTLE","filter":{...},"as":"X"}  条件に合うカードをすべて（「すべて」のとき）
- {"do":"move","ids":"$X","to":"hand|mana|manaTapped|grave|shield|deckTop|deckBottom"}  カードを動かす（「捨てる」は grave、「山札の下」は deckBottom、「シールド化」は shield）
- choose・filter・look には必ず "as" を付ける。選んだ後に「破壊する」「手札に戻す」などがあれば、その手順も続けて書く。
- {"do":"put","ids":"$X"} バトルゾーンに出す / {"do":"destroy","ids":"$X"} 破壊
- {"do":"draw","n":1} / {"do":"topToMana","n":1,"tapped":false}（「タップして」なら tapped:true） / {"do":"topToShield","n":1}（シールド化）
- {"do":"tap","ids":"$X"} / {"do":"untap","ids":"$X"} / {"do":"freeze","ids":"$X"}（「次の自分のターンのはじめまで…アンタップしない」）
- {"do":"noattack","ids":"$X"}（このターン攻撃できない） / {"do":"lock","ids":"$X"}（次の自分のターンのはじめまで攻撃もブロックもできない）
- {"do":"giveSA","ids":"$X"}（「スピードアタッカー」を与える） / {"do":"power","ids":"$X","delta":3000}（パワーを+3000。−なら負の数）
- {"do":"untapMana","n":1}（自分のマナゾーンのカードをn枚アンタップ） / {"do":"win"} / {"do":"lose","who":"opp"}
- 置き場の変数：$MY_HAND $MY_BATTLE $MY_MANA $MY_GRAVE $MY_SHIELD $OPP_HAND $OPP_BATTLE $OPP_MANA $OPP_GRAVE $OPP_SHIELD、このクリーチャー自身は $SELF
- 「このクリーチャーに含まれるカード１枚につき」「〜の枚数まで」の数は "$CONTAINED"
- filter: {"type":"クリーチャー|呪文|カード","civ":"光|水|闇|火|自然","race":"種族","text":"ブロッカー などの能力名","minCost":0,"maxCost":3,"minPower":0,"maxPower":5000,
  "nonEvolution":true（進化ではない）,"lowestPower":true（パワーが一番小さい）,"costOf":"$X"（$Xと同じコスト）,"costGreaterThanMana":"opp"（相手のマナゾーンの枚数より大きいコスト）}
- 「相手の〜」のクリーチャー・エレメントは $OPP_BATTLE から、「自分の〜」は $MY_… から選ぶ。「エレメント」は type を付けない。
- 新しく選んだカードは、指定された変数名（as）に入れる。前に選んだカードを指す「それ」「そのクリーチャー」「それら」は、指定された変数を使う。"""

COND_SPEC = """条件(conds)の書き方：[{"who":"me|opp","zone":"mana|hand|grave|battle|shield|deck","filter":{...},"op":">=|<=|==","n":数}] か [{"self":"evolved"}]
- 「自分のマナゾーンにカードが10枚以上あれば」→ [{"who":"me","zone":"mana","op":">=","n":10}]
- 「相手のシールドがなければ」→ [{"who":"opp","zone":"shield","op":"==","n":0}]
- 「自分の他の〜があれば」：このクリーチャーも数えるので n は 2（例：自分の他のドラゴンがあれば → zone battle, filter {"race":"ドラゴン"}, op ">=", n 2）
- 「〜がすべて○○・カードなら」：○○ではないカードが0枚 → filter {"notRace":"○○"}, op "==", n 0
- 「それが進化クリーチャーなら」（このクリーチャー自身が進化）→ [{"self":"evolved"}]"""

EXAMPLES = [
    ('相手のクリーチャーを1体選び、持ち主の手札に戻す', {'新しい変数': 'X'},
     [{'do': 'choose', 'from': '$OPP_BATTLE', 'min': 1, 'max': 1, 'as': 'X'}, {'do': 'move', 'ids': '$X', 'to': 'hand'}]),
    ('相手のパワー5000以下のクリーチャーを2体まで選び、破壊する', {'新しい変数': 'X'},
     [{'do': 'choose', 'from': '$OPP_BATTLE', 'filter': {'maxPower': 5000}, 'min': 0, 'max': 2, 'as': 'X'}, {'do': 'destroy', 'ids': '$X'}]),
    ('相手は自身のクリーチャーを1体選び、山札の下に置く', {'新しい変数': 'X'},
     [{'do': 'choose', 'from': '$OPP_BATTLE', 'min': 1, 'max': 1, 'as': 'X', 'by': 'opp'}, {'do': 'move', 'ids': '$X', 'to': 'deckBottom'}]),
    ('自分の山札の上から3枚を墓地に置く', {'新しい変数': 'X'},
     [{'do': 'look', 'n': 3, 'as': 'X'}, {'do': 'move', 'ids': '$X', 'to': 'grave'}]),
    ('光のカードを2枚まで、自分のマナゾーンから手札に戻す', {'新しい変数': 'Y'},
     [{'do': 'choose', 'from': '$MY_MANA', 'filter': {'civ': '光'}, 'min': 0, 'max': 2, 'as': 'Y'}, {'do': 'move', 'ids': '$Y', 'to': 'hand'}]),
    ('このターン、そのクリーチャーのパワーを+4000する', {'前に選んだカード': '$X'},
     [{'do': 'power', 'ids': '$X', 'delta': 4000}]),
    ('そのターン、それは攻撃できない', {'前に選んだカード': '$Y'}, [{'do': 'noattack', 'ids': '$Y'}]),
    ('次の自分のターンのはじめまで、それらはアンタップしない', {'前に選んだカード': '$X'}, [{'do': 'freeze', 'ids': '$X'}]),
    ('このクリーチャーに含まれるカード1枚につき、カードを1枚引く', {}, [{'do': 'draw', 'n': '$CONTAINED'}]),
    ('自分のシールドを1つ見ないで選び、墓地に置く', {'新しい変数': 'X'},
     [{'do': 'choose', 'from': '$MY_SHIELD', 'random': True, 'max': 1, 'as': 'X'}, {'do': 'move', 'ids': '$X', 'to': 'grave'}]),
    ('相手の手札を2枚見ないで選ぶ', {'新しい変数': 'Y'}, [{'do': 'choose', 'from': '$OPP_HAND', 'random': True, 'max': 2, 'as': 'Y'}]),
    ('相手はそれらをすべて手札に戻す', {'前に選んだカード': '$X, $Y'},
     [{'do': 'move', 'ids': '$X', 'to': 'hand'}, {'do': 'move', 'ids': '$Y', 'to': 'hand'}]),
    ('相手のブロッカーを持つクリーチャーをすべてタップする', {'新しい変数': 'X'},
     [{'do': 'filter', 'from': '$OPP_BATTLE', 'filter': {'text': 'ブロッカー'}, 'as': 'X'}, {'do': 'tap', 'ids': '$X'}]),
    ('自分の水のクリーチャーを1体、自分のマナゾーンから出してもよい', {'新しい変数': 'X'},
     [{'do': 'choose', 'from': '$MY_MANA', 'filter': {'type': 'クリーチャー', 'civ': '水'}, 'min': 0, 'max': 1, 'as': 'X'}, {'do': 'put', 'ids': '$X'}]),
    ('そのクリーチャーに「スピードアタッカー」を与える', {'前に選んだカード': '$X'}, [{'do': 'giveSA', 'ids': '$X'}]),
]

FILTER_SCHEMA = {'type': 'object', 'properties': {
    'type': {'type': 'string', 'enum': ['クリーチャー', '呪文', 'カード']}, 'civ': {'type': 'string', 'enum': ['光', '水', '闇', '火', '自然']},
    'race': {'type': 'string'}, 'notRace': {'type': 'string'}, 'text': {'type': 'string'},
    'minCost': {'type': 'integer'}, 'maxCost': {'type': 'integer'}, 'minPower': {'type': 'integer'}, 'maxPower': {'type': 'integer'},
    'nonEvolution': {'type': 'boolean'}, 'lowestPower': {'type': 'boolean'}, 'costOf': {'type': 'string'},
    'costGreaterThanMana': {'type': 'string', 'enum': ['opp', 'me']}}, 'additionalProperties': False}
NUM = {'anyOf': [{'type': 'integer'}, {'type': 'string', 'enum': ['$CONTAINED']}]}
STEP_SCHEMA = {'type': 'object', 'properties': {
    'do': {'type': 'string', 'enum': DOS}, 'n': NUM, 'as': {'type': 'string'}, 'from': {'type': 'string'}, 'ids': {'type': 'string'},
    'except': {'type': 'string'}, 'to': {'type': 'string', 'enum': TO}, 'filter': FILTER_SCHEMA, 'min': NUM, 'max': NUM,
    'random': {'type': 'boolean'}, 'tapped': {'type': 'boolean'}, 'who': {'type': 'string', 'enum': ['me', 'opp']},
    'by': {'type': 'string', 'enum': ['me', 'opp']}, 'delta': {'type': 'integer'}}, 'required': ['do'], 'additionalProperties': False}
SCHEMA = {'type': 'object', 'properties': {'steps': {'type': 'array', 'items': STEP_SCHEMA}, 'cannot': {'type': 'boolean'}}, 'required': ['steps', 'cannot']}
COND_SCHEMA = {'type': 'object', 'properties': {'conds': {'type': 'array', 'items': {'type': 'object', 'properties': {
    'who': {'type': 'string', 'enum': ['me', 'opp']}, 'zone': {'type': 'string', 'enum': ['mana', 'hand', 'grave', 'battle', 'shield', 'deck']},
    'filter': FILTER_SCHEMA, 'op': {'type': 'string', 'enum': ['>=', '<=', '==']}, 'n': {'type': 'integer'},
    'self': {'type': 'string', 'enum': ['evolved']}}, 'additionalProperties': False}}}, 'required': ['conds']}

CALLS = {'n': 0, 'sec': 0.0}


FMT = {'schema': True}


def ollama(prompt, schema):
    t = time.time()
    while True:
        body = json.dumps({'model': MODEL, 'messages': [{'role': 'user', 'content': prompt}], 'stream': False, 'think': THINK,
                           'format': schema if FMT['schema'] else 'json', 'options': {'temperature': 0, 'num_ctx': 8192}}).encode('utf-8')
        req = urllib.request.Request(OLLAMA, data=body, headers={'Content-Type': 'application/json'})
        try:
            with urllib.request.urlopen(req, timeout=600) as r:
                out = json.loads(r.read())['message']['content']
            break
        except urllib.error.HTTPError as e:
            if not FMT['schema']:
                raise
            print(f'スキーマ指定が使えないので JSON 形式だけにします: {e}', flush=True)
            FMT['schema'] = False
    CALLS['n'] += 1; CALLS['sec'] += time.time() - t
    return json.loads(out)


def hints(cl, cx):
    h = {}
    if cx.look:
        h['山札の上から見たカード'] = '$L'
    if cx.picked:
        h['前に選んだカード'] = ', '.join('$' + v for v in cx.picked)
    if len(cx.picked) < len(VARS):
        h['新しい変数'] = VARS[len(cx.picked)]
    nums = re.findall(r'[+-]?\d+', cl)
    if nums:
        h['文中の数'] = ', '.join(nums)
    return h


ALLOWED = {'choose': {'from', 'filter', 'min', 'max', 'as', 'random', 'by'}, 'filter': {'from', 'filter', 'as'}, 'look': {'n', 'as', 'who'},
           'move': {'ids', 'except', 'to'}, 'power': {'ids', 'delta'}, 'draw': {'n', 'who'}, 'topToMana': {'n', 'tapped', 'who'},
           'topToShield': {'n'}, 'untapMana': {'n'}, 'shuffle': set(), 'win': set(), 'lose': {'who'}}
ACT = re.compile(r'破壊|手札に(加え|戻)|マナゾーンに置|捨て|墓地に置|出す|出して|シールド化|山札の(一番)?下|タップ|アンタップ|攻撃|パワー|与え')


def tidy(steps, cx, cl=''):
    """Qwen がよく間違える形を Python で直す：as（変数名）の書き忘れ、その手順に関係ない項目、既定値のままの項目。"""
    used = set(cx.picked) | ({'L'} if cx.look else set())
    for st in steps:
        d = st.get('do')
        keep = ALLOWED.get(d, {'ids'})
        for k in [k for k in st if k != 'do' and k not in keep]:
            del st[k]
        if st.get('random') is False:
            del st['random']
        if st.get('by') == 'opp' and not re.search(r'相手は', cl):
            del st['by']
        if st.get('by') == 'me':
            del st['by']
        if st.get('who') == 'me':
            del st['who']
        if d in ('choose', 'filter', 'look') and st.get('as') in used:  # 前の変数と同じ名前は付け替える
            old, st['as'] = st['as'], next(x for x in VARS if x not in used)
            for later in steps[steps.index(st) + 1:]:
                for k in ('ids', 'from', 'except'):
                    if later.get(k) == '$' + old:
                        later[k] = '$' + st['as']
        if d in ('choose', 'filter', 'look') and not st.get('as'):
            v = 'L' if d == 'look' and 'L' not in used else next(x for x in VARS if x not in used)
            st['as'] = v
        if st.get('as'):
            used.add(st['as'])
        if d == 'choose' and st.get('random') and 'min' in st:
            del st['min']
    return steps


def check(cl, steps, cx):
    """Qwen の答えを確かめる。問題があればその説明（文字列）、なければ None。"""
    if not steps:
        return '手順が空です'
    known = set(SPECIAL) | {'$' + v for v in cx.picked} | ({'$L'} if cx.look else set())
    made = set()
    for st in steps:
        for k in ('from', 'ids', 'except'):
            v = st.get(k)
            if v is not None and v not in known and v not in made:
                return f'{k} の {v} はまだない変数です（使えるのは {", ".join(sorted(known | made))}）'
        if st.get('do') in ('choose', 'filter', 'look'):
            if not st.get('as'):
                return f'{st["do"]} に as（変数名）がありません'
            made.add('$' + st['as'])
            if st['do'] != 'look' and not st.get('from'):
                return 'choose/filter に from がありません'
        if st.get('do') in ('move',) and not st.get('to'):
            return 'move に to がありません'
        if st.get('do') in ('destroy', 'put', 'tap', 'untap', 'freeze', 'noattack', 'lock', 'giveSA', 'power', 'move') and not st.get('ids'):
            return f'{st["do"]} に ids がありません'
    TOWORD = {'grave': r'墓地|捨て', 'hand': r'手札', 'mana': r'マナ', 'manaTapped': r'マナ', 'shield': r'シールド', 'deckBottom': r'山札の(一番)?下', 'deckTop': r'山札の(一番)?上'}
    for st in steps:
        if st.get('as') and not re.fullmatch(r'[A-Z]', st['as']):
            return f'as は大文字1字（X・Y など）。「{st["as"]}」は使えません'
        if st.get('do') == 'move' and st.get('to') in TOWORD and not re.search(TOWORD[st['to']], cl):
            return f'文にない置き場（{st["to"]}）へ動かしています'
        if st.get('do') == 'choose' and isinstance(st.get('max'), int) and st['max'] < 1:
            return 'choose の max は1以上'
        if st.get('do') == 'look' and re.search(r'相手は自身の山札|相手の山札', cl) and st.get('who') != 'opp':
            return '相手の山札を見るときは who:"opp"'
        if st.get('do') == 'look' and '山札の上から' not in cl:
            return '文に「山札の上から」がないのに look があります'
        if st.get('do') == 'choose' and 'すべて' in cl and '選' not in cl:
            return '「すべて」は choose ではなく filter で全部にします'
        if st.get('do') == 'draw' and '引' not in cl:
            return '文に「引く」がないのに draw があります'
        if 'costOf' in (st.get('filter') or {}) and re.search(r'以下|以上', cl):
            return 'costOf は「同じコスト」だけ。「以下」「以上」は表せないので cannot にする'
        if st.get('do') == 'choose' and re.search(r'相手の.*(クリーチャー|エレメント)', cl) and st.get('from') in ('$OPP_MANA', '$OPP_HAND', '$OPP_GRAVE') and not re.search(r'マナゾーンから|手札から|墓地から', cl):
            return '相手のクリーチャー・エレメントは $OPP_BATTLE から選びます'
    if steps[-1].get('do') in ('choose', 'filter') and ACT.search(cl.split('選')[-1]):
        return '選んだカードをどうするか（破壊する・手札に戻す など）の手順がありません'
    dump = json.dumps(steps, ensure_ascii=False)
    for n in re.findall(r'\d+', cl):
        if n not in dump and not (n == '1' and ('"min": 1' in dump or '"max": 1' in dump or '"n": 1' in dump)):
            return f'文中の数 {n} が手順にありません'
    if 'CONTAINED' not in dump and '含まれる' in cl:
        return '「含まれる」の数は "$CONTAINED" です'
    return None


def ask_steps(cl, cx, log):
    h = hints(cl, cx)
    exs = EXAMPLES + [(b['clause'], b.get('hints', {}), b['steps']) for b in similar(cl)]
    ex = '\n'.join(f'文：{a}\nヒント：{json.dumps(b, ensure_ascii=False)}\n答え：{json.dumps({"steps": c, "cannot": False}, ensure_ascii=False)}' for a, b, c in exs)
    base = f"""あなたはデュエル・マスターズのルール処理係。カードの能力の1文を、ゲームエンジンの手順に変換する。
{SPEC}

例：
{ex}

文：{cl}
ヒント：{json.dumps(h, ensure_ascii=False)}
手順で書けない文なら {{"steps":[],"cannot":true}}。JSON だけで答える。"""
    p, err = base, None
    for tries in range(3):
        try:
            res = ollama(p, SCHEMA)
        except Exception as e:
            log.append(f'  Qwen エラー: {e!r}')
            return None
        if res.get('cannot'):
            log.append(f'  Qwen: 書けない「{cl}」')
            return None
        steps = tidy(res.get('steps') or [], cx, cl)
        err = check(cl, steps, cx)
        log.append(f'  Qwen[{tries + 1}] 「{cl}」→ {json.dumps(steps, ensure_ascii=False)}' + (f'  ×{err}' if err else ''))
        if not err:
            for st in steps:
                if st.get('do') in ('choose', 'filter'):
                    v = st['as']
                    if v not in cx.picked:
                        cx.picked.append(v)
                        if st.get('from') == '$L':
                            cx.from_look.append(v)
                if st.get('do') == 'look':
                    cx.look = '$' + st['as']
            return steps
        p = base + f'\n\n前の答え：{json.dumps({"steps": steps}, ensure_ascii=False)}\nこの答えは間違い：{err}\n直して、JSON だけで答える。'
    return None


def cond_rule(c):
    if c == 'それが進化クリーチャー':
        return [{'self': 'evolved'}]
    m = re.fullmatch(r'自分の(マナゾーン|墓地|手札)にあるカードがすべて(.+?)・?カード', c)
    if m:
        return [{'who': 'me', 'zone': {'マナゾーン': 'mana', '墓地': 'grave', '手札': 'hand'}[m.group(1)], 'filter': {'notRace': m.group(2)}, 'op': '==', 'n': 0}]
    m = re.fullmatch(r'自分の他の(.+?)が(ある|あれ)?', c)
    if m:
        f = parse_filter(m.group(1))
        if f:
            return [{'who': 'me', 'zone': 'battle', 'filter': f, 'op': '>=', 'n': 2}]
    m = re.fullmatch(r'自分のマナゾーンに(ある)?カードが(\d+)枚以上(ある|あれ)?', c)
    if m:
        return [{'who': 'me', 'zone': 'mana', 'op': '>=', 'n': int(m.group(2))}]
    return None


def ask_cond(c, log):
    if c.startswith('それ'):  # 見たカードなどについての条件は、エンジンの条件で表せない
        log.append(f'  ×条件で表せない「{c}」')
        return None
    p = f"""あなたはデュエル・マスターズのルール処理係。能力の条件（「〜なら」「〜あれば」の前の部分）を、ゲームエンジンの条件にする。
{COND_SPEC}

条件：{c}
JSON だけで答える：{{"conds":[...]}}"""
    try:
        res = ollama(p, COND_SCHEMA)
    except Exception as e:
        log.append(f'  Qwen エラー: {e!r}')
        return None
    conds = res.get('conds') or []
    log.append(f'  Qwen 条件「{c}」→ {json.dumps(conds, ensure_ascii=False)}')
    return conds or None


# ---------------- 4. 組み立て ----------------
def convert(body, log):
    cx = Ctx()
    steps, i = [], 0
    cls = clauses(body)
    out_stack = [steps]
    cur = 0
    for k, cl in cls:
        if k != cur:  # 「〜なら、」の中はその文の終わりまで
            out_stack, cur = [steps], k
        m = re.match(r'^(.+?)(なら|あれば)、(.+)$', cl)
        if m and '選' not in m.group(1):
            c = m.group(1)
            conds = cond_rule(c)
            if conds:
                log.append(f'  Python 条件「{c}」→ {json.dumps(conds, ensure_ascii=False)}')
            elif not RULES_ONLY:
                conds = ask_cond(c, log)
            if not conds:
                log.append(f'  ×条件を読めない「{c}」')
                return None
            node = {'do': 'if', 'conds': conds, 'steps': []}
            out_stack[-1].append(node)
            out_stack.append(node['steps'])  # この文の残りは条件の中（その後に続く文も中に入れる）
            cl = m.group(3)
        sub = [cl]
        mm = re.fullmatch(r'(相手の.+?を\d+(?:体|つ)選び)、(このターン、|次の自分のターンのはじめまで、)?(そのクリーチャー|それ)(.+)', cl)
        if mm and rule(cl, Ctx()) is None:  # 「選び、このターン、そのクリーチャーの〜」は「選ぶ」と後ろに分ける
            sub = [mm.group(1).replace('選び', '選ぶ'), (mm.group(2) or '') + mm.group(3) + mm.group(4)]
        for s in sub:
            if CANNOT.search(s):
                log.append(f'  ×手順で表せない言い回し「{s}」')
                return None
            r = rule(s, cx)
            if r is not None:
                log.append(f'  Python「{s}」→ {json.dumps(r, ensure_ascii=False)}')
            elif RULES_ONLY:
                log.append(f'  ×決まり文句にない「{s}」')
                return None
            else:
                r = ask_steps(s, cx, log)
                if r is None:
                    return None
            optional = bool(re.search(r'てもよい$', s)) and not any(x.get('do') in ('choose', 'optional', 'mode') for x in r)
            out_stack[-1].extend([{'do': 'optional', 'q': s + '？', 'steps': r}] if optional else r)
    steps = fix_order(steps, cx)
    flat = json.dumps(steps, ensure_ascii=False)
    for st in walk(steps):  # 選んだのに何もしないカードがあれば、文の切り出しが途中で終わっている
        if st.get('do') in ('choose', 'filter') and flat.count('"$' + st['as'] + '"') < 1:
            log.append(f'  ×選んだ ${st["as"]} を使う手順がない（能力の切り出しが途中まで）')
            return None
    return steps


def walk(steps):
    for st in steps:
        yield st
        for k in ('steps', 'else'):
            if isinstance(st.get(k), list):
                yield from walk(st[k])
        for o in st.get('options', []) or []:
            yield from walk(o.get('steps', []))


def fix_order(steps, cx):
    """エンジンの決まり：①「残りを山札の下に」は、見たカードから出す（put）より先にする（出た時の能力で引くカードが変わらないように）。
    ②手札・マナから出すクリーチャーは進化ではないもの（進化は進化元がいる）。"""
    src = {st['as']: st.get('from') for st in steps if st.get('do') in ('choose', 'filter')}
    for st in steps:
        if st.get('do') == 'choose' and st.get('from') in ('$MY_HAND', '$MY_MANA'):
            if any(x.get('do') == 'put' and x.get('ids') == '$' + st['as'] for x in steps):
                st.setdefault('filter', {})['nonEvolution'] = True
                st['filter'].setdefault('type', 'クリーチャー')
    rest = [i for i, st in enumerate(steps) if st.get('do') == 'move' and st.get('ids') == '$L' and st.get('to') == 'deckBottom']
    puts = [i for i, st in enumerate(steps) if st.get('do') == 'put' and src.get(str(st.get('ids'))[1:]) == '$L']
    if rest and puts and puts[0] < rest[0]:
        st = steps.pop(rest[0]); steps.insert(puts[0], st)
    return steps


def main():
    cards = json.loads(pathlib.Path(ARGS[0]).read_text(encoding='utf-8'))
    out_path = pathlib.Path(ARGS[1])
    progs, report = [], []
    t0 = time.time()
    for c in cards:
        abl, skipped = abilities(c)
        for side, why in skipped:
            report.append(f'- {c["name"]}：{why}')
        CUR['id'] = c['id']
        for ev, side, body in abl:
            log = []
            steps = convert(body, log)
            ok = steps is not None
            report.append(f'{"○" if ok else "×"} {c["name"]} {ev}（{"。".join(body)}）')
            report.extend(log)
            if ok:
                p = {'id': c['id'], 'name': c['name'], 'ev': ev, 'steps': steps}
                if ev == 'spell':
                    p['side'] = side
                progs.append(p)
        print(f'{c["name"]} 済（{len(progs)} 能力）', flush=True)
        out_path.write_text(json.dumps(progs, ensure_ascii=False, indent=1), encoding='utf-8')
        out_path.with_suffix('.log.txt').write_text('\n'.join(report), encoding='utf-8')
    report.append(f'\nモデル {MODEL if not RULES_ONLY else "なし（Pythonのみ）"}、Qwen 呼び出し {CALLS["n"]} 回・{CALLS["sec"]:.0f} 秒、全体 {time.time() - t0:.0f} 秒、手順にできた能力 {len(progs)}')
    out_path.with_suffix('.log.txt').write_text('\n'.join(report), encoding='utf-8')
    print(report[-1])


if __name__ == '__main__':
    main()
