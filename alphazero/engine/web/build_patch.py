"""dmaz_patch.js に wasm とカード一覧を埋め込み、index.html の </body> の前に <script> として入れる。
python build_patch.py <dmengine.wasm> <cards.json> <index.html>（上書き）"""
import base64, json, sys, re
wasm, cards, html = sys.argv[1:4]
js = open(__file__.replace('build_patch.py', 'dmaz_patch.js'), encoding='utf-8').read()
js = js.replace('__CARDS__', json.dumps(json.load(open(cards, encoding='utf-8')), ensure_ascii=False))
js = js.replace('__WASM__', base64.b64encode(open(wasm, 'rb').read()).decode())
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
