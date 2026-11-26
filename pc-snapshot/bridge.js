// DM-AI bridge: connects the simulator page to the local Python server (dmai/server.py).
// Loaded before the simulator's own script. It stands in for window.claude so that the
// simulator's "Claude" opponent calls the local model instead, records the human's moves
// for imitation learning, and (with ?selfplay=1) runs AI-vs-AI games headlessly.
(() => {
  const Q = new URLSearchParams(location.search);
  const SP = Q.get('selfplay') === '1';
  const PID = Q.get('pid') || '0';
  const DMAI = window.DMAI = { selfplay: SP, pid: PID, flip: 0, banned: new Set(), bannedTurn: -1, v0: null, turnKey: '' };
  window.DMAI_FAST = SP;
  window.DMAI_MAXSTEPS = SP ? 60 : 40;

  async function post(path, body, signal) {
    const r = await fetch(path, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body), signal });
    if (!r.ok) throw new Error('server ' + r.status);
    return r.json();
  }

  /* ---------- observation (what player p can see) ---------- */
  const KW = [/ブロッカー/, /スピードアタッカー/, /S・トリガー/, /マッハファイター/, /(W|T|Q|ワールド|パワード)・ブレイカー/, /進化/, /G・ストライク/, /革命チェンジ|侵略/];
  function feat(c) {
    const d = DB[c.cid]; const s = d.sides[c.side || 0] || d.sides[0];
    const t = s.type || ''; const tx = (d.hyperSplit ? [s] : d.sides).map(x => x.text).join(' ').normalize('NFKC');   // hyper cards: only the current mode
    return [
      Math.min(s.cost, 20) / 10, (isCreature(s) ? power(c) : 0) / 10000,
      ...CIVS.map(v => d.civs.includes(v) ? 1 : 0),
      /クリーチャー/.test(t) ? 1 : 0, /呪文/.test(t) ? 1 : 0, /タマシード|城|フィールド|クロスギア|D2/.test(t) ? 1 : 0, d.sides.length > 1 && !d.hyperSplit ? 1 : 0,
      ...KW.map(r => r.test(tx) ? 1 : 0),
      c.tapped ? 1 : 0, (c.sick && !speed(c)) ? 1 : 0, Math.min(5, (c.under || []).length) / 5,
    ];
  }
  function observe(p) {
    const q = other(p); const tok = []; const idx = {};
    const add = (u, z, o) => { const c = C(u); if (!c || c.cid === HIDDEN_CID) return; idx[u] = tok.length; tok.push({ c: c.cid, z, o, f: feat(c) }); };
    for (const u of zoneArr(p, 'hand')) add(u, 0, 0);
    for (const u of zoneArr(p, 'mana')) add(u, 1, 0);
    for (const u of zoneArr(p, 'battle')) add(u, 2, 0);
    for (const c of faceUpShields(p)) add(c.uid, 3, 0);
    for (const u of zoneArr(p, 'grave').slice(-12)) add(u, 4, 0);
    for (const u of zoneArr(q, 'mana')) if (!C(u).faceDown) add(u, 1, 1);
    for (const u of zoneArr(q, 'battle')) add(u, 2, 1);
    for (const c of faceUpShields(q)) add(c.uid, 3, 1);
    for (const u of zoneArr(q, 'grave').slice(-12)) add(u, 4, 1);
    const untapped = zoneArr(p, 'mana').filter(u => !C(u).tapped).length;
    const g = [S.turn / 20, S.first === p ? 1 : 0, S.charged ? 1 : 0,
      zoneArr(p, 'hand').length / 10, zoneArr(q, 'hand').length / 10, zoneArr(p, 'shield').length / 5, zoneArr(q, 'shield').length / 5,
      zoneArr(p, 'deck').length / 40, zoneArr(q, 'deck').length / 40, manaCount(p) / 10, manaCount(q) / 10, untapped / 10,
      zoneArr(p, 'battle').length / 10, zoneArr(q, 'battle').length / 10];
    // legal actions: 0 charge, 1 use, 2 attack player, 3 attack creature, 4 end, 5 use the 2nd side of a twinpact,
    // 6 release hyper mode (patches.js)
    const acts = [];
    const push = (t, src, tgt, act) => acts.push({ t, s: src == null ? -1 : (idx[src] ?? -1), g: tgt == null ? -1 : (idx[tgt] ?? -1), act });
    const H = window.DMAI_HYPER;
    if (!S.charged) for (const u of zoneArr(p, 'hand')) push(0, u, null, { a: 'charge', id: u });
    for (const u of zoneArr(p, 'hand')) {
      const c = C(u);
      DB[c.cid].sides.forEach((s, si) => {
        if (!s.type || s.hyper) return;
        let cost = s.cost; try { cost = costMods(p, c, s, null).cost; } catch (e) {}
        if (cost <= 0 || payPlan(p, s.civs, cost, u)) { push(si ? 5 : 1, u, null, { a: 'use', id: u, side: si, cost }); return; }
        const he = H && H.heFor(p, c, s, cost);   // ハイパーエナジー: tap creatures to make it payable
        if (he) push(si ? 5 : 1, u, null, { a: 'use', id: u, side: si, cost: he.cost, he: he.tap });
      });
    }
    if (H) for (const c of inBattle(p)) if (H.can(c)) push(6, c.uid, null, { a: 'hyper', id: c.uid, tap: H.pick(c) });
    for (const c of inBattle(p, x => isCreature(side(x)))) {
      if (!canAttack(c, 'player')) push(2, c.uid, null, { a: 'attack', id: c.uid, target: 'player' });
      for (const t of inBattle(q, x => isCreature(side(x)))) if (!canAttack(c, t.uid)) push(3, c.uid, t.uid, { a: 'attack', id: c.uid, target: t.uid });
    }
    push(4, null, null, { a: 'end' });
    return { obs: { tok, g }, acts };
  }
  const enc = acts => acts.map(a => [a.t, a.s, a.g]);
  const seatOf = p => (p === 'op') === (DMAI.flip === 0) ? 'B' : 'A';
  const gameId = () => (S.dmaiGame ||= 'g' + Date.now().toString(36) + Math.random().toString(36).slice(2, 6));
  const turnLines = () => { const L = S.log.map(l => l.t); let i = L.length - 1; while (i > 0 && !L[i].startsWith('――')) i--; return L.slice(i).slice(-16); };

  /* ---------- the AI's main-phase decision ---------- */
  async function mainDecision(signal) {
    const key = gameId() + ':' + S.turn;
    if (DMAI.turnKey !== key) { DMAI.turnKey = key; DMAI.banned = new Set(); DMAI.v0 = null; }
    if (S.lastErr) { const j = S.lastErr.split(' → ')[0]; DMAI.banned.add(j); }
    const ob = observe('op');
    const acts = ob.acts.filter(a => !DMAI.banned.has(JSON.stringify(a.act)));
    const r = await post('/api/act', { seat: seatOf('op'), game: gameId(), obs: ob.obs, acts: enc(acts), selfplay: SP }, signal);
    if (r.fallback) return botTurn();
    const a = acts[r.i] || acts[acts.length - 1];
    if (DMAI.v0 == null && typeof r.v === 'number') DMAI.v0 = r.v;
    if (!SP && a.act.a === 'end' && r.comment !== false) {
      const lines = turnLines();
      post('/api/comment', { game: gameId(), lines, v0: DMAI.v0, v1: r.v, deck: S.deckName?.op || '' })
        .then(c => { if (c && c.text) setVoice(c.text); }).catch(() => {});
    }
    return { say: r.say || '', actions: [a.act] };
  }

  const sample = {
    async json(prompt, opts = {}) {
      const signal = opts.signal;
      if (prompt.includes('# 使えるアクション')) return mainDecision(signal);
      // blocks, S-triggers and other small choices: let the simulator's built-in heuristics decide
      if (prompt.includes('# いま必要な判断') || prompt.includes('# いま必要な選択')) return {};
      const purpose = prompt.includes('ルール処理係') ? 'effect' : 'other';
      try { return await post('/api/llm_json', { prompt, purpose, selfplay: SP }, signal); } catch (e) { return {}; }
    },
    async limits() { return { images: false }; },
  };

  /* ---------- recording the human's moves (imitation data) ---------- */
  let pre = null;
  function commit(ob, match) {
    const i = ob.acts.findIndex(match);
    if (i < 0) return;
    post('/api/log', { game: gameId(), obs: ob.obs, acts: enc(ob.acts), a: i, decks: S.deckName }).catch(() => {});
  }
  function installHooks() {
    if (DMAI.hooked) return; DMAI.hooked = true;
    const wrap = (name, f) => { const orig = window[name]; if (typeof orig !== 'function') { console.warn('dmai: missing ' + name); return; } window[name] = f(orig); };
    const human = () => !SP && S && !S.demo && !NET.on;
    document.addEventListener('click', e => {
      const b = e.target.closest && e.target.closest('button'); if (!b || !human() || !myTurn()) return;
      const t = b.textContent;
      if (t === 'マナチャージ' || t === '使う' || t === 'マナから召喚') pre = { kind: t === 'マナチャージ' ? 'charge' : 'use', uid: selected, ob: observe('me') };
    }, true);
    wrap('brainNote', o => function (kind, cid) {
      if (kind === 'charge' && pre?.kind === 'charge' && C(pre.uid)?.cid === cid) { commit(pre.ob, a => a.act.a === 'charge' && a.act.id === pre.uid); pre = null; }
      return o.apply(this, arguments);
    });
    wrap('useCard', o => function (uid, opts) {
      if (pre?.kind === 'use' && pre.uid === uid && C(uid)?.owner === 'me') {
        const u = window.DMAI_USE_SIDE; const si = opts?.sideIdx ?? (u && u.uid === uid ? u.si : 0);
        commit(pre.ob, a => a.act.a === 'use' && a.act.id === uid && (a.act.side || 0) === si); pre = null;
      }
      return o.apply(this, arguments);
    });
    wrap('attack', o => function (attUid, target) {
      const c = C(attUid);
      if (human() && c?.owner === 'me' && S.active === 'me') commit(observe('me'), a => a.act.a === 'attack' && a.act.id === attUid && a.act.target === target);
      return o.apply(this, arguments);
    });
    wrap('userEndTurn', o => function () {
      if (human() && myTurn()) commit(observe('me'), a => a.act.a === 'end');
      return o.apply(this, arguments);
    });
    wrap('endGame', o => function (w) {
      const had = S.winner; const r = o.apply(this, arguments);
      if (!had && S.winner && !SP && !S.demo && S.dmaiGame) post('/api/game_end', { game: S.dmaiGame, winner: S.winner === 'me' ? 'human' : 'ai', decks: S.deckName }).catch(() => {});
      return r;
    });
    if (SP) installSelfplay();
  }

  /* ---------- self-play (headless, both sides are the AI) ---------- */
  function installSelfplay() {
    for (const n of ['render', 'save', 'setVoice', 'kifuSave', 'pushUndo']) if (typeof window[n] === 'function') window[n] = () => {};
    window.orderCards = uids => Promise.resolve(uids.slice());
    // every prompt meant for the human gets an automatic answer
    window.modal = async ({ buttons, pick }) => {
      const bs = buttons || [];
      const b = bs.find(x => x.primary) || bs.find(x => x.value !== null && x.value !== false && x.value !== 0 && x.value !== -1) || bs[0] || { value: null };
      if (pick) {
        if (pick.dest) { const dest = {}; for (const u of pick.uids) dest[u] = pick.dest[0][0]; return { value: b.value, dest }; }
        const pre = (pick.pre || []).slice(0, pick.max);
        const need = Math.min(pick.max, Math.max(pick.min || 0, b.needPick ? 1 : 0, 1));
        return { value: b.value, picked: pre.length >= (pick.min || 0) && pre.length ? pre : pick.uids.slice(0, need) };
      }
      return b.value;
    };
    const wait = () => new Promise(r => { const t = setInterval(() => { if (sampleFn && DECKS.length) { clearInterval(t); r(); } }, 100); });
    wait().then(spLoop);
  }
  async function spLoop() {
    $('#oppMode').value = 'claude';
    for (;;) {
      let cfg;
      try { cfg = await post('/api/sp/next', { pid: PID }); } catch (e) { await sleep(2000); continue; }
      if (cfg.stop) break;
      try { await spGame(cfg); }
      catch (e) { console.error(e); post('/api/sp/end', { game: cfg.game, pid: PID, error: String(e && e.stack || e) }).catch(() => {}); }
    }
  }
  async function spGame(cfg) {
    const byName = n => n ? DECKS.find(d => d.name.includes(n)) : null;
    const ch = chosen();
    const dA = byName(cfg.deckA) || ch.me, dB = byName(cfg.deckB) || ch.op;
    const first = Math.random() < 0.5 ? 'me' : 'op';
    S = newState(first, { me: dA, op: dB }); brainBegin();
    for (const p of ['me', 'op']) { for (let i = 0; i < 5; i++) place(topOf(p), 'shield'); drawN(p, 5); }
    S.pend = { me: [], op: [] }; S.dmaiGame = cfg.game;
    DMAI.flip = 0;
    if (S.active === 'me') { S = flipShared(S); DMAI.flip = 1; }
    let turns = 0;
    while (!S.winner && turns < cfg.maxTurns) {
      turns++;
      await spTurn();
      if (S.winner) break;
      S = flipShared(S); DMAI.flip ^= 1;
    }
    await post('/api/sp/end', { game: cfg.game, pid: PID, winner: S.winner ? seatOf(S.winner) : null, turns });
  }
  async function spTurn() {
    opRunning = true; busy = true;
    try {
      await startTurn('op'); if (S.winner) return;
      let steps = 0, empty = 0;
      while (!S.winner && steps < 60 && !S.skipRest) {
        steps++;
        const resp = await mainDecision();
        const acts = Array.isArray(resp?.actions) ? resp.actions : [];
        if (!acts.length) { if (++empty >= 2) break; continue; }
        const out = await execOpActions(acts);
        if (out === 'end' || out === 'skip' || S.winner) break;
      }
      if (!S.winner) { await resolveOpPending(); await endTurn('op'); }
    } finally { opRunning = false; busy = false; }
  }

  /* ---------- stand-in for window.claude ---------- */
  window.claude = {
    use: async name => {
      if (name !== 'sample') return null;
      installHooks();
      setTimeout(() => { const o = document.querySelector('#oppMode option[value=claude]'); if (o) o.textContent = 'ローカルAI（学習モデル）'; }, 0);
      return sample;
    },
  };
})();
