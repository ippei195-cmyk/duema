# 学習AIの切り替え（80%を超えたら）

一平さんの指示（2026-11-25）：フェーズ2版への勝率が80%を超えたら、ネット・PC・スマホ版すべてを学習AIに切り替える。

作り方はすべて `engine/web/build_patch.py <dmengine.wasm> <cards.json> <出力> [net.bin net.json] [--pc] [--js-only]`。

| 版 | 置き場所 | 作り方 | 切り替え方 |
|---|---|---|---|
| GitHub Pages | ippei195-cmyk/duema | PR #5（パッチ）＋#6（alphazero/net.bin） | #6 の重みを最新にして、#5・#6 をマージ |
| スマホ版 | Artifact SZgfsFwFJsT84jsFY5gUpW | 読んだ HTML に net.bin を埋め込んで build | 同じ URL に再公開（phone/artifact-dmaz-genN.html） |
| Netlify | duema-taisentaku-ippei.netlify.app | 今の公開版 index.html に net.bin を埋め込んで build | デプロイ（元の公開版を残してから） |
| PC | C:\dm-ai\web\patches.js | `--pc --js-only` で JS を作る（pc/dmaz_pc-genN.js） | patches.js をバックアップして末尾に足す。相手が「簡易AI」「ローカルAI（学習モデル）」のどちらでも学習AIになる |

どの版も、両方のデッキが学習した3デッキのカードだけの時に動き、それ以外は今までのAI。保存済みデッキには触らない。
