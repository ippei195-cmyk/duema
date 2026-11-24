# 自己対戦学習（IPPEIのPC用）

対戦卓（index.html）とは独立した、学習専用のフォルダです。ここを変えても対戦卓や保存済みデッキには影響しません。

- `selfplay.exe` … 自己対戦で学習データを作る
- `arena.exe` … AI同士を対戦させて勝率を測る
- `train.py` … RTX 4070 でネットを学習
- `run.py` … 自己対戦 → 学習 → 評価 → 交代 を止めるまで繰り返す

使い方：`C:\dm-ai\.venv\Scripts\python.exe run.py`。止めるときは同じフォルダに `STOP` というファイルを置く（今の世代が終わったら止まる）。
記録は `log.csv`（世代ごと）・`elo.csv`（レート）・`PASSED.txt`（フェーズ2版に80%以上で作られる）。
