import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "data"
HUMAN = DATA / "human"
MODELS = ROOT / "models"
WEB = ROOT / "web"
SIM_SRC = ROOT / "sim" / "simulator.html"
SIM_OUT = WEB / "sim_local.html"
CARD_LIST = ROOT / "fyi" / "デュエマ全カード一覧.xlsx"
CARD_THUMBS = WEB / "card_thumbs.json"   # card id -> official thumbnail name, only where it differs from the simulator's
THUMB_CACHE = ROOT / "cache" / "thumb"

DEFAULTS = {
    "port": 8765,
    "llm": {
        "enabled": True,
        "model": "qwen3:8b",
        "host": "http://127.0.0.1:11434",
        "effects": True,
        "persona": "あなたはデュエル・マスターズの対戦相手。落ち着いた中級者で、少しだけ負けず嫌い。",
    },
    "play": {"model": "current.pt", "temperature": 0.7},
    "model": {"d": 128, "layers": 3, "heads": 4},
    "bc": {"epochs": 20, "batch": 256, "lr": 3e-4, "value_weight": 0.5},
    "selfplay": {
        "pages": 4, "games_per_iter": 32, "max_turns": 60, "headless": True,
        "ppo_epochs": 3, "minibatch": 256, "lr": 1e-4, "clip": 0.2,
        "kl_coef": 0.05, "ent_coef": 0.01, "vf_coef": 0.5,
        "snapshot_every": 5, "pool_size": 10, "p_vs_current": 0.4, "p_vs_bot": 0.2,
        "deckA": "", "deckB": "", "game_timeout_sec": 240,
    },
}


def _merge(base, over):
    out = dict(base)
    for k, v in over.items():
        out[k] = _merge(base[k], v) if isinstance(v, dict) and isinstance(base.get(k), dict) else v
    return out


def load():
    p = ROOT / "config.json"
    cfg = DEFAULTS
    if p.exists():
        cfg = _merge(DEFAULTS, json.loads(p.read_text(encoding="utf-8")))
    for d in (HUMAN, MODELS):
        d.mkdir(parents=True, exist_ok=True)
    return cfg
