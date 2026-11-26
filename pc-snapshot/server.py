"""Local server: serves the simulator with the bridge, answers the AI's moves, records the human's games, and
relays commentary to the LLM.  python -m dmai.server  → opens http://127.0.0.1:8765/ in the browser."""
import json
import re
import threading
import time
import webbrowser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import unquote

from . import card_images, config
from .build_sim import build
from .llm import LLM


def safe_id(s):
    return re.sub(r"[^A-Za-z0-9_-]", "", str(s))[:40] or "unknown"


class App:
    def __init__(self, cfg, selfplay=None):
        self.cfg = cfg
        self.llm = LLM(cfg)
        self.sp = selfplay
        self.file_lock = threading.Lock()
        self.device, self.policy = "なし", None
        if selfplay is None:
            # PyTorch is optional here: without it the simulator's simple AI plays, and games are still recorded
            try:
                from .policy import Policy, pick_device
                self.device = pick_device()
                self.policy = Policy(self.device)
            except (ImportError, OSError) as e:
                print(f"PyTorch を読み込めませんでした（{e}）。相手は簡易AIで動きます。記録と会話は使えます。")

    def handle(self, path, body):
        if path == "/api/act":
            if self.sp:
                return self.sp.act(body)
            net = self.policy.get(self.cfg["play"]["model"]) if self.policy else None
            if net is None:
                return {"fallback": True}  # no trained model yet → the simulator's built-in simple AI
            i, v, _ = self.policy.act(net, body["obs"], body["acts"], self.cfg["play"]["temperature"])
            return {"i": i, "v": v}
        if path == "/api/log":
            rec = {"game": safe_id(body["game"]), "obs": body["obs"], "acts": body["acts"], "a": body["a"], "decks": body.get("decks"), "t": time.time()}
            self._append(config.HUMAN / f"{rec['game']}.jsonl", rec)
            return {"ok": True}
        if path == "/api/game_end":
            self._append(config.HUMAN / "results.jsonl", {"game": safe_id(body["game"]), "winner": body["winner"], "decks": body.get("decks"), "t": time.time()})
            return {"ok": True}
        if path == "/api/comment":
            return {"text": self.llm.comment(body.get("lines", []), body.get("v0"), body.get("v1"), body.get("deck", ""))}
        if path == "/api/llm_json":
            if body.get("selfplay") or (body.get("purpose") == "effect" and not self.cfg["llm"]["effects"]):
                return {}
            return self.llm.json_task(body["prompt"])
        if path == "/api/sp/next" and self.sp:
            return self.sp.next(body)
        if path == "/api/sp/end" and self.sp:
            return self.sp.end(body)
        return {"error": "unknown " + path}

    def _append(self, path, rec):
        with self.file_lock, open(path, "a", encoding="utf-8") as f:
            f.write(json.dumps(rec, ensure_ascii=False) + "\n")


def make_handler(app):
    class H(BaseHTTPRequestHandler):
        def log_message(self, *a):
            pass

        def _send(self, code, data, ctype, cache="no-store"):
            self.send_response(code)
            self.send_header("Content-Type", ctype)
            self.send_header("Cache-Control", cache)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self):
            p = self.path.split("?")[0]
            js = "text/javascript; charset=utf-8"
            files = {"/": (config.SIM_OUT, "text/html; charset=utf-8"), "/bridge.js": (config.WEB / "bridge.js", js), "/patches.js": (config.WEB / "patches.js", js),
                     "/card_thumbs.json": (config.CARD_THUMBS, "application/json; charset=utf-8"), "/dojo.jpg": (config.WEB / "dojo.jpg", "image/jpeg")}
            if p.startswith("/thumb/") and p.endswith(".jpg"):
                data, code = card_images.thumb(unquote(p[7:-4]))
                if data:
                    self._send(200, data, "image/jpeg", "max-age=2592000")
                else:
                    self._send(code, b"no picture", "text/plain")
            elif p in files and files[p][0].exists():
                self._send(200, files[p][0].read_bytes(), files[p][1])
            else:
                self._send(404, b"not found", "text/plain")

        def do_POST(self):
            try:
                n = int(self.headers.get("Content-Length", 0))
                body = json.loads(self.rfile.read(n).decode("utf-8") or "{}")
                out = app.handle(self.path, body)
                self._send(200, json.dumps(out, ensure_ascii=False).encode("utf-8"), "application/json")
            except Exception as e:
                import traceback
                traceback.print_exc()
                self._send(500, json.dumps({"error": str(e)}).encode("utf-8"), "application/json")

    return H


def serve(app, port):
    srv = ThreadingHTTPServer(("127.0.0.1", port), make_handler(app))
    srv.daemon_threads = True
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv


def main():
    cfg = config.load()
    build()
    card_images.ensure_map()
    app = App(cfg)
    port = cfg["port"]
    serve(app, port)
    have = app.policy is not None and (config.MODELS / cfg["play"]["model"]).exists()
    print(f"対戦卓: http://127.0.0.1:{port}/   （AI: {'学習モデル ' + cfg['play']['model'] if have else 'まだモデルがないので簡易AI'} / 実行環境: {app.device}）")
    print("設定で相手を「ローカルAI（学習モデル）」にして遊ぶと、あなたの手が学習データとして data/human に保存されます。")
    print("終了するときはこの画面で Ctrl+C を押してください。")
    webbrowser.open(f"http://127.0.0.1:{port}/")
    try:
        while True:
            time.sleep(3600)
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
