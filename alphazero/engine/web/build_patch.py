"""dmaz_patch.js に wasm とカード一覧を埋め込み、index.html の </body> の前に <script> として入れる。
python build_patch.py <dmengine.wasm> <cards.json> <index.html>（上書き） [net.bin net.json]
net.bin を渡すと重みもページに埋め込む（スマホ版など、別ファイルを読めない所用）
--pc：相手が「ローカルAI（学習モデル）」の時も学習AIにする（PCの対戦卓用）
--js-only：<script> を入れずに、JS だけを第3引数のファイルに書き出す（patches.js の末尾に足す用）"""
import base64, json, sys, re
args = [a for a in sys.argv[1:] if not a.startswith('--')]
opts = [a for a in sys.argv[1:] if a.startswith('--')]
wasm, cards, html = args[:3]
sys.argv = [sys.argv[0]] + args
modes = ['bot', 'claude'] if '--pc' in opts else ['bot']
js = open(__file__.replace('build_patch.py', 'dmaz_patch.js'), encoding='utf-8').read()
js = js.replace('__CARDS__', json.dumps(json.load(open(cards, encoding='utf-8')), ensure_ascii=False))
js = js.replace('__WASM__', base64.b64encode(open(wasm, 'rb').read()).decode())
if len(sys.argv) > 5:
    js = js.replace('__NET__', base64.b64encode(open(sys.argv[4], 'rb').read()).decode())
    js = js.replace('__NETINFO__', json.dumps(json.load(open(sys.argv[5], encoding='utf-8'))))
else:
    js = js.replace('__NET__', '').replace('__NETINFO__', 'null')
js = js.replace('__MODES__', json.dumps(modes))
if '--js-only' in opts:  # PC版：patches.js の末尾に足す JS だけを書き出す
    open(html, 'w', encoding='utf-8').write(js)
    print('ok js', len(js))
    sys.exit(0)
h = open(html, encoding='utf-8').read()
start = h.find('<script id="dmaz">')
if start >= 0:
    end = h.index('</script>', start) + len('</script>')
    h = h[:start] + h[end:]
    h = h.replace('\n\n\n</body>', '\n\n</body>')
k = h.rindex('</body>')
h = h[:k] + '<script id="dmaz">\n' + js + '</script>\n\n' + h[k:]
open(html, 'w', encoding='utf-8').write(h)
print('ok', len(js))
