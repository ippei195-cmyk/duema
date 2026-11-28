"""カードの効果文からキーワード能力を読み取り、学習エンジンの持ち方と比べて「反映漏れ」を探す。

キーワード能力（ブロッカー・スレイヤー・J・O・E など）はカードごとに書くのではなく、
テキストから読み取って共通の処理に回す。そのための第一歩として、次を出す：
  1. 学習に使うカードそれぞれが持つキーワード（テキストから読み取り）
  2. エンジンの kw ビット（ブロッカー・SA・マッハファイター・S・トリガー・G・ストライク・ジャストダイバー・EXライフ・W/T・ブレイカー）と食い違うカード
  3. ビットのないキーワードを持つのに、エンジンのコード（effects.rs・engine.rs）にそのカードが一度も出てこないもの＝処理されていない
使い方: python3 textfx/keywords.py カードのテキスト... > 報告.md
  テキストは cards.json（[{id,name,text}]）か、「## 名前 (id)」の次の行から本文が並ぶ .md。
"""
import json, re, sys, pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
cards_rs = (ROOT / 'src' / 'cards.rs').read_text(encoding='utf-8')
code = (ROOT / 'src' / 'effects.rs').read_text(encoding='utf-8') + (ROOT / 'src' / 'engine.rs').read_text(encoding='utf-8')
IDS = re.findall(r'CardDef \{ id: "([^"]+)"', cards_rs)
VARIANTS = [v.strip() for v in re.sub(r'//[^\n]*', '', re.search(r'pub enum C \{([^}]*)\}', cards_rs).group(1)).replace('\n', ' ').split(',') if v.strip()]
ID2C = dict(zip(IDS, VARIANTS))
ID2KW = {}
for m in re.finditer(r'CardDef \{ id: "([^"]+)", main: side!\((.*?)\), spell: (None|Some\(side!\((.*?)\)\))', cards_rs):
    def kw(s):
        return s.rsplit(',', 1)[1].strip() if s else ''
    ID2KW[m.group(1)] = kw(m.group(2)) + ' ' + kw(m.group(4))
for m in re.finditer(r'CardDef \{ id: "([^"]+)".*?hyper: Some\(\(\d+, ([^)]*)\)\)', cards_rs):  # ハイパーモード側のキーワード
    ID2KW[m.group(1)] = ID2KW.get(m.group(1), '') + ' ' + m.group(2)

BITS = {'ブロッカー': 'K_BLOCKER', 'スピードアタッカー': 'K_SA', 'マッハファイター': 'K_MACH', 'S・トリガー': 'K_STRIG', 'G・ストライク': 'K_GSTRIKE',
        'ジャストダイバー': 'K_JUSTDIVER', 'EXライフ': 'K_EXLIFE', 'W・ブレイカー': 'K_WB', 'T・ブレイカー': 'K_TB'}
OTHERS = json.loads(pathlib.Path('/mnt/project-files/keywords/keyword-reminders.json').read_text(encoding='utf-8')) \
    if pathlib.Path('/mnt/project-files/keywords/keyword-reminders.json').exists() else {}
EXTRA = ['J・O・E', 'Zラッシュ', '革命チェンジ', 'NEO進化', 'G-NEO進化', '超魂X', '超魂レイド', 'ニンジャ・ストライク', 'ウラ・ニンジャ・ストライク',
         'Q・ブレイカー', 'スレイヤー', 'G・ゼロ', 'S・トリガー・プラス', '水晶チャージャー', '逆転撃', 'アビスラッシュ']
ALL = sorted(set(BITS) | set(OTHERS) | set(EXTRA), key=len, reverse=True)
ZEN = str.maketrans('０１２３４５６７８９', '0123456789')


def load(paths):
    out = {}
    for p in paths:
        t = pathlib.Path(p).read_text(encoding='utf-8')
        if p.endswith('.json'):
            for c in json.loads(t):
                if c.get('text'):
                    out[c['id']] = (c['name'], c['text'])
        else:
            # 「## 名前 (id) 種類…」の次の行から本文。種類が別の行（「クリーチャー / 火 / コスト3 …」）なら、その行は飛ばす
            for m in re.finditer(r'^##+ (.+?) \(([^)]+)\)[^\n]*\n(.*?)(?=^#|\Z)', t, re.S | re.M):
                lines = m.group(3).strip().split('\n')
                if lines and re.search(r' / .*コスト', lines[0]):
                    lines = lines[1:]
                body = '\n'.join(lines).strip()
                if body:
                    out[m.group(2)] = (m.group(1), body)
    return out


def read_keywords(text):
    """持っているキーワード と 他に与えるキーワード（「…」を与える）を分けて返す。"""
    t = text.translate(ZEN)
    granted = set()
    for q in re.findall(r'「([^」]*)」を(与え|得る)', t):
        for k in ALL:
            if q[0].startswith(k):
                granted.add(k)
    t = re.sub(r'「[^」]*」', '', t)
    prev = None
    while prev != t:  # 説明文（括弧の中）は読まない
        prev = t
        t = re.sub(r'（[^（）]*）|\([^()]*\)', '', t)
    own, rest = set(), t
    for k in ALL:
        if k in rest:
            own.add(k)
            rest = rest.replace(k, '')
    return own, granted


def main():
    texts = load(sys.argv[1:])
    rows, miss, unhandled, counts = [], [], [], {}
    for cid in IDS:
        if cid not in texts:
            rows.append(f'| {cid} | （テキストなし） | | |')
            continue
        name, text = texts[cid]
        own, granted = read_keywords(text)
        for k in own:
            counts.setdefault(k, []).append(name)
        bits = ID2KW.get(cid, '')
        for k, b in BITS.items():
            has_text, has_bit = k in own or (k == 'S・トリガー' and 'S・トリガー・プラス' in own), b in bits
            if has_text != has_bit:
                miss.append(f'- {name}（{ID2C[cid]}）：{k} が {"テキストにあるのにビットがない" if has_text else "ビットがあるのにテキストにない"}')
        v = ID2C[cid]
        nonbit = sorted(k for k in own if k not in BITS)
        if nonbit and not re.search(r'C::' + re.escape(v) + r'\b', code):
            unhandled.append(f'- {name}（{v}）：{"・".join(nonbit)}')
        rows.append(f'| {name} | {"・".join(sorted(own)) or "-"} | {"・".join(sorted(granted)) or "-"} | {bits.strip() or "0"} |')
    print('# キーワード能力の読み取りと、エンジンとの食い違い\n')
    print(f'カード {len(IDS)} 枚、テキストあり {sum(1 for c in IDS if c in texts)} 枚。\n')
    print('## ビットの食い違い（反映漏れ・付け間違いの候補）\n')
    print('\n'.join(miss) or '- なし')
    print('\n## ビットのないキーワードを持つのに、エンジンのコードにカードが一度も出てこない（処理されていない）\n')
    print('\n'.join(unhandled) or '- なし')
    print('\n## キーワードごとの枚数（学習のカードで）\n')
    for k, v in sorted(counts.items(), key=lambda x: -len(x[1])):
        print(f'- {k}（{"ビットあり" if k in BITS else "共通処理なし"}）{len(v)}枚：{"、".join(v)}')
    print('\n## カードごと\n\n| カード | 持っている | 与える | エンジンのビット |\n|---|---|---|---|')
    print('\n'.join(rows))


if __name__ == '__main__':
    main()
