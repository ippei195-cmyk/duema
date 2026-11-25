/* ---------- 学習AI（AlphaZero式、2026-11-25） ---------- */
// 自己対戦で学習したAI（alphazero ブランチの Rust エンジンを WebAssembly にしたもの）が、相手の手番の
// 「チャージ・カードを使う・攻撃・ターン終了」を1つずつ決める。効果の中の選択（どれを選ぶか等）は今までの簡易AIのまま。
//   ・両方のデッキが学習した3デッキ（リースバイク・ハッスル・キャッスル・ジョーカーズ）のカードだけでできている時に使う。
//     ほかのデッキの時・Claude/手動/オンライン対戦・自己対戦の時は今までどおり。
//   ・あなたの手札・山札・裏向きシールドは覗き見しない（先読みのたびに混ぜて配り直す）。
//   ・重みは alphazero/net.bin（無ければ重みなしの先読みで動く）。保存済みデッキ・記録には触らない。
// window.DMAZ：{ on(), set(bool), cfg, stats, last }。ツール欄に「学習AI：ON/OFF」。設定は localStorage 'dmai-az'。
(() => {
  const wrap = (name, f) => { const orig = window[name]; if (typeof orig !== 'function') { console.warn('dmaz patch: missing ' + name); return; } window[name] = f(orig); };
  const CARDS = __CARDS__;
  const WASM_B64 = "__WASM__";
  const NET_B64 = "__NET__";
  const NET_INFO = __NETINFO__; // 重みを埋め込んだ版（スマホ版など、別ファイルを読めない所）。空なら netUrl から読む
  const CFG = { modes: __MODES__, sims: 1200, netUrl: 'alphazero/net.bin', infoUrl: 'alphazero/net.json' };
  const KEY = 'dmai-az';
  let on = true; try { on = localStorage.getItem(KEY) !== 'off'; } catch (e) {}
  const stats = { calls: 0, errors: 0, ms: 0, fallback: 0, invalid: [] };
  let ex = null, netInfo = null, last = null, broken = false;
  const nf = s => String(s || '').normalize('NFKC').replace(/\s+/g, '');
  const byId = new Map(CARDS.map(([id], i) => [id, i]));
  const byName = new Map(CARDS.map(([, n], i) => [nf(n), i]));
  const idxCache = new Map();
  const idx = cid => {
    if (idxCache.has(cid)) return idxCache.get(cid);
    let i = byId.has(cid) ? byId.get(cid) : -1;
    if (i < 0) { try { const d = DB[cid]; const n = nf(d.sides[0].name); if (byName.has(n)) i = byName.get(n); } catch (e) {} }
    idxCache.set(cid, i); return i;
  };

  async function init() {
    try {
      const bin = Uint8Array.from(atob(WASM_B64), ch => ch.charCodeAt(0));
      const { instance } = await WebAssembly.instantiate(bin, {});
      ex = instance.exports;
    } catch (e) { console.warn('dmaz wasm', e); broken = true; return; }
    try {
      let b = null;
      if (NET_B64) b = Uint8Array.from(atob(NET_B64), ch => ch.charCodeAt(0));
      else { const r = await fetch(CFG.netUrl, { cache: 'no-cache' }); if (r.ok) b = new Uint8Array(await r.arrayBuffer()); }
      if (b) {
        const p = ex.dmaz_alloc(b.length); new Uint8Array(ex.memory.buffer, p, b.length).set(b);
        if (ex.dmaz_load(p, b.length) === 1) {
          netInfo = { bytes: b.length };
          if (NET_INFO) netInfo = Object.assign(netInfo, NET_INFO);
          else try { const j = await fetch(CFG.infoUrl, { cache: 'no-cache' }); if (j.ok) netInfo = Object.assign(netInfo, await j.json()); } catch (e) {}
        }
      }
    } catch (e) { /* 重みなしでも動く */ }
  }
  const ready = init();

  /* ---- 盤面を数字の並びにする（AI＝op をプレイヤー0に） ---- */
  const ZONES = ['hand', 'mana', 'battle', 'shield', 'grave', 'deck'];
  function encode() {
    const uids = []; const tid = u => { uids.push(u); return uids.length - 1; };
    const out = [1, S.turn | 0, (S.charged && S.active === 'op') ? 1 : 0, CFG.sims, (Math.random() * 2147483647) | 0];
    for (const p of ['op', 'me']) {
      for (const z of ZONES) {
        let arr = (S.players[p][z] || []).slice();
        if (z === 'deck') arr.reverse(); // エンジンは末尾が山札の上
        out.push(arr.length);
        for (const u of arr) {
          const c = S.cards[u]; if (!c) return null;
          const ci = idx(c.cid); if (ci < 0) return null;
          let fl = 0;
          if (c.tapped) fl |= 1;
          if (c.sick) fl |= 2;
          if (c.faceUp || (c.m && c.m.fu)) fl |= 4;
          if (c.m && c.m.noAttack) fl |= 16;
          const exl = z === 'battle' && c.ex && S.cards[c.ex] ? c.ex : null;
          if (exl) fl |= 8;
          out.push(ci, tid(u), fl);
          if (exl) out.push(-2); // あとで埋める
          const exPos = exl ? out.length - 1 : -1;
          const under = (z === 'battle' || z === 'shield') ? (c.under || []).filter(x => S.cards[x]) : [];
          out.push(under.length);
          for (const x of under) { const k = idx(S.cards[x].cid); if (k < 0) return null; out.push(k, tid(x)); }
          if (exl) out[exPos] = { ex: exl };
        }
      }
    }
    // EXライフのシールドの番号を埋める
    for (let i = 0; i < out.length; i++) if (out[i] && typeof out[i] === 'object') { const k = uids.indexOf(out[i].ex); out[i] = k; }
    return { arr: Int32Array.from(out), uids };
  }

  function supported() {
    if (broken || !ex || !on || !S || S.winner || S.demo || S.active !== 'op') return false;
    let mode = 'bot'; try { mode = oppMode(); } catch (e) {}
    if (!CFG.modes.includes(mode) || (typeof NET !== 'undefined' && NET.on)) return false;
    if (window.DMAI_FULL_MANUAL && window.DMAI_FULL_MANUAL()) return false;
    if (window.DMAI && window.DMAI.selfplay) return false;
    return true;
  }

  function decide() {
    const e = encode(); if (!e) return null;
    const t0 = performance.now();
    const p = ex.dmaz_alloc(e.arr.length * 4);
    new Int32Array(ex.memory.buffer, p, e.arr.length).set(e.arr);
    ex.dmaz_decide(p, e.arr.length);
    const r = Array.from(new Int32Array(ex.memory.buffer, ex.dmaz_out(), 7));
    const ms = performance.now() - t0; stats.calls++; stats.ms += ms;
    const U = k => (k >= 0 && k < e.uids.length) ? e.uids[k] : null;
    let act = null;
    switch (r[0]) {
      case 1: act = { a: 'charge', id: U(r[1]) }; break;
      case 2: act = r[3] === 3 ? { a: 'put', id: U(r[1]) } : { a: 'use', id: U(r[1]), side: r[2] }; if (r[3] !== 3 && r[6] >= 0) act.cost = r[6]; break;
      case 3: act = { a: 'attack', id: U(r[1]), target: r[4] < 0 ? 'player' : U(r[4]) }; break;
      case 4: act = { a: 'end' }; break;
      default: return null;
    }
    if (act.a !== 'end' && !act.id) return null;
    last = { turn: S.turn, act, value: r[5] / 1000, ms: Math.round(ms) };
    return act;
  }

  wrap('botTurn', o => function () {
    if (!supported()) return o.apply(this, arguments);
    // 直前の行動が無効だった時は、その1回だけ今までの簡易AIに任せる
    if (S.lastErr) {
      stats.fallback++;
      if (last && last.act && !last.reported) { last.reported = true; stats.invalid.push({ err: S.lastErr, act: last.act, turn: last.turn }); if (stats.invalid.length > 20) stats.invalid.shift(); }
      return o.apply(this, arguments);
    }
    let act = null;
    try { act = decide(); } catch (e) { stats.errors++; broken = true; console.warn('dmaz off', e); }
    if (!act) { stats.fallback++; return o.apply(this, arguments); }
    if (!(window.DMAI_MAXSTEPS >= 40)) window.DMAI_MAXSTEPS = 40; // 1回に1つずつ決めるので、1手番の問い合わせ回数を増やす
    if (!S.dmazAnn) {
      S.dmazAnn = 1;
      try { log(`相手AI：学習AI（${netInfo ? '学習済みの重み' + (netInfo.gen ? '・第' + netInfo.gen + '世代' : '') : '重みなし・先読みのみ'}、${CFG.sims}回の先読み）で考えます`, 'sys'); } catch (e) {}
    }
    const acts = act.a === 'end' ? [act] : [act];
    acts.dmaz = true;
    return { say: '', actions: acts };
  });

  // PC版：相手が「ローカルAI（学習モデル）」の時は askMain から呼ばれるので、そこでも学習AIに替える
  if (CFG.modes.includes('claude')) wrap('askMain', o => async function () {
    if (!supported()) return o.apply(this, arguments);
    const r = window.botTurn();
    if (r && r.actions && r.actions.dmaz) return r;
    return o.apply(this, arguments);
  });

  // 学習AIの一手は「相手の先読み」で別案に差し替えない
  wrap('execOpActions', o => async function (acts) {
    const L = window.DMAI_LOOK;
    if (!acts || !acts.dmaz || !L || !L.cfg) return o.apply(this, arguments);
    const keep = L.cfg.decks; L.cfg.decks = ['\u0000dmaz'];
    try { return await o.apply(this, arguments); } finally { L.cfg.decks = keep; }
  });

  wrap('renderTools', o => function () {
    const r = o.apply(this, arguments);
    try {
      const box = document.querySelector('#tools'); if (!box || box.querySelector('[data-dmaz]')) return r;
      const b = document.createElement('button'); b.dataset.dmaz = '1'; b.className = 'mini';
      b.textContent = '学習AI：' + (on ? 'ON' : 'OFF');
      b.title = '自己対戦で学習したAIが相手の手番を考えます（リースバイク・ハッスル・キャッスル・ジョーカーズのデッキ同士の時だけ）';
      b.onclick = () => { window.DMAZ.set(!on); renderTools(); };
      box.appendChild(b);
    } catch (e) {}
    return r;
  });

  window.DMAZ = { on: () => on, set: v => { on = !!v; broken = false; try { localStorage.setItem(KEY, on ? 'on' : 'off'); } catch (e) {} }, cfg: CFG, stats, ready, get last() { return last; }, get net() { return netInfo; }, _encode: encode, _decide: decide };
})();
