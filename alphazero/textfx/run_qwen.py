"""学習エンジン用：カードの効果文をローカルAI（Ollama の qwen3:8b）で「手順(steps)」に一度だけ変換して保存する実験。
使い方: python run_qwen.py [cards.json] [out.json] [モデル名（既定 qwen3:8b）]   （Ollama が 127.0.0.1:11434 で動いていること）
対局中には使わない。結果は人とテストで確かめてから使う。"""
import json, sys, time, urllib.request, pathlib

HERE = pathlib.Path(__file__).parent
cards = json.loads((HERE / (sys.argv[1] if len(sys.argv) > 1 else 'cards.json')).read_text(encoding='utf-8'))
out_path = HERE / (sys.argv[2] if len(sys.argv) > 2 else 'qwen_out.json')
MODEL = sys.argv[3] if len(sys.argv) > 3 else 'qwen3:8b'
spec = (HERE / 'step_spec.txt').read_text(encoding='utf-8')
spec = spec.replace('idは状況に出ているカードid。', 'カードidは使わない（どの対局でも使える手順にする）。')
EVS = 'etb（出た時）, spell（呪文を唱えた時）, attack（攻撃する時）, destroyed（破壊された時）, turnStart（自分のターンのはじめ）, turnEnd（自分のターンの終わり）, allyEnter（自分の他のクリーチャーが出た時）, oppEnter（相手のクリーチャーが出た時）, shieldPlaced（自分のシールドゾーンにカードが置かれた時）, gstrike（G・ストライク）, strigger（S・トリガーで使った時）'

def prompt(c):
    return f"""あなたはデュエル・マスターズのルール処理係。下のカードの能力を、ゲームエンジンが実行できる手順(steps)に変換する。
カード：《{c['name']}》（{c['type']}／{c['civ']}／コスト{c['cost']}／パワー{c['power'] or '-'}／種族 {c['race'] or '-'}）
カードテキスト：{c['text']}

やること：
- 「〜した時」に一度だけ処理する能力（誘発能力）と、呪文の効果を、それぞれ1つの ability にする。ev は次から選ぶ：{EVS}
- 「各ターン1回」「召喚によって出た時だけ」などの制限は once_per_turn / summon_only に書く。
- 手順で書けない能力（コスト軽減、置換効果「かわりに」、ずっと続く効果、キーワード能力、出し方の条件）は abilities に入れず、static に原文の一部を短く入れる。
- 1つの能力の中に手順で書けない部分があれば、その ability は入れず static に回す（あいまいなまま手順にしない）。

{spec}

JSONだけで答える：{{"abilities":[{{"ev":"etb","once_per_turn":false,"summon_only":false,"text":"元の文","steps":[...]}}],"static":["…"]}}"""

def ask(p):
    body = json.dumps({'model': MODEL, 'messages': [{'role': 'user', 'content': p}], 'stream': False, 'format': 'json',
                       'think': False, 'options': {'temperature': 0, 'num_ctx': 8192}, 'keep_alive': '10m'}).encode()
    req = urllib.request.Request('http://127.0.0.1:11434/api/chat', data=body, headers={'Content-Type': 'application/json'})
    with urllib.request.urlopen(req, timeout=600) as r:
        return json.loads(r.read())['message']['content']

res = []
for i, c in enumerate(cards):
    t = time.time()
    try:
        raw = ask(prompt(c)); data = json.loads(raw); err = None
    except Exception as e:
        raw, data, err = None, None, repr(e)
    res.append({'id': c['id'], 'name': c['name'], 'sec': round(time.time() - t, 1), 'result': data, 'raw': raw if data is None else None, 'error': err})
    print(f"{i+1}/{len(cards)} {c['name']} {res[-1]['sec']}s {'ERR '+err if err else ''}", flush=True)
    out_path.write_text(json.dumps(res, ensure_ascii=False, indent=1), encoding='utf-8')
print('done', out_path)
