# PC版の対戦卓の控え（2026-11-26）

C:\dm-ai の PC版（play.bat）を調べるためのコピーです。このフォルダは動かすためのものではありません。

- play.bat：`.venv\Scripts\python.exe -m dmai.server` を起動する
- server.py：http://127.0.0.1:8765/ で、`/` に web/sim_local.html、`/bridge.js`・`/patches.js`・`/card_thumbs.json`・`/thumb/*.jpg` を返す。`/api/*` は学習モデル（PPO）・記録・Qwen
- build_sim.py：起動のたびに sim/simulator.html から web/sim_local.html を作る。最初の `<script>` の前に `<script src="/bridge.js">`、`</body>` の前に `<script src="/patches.js">` を差し込み、PATCHES の文字列置換をする
- simulator.html：sim/simulator.html（対戦卓の本体、変更しない決まり）
- patches.js：web/patches.js（追加パッチ。末尾に学習AI（第278世代）・学習用2デッキ・旧プリセット除去）
- bridge.js：web/bridge.js（window.claude をローカル Python に差し替え）
- filelist.txt：C:\dm-ai\web の一覧
