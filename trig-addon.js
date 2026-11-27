/* PC用：誘発効果（GitHub duema PR #16・#17・#18 と同じ内容） 2026-11-27 */

/* 学習デッキの誘発効果の追加（2026-11-27）：アーシュ・熊田すず・ジャストダイバー・ダークマスターズ・ラフルル・ラブ・クサビラガミ・マーダン=ダイロウ */
(function(){
  if (window.__etbfx) return; window.__etbfx = true;
  const wrapF = (n, f) => { const o = window[n]; if (typeof o === 'function') window[n] = f(o); };
  const nonEvo = c => !(c.under && c.under.length) && !/進化/.test(side(c).type);
  const isSeitokai = c => /生徒会組/.test(DB[c.cid].race || '');
  const randDiscard = (who, by) => { const h = zoneArr(who, 'hand'); if (!h.length) return; const u = h[Math.floor(Math.random() * h.length)]; place(u, 'grave'); log(`${by}：${PN[who]}の手札《${cname(C(u))}》を捨てさせた`, other(who)); };

  Object.assign(SCRIPTS, {
    // 各ターンに１度、自分のドラゴンが出た時、カードを１枚引いてもよい
    '流星アーシュ＜私が主役': { async allyDragon(x){ if (zoneArr(x.p, 'deck').length && await x.optional('カードを1枚引きますか？')){ x.draw(1); if (C(x.src)) C(x.src).m.t1Dragon = S.turn; } } },
    // 各ターンに１度、自分のドラゴンが出た時、相手の手札を１枚見ないで選び、捨てさせる
    '熊田すず＜わらわは天才': { async allyDragon(x){ if (C(x.src)) C(x.src).m.t1Dragon = S.turn; randDiscard(x.q, '熊田すず'); } },
    // 出た時、相手の手札を見て３枚まで選び、捨てさせる
    '悪魔龍 ダークマスターズ': { async etb(x){
      const h = zoneArr(x.q, 'hand').slice(); if (!h.length) return;
      log(`ダークマスターズ：${PN[x.q]}の手札を見た：${h.map(u => '《' + cname(C(u)) + '》').join('')}`, x.p);
      const pick = await x.choose({q: '捨てさせるカード（3枚まで）', cands: h, min: Math.min(3, h.length), max: 3, score: u => side(C(u)).cost});
      for (const u of pick) place(u, 'grave');
      if (pick.length) log(`ダークマスターズ：${pick.map(u => '《' + cname(C(u)) + '》').join('')}を捨てさせた`, x.p);
    } },
    // 出た時、次の相手のターンの終わりまで、相手は呪文を唱えられない
    '音卿の精霊龍 ラフルル・ラブ': { async etb(x){
      const until = S.active === x.p ? S.turn + 1 : S.turn;
      (S.spellLock ||= {})[x.q] = Math.max(S.spellLock[x.q] || 0, until);
      log(`ラフルル・ラブ：次の${PN[x.q]}のターンの終わりまで、${PN[x.q]}は呪文を唱えられない`, x.p);
    } },
    // 召喚によって出た時、または自分のターンのはじめにアンタップした時
    '土神 クサビラガミ': {
      async etb(x){ if (x.how === 'summon') await kusabira(x); },
      async turnStart(x){ if ((S.kusaUntapped || []).includes(x.src)) await kusabira(x); },
    },
    // 出た時、それが進化なら相手の進化ではないクリーチャーをすべて山札の下へ。出た時、相手の進化ではないクリーチャーをすべて破壊
    '大崩壊炉 マーダン=ダイロウ': { async etb(x){
      if (x.evolved){ const l = inBattle(x.q, nonEvo).map(c => c.uid); if (l.length){ await x.move(l, 'deckBottom'); log(`マーダン=ダイロウ：${PN[x.q]}の進化ではないクリーチャー${l.length}体を山札の下へ`, x.p); } }
      const d = inBattle(x.q, nonEvo).map(c => c.uid); if (d.length){ await x.destroy(d); log(`マーダン=ダイロウ：${PN[x.q]}の進化ではないクリーチャー${d.length}体を破壊`, x.p); }
    } },
  });
  async function kusabira(x){
    // 山札の上から２枚までマナゾーンに置く
    const n = Math.min(2, zoneArr(x.p, 'deck').length);
    let k = n;
    if (x.p === 'me' && n){ const i = await x.mode('山札の上から何枚マナゾーンに置きますか？', Array.from({length: n + 1}, (_, i) => `${n - i}枚`)); k = n - i; }
    if (k) x.topToMana(k, false);
    if (zoneArr(x.p, 'mana').length < 12) return;
    const oc = inBattle(x.q, c => isCreature(side(c))).map(c => c.uid);
    if (oc.length){ const t = await x.choose({q: 'マナゾーンに置く自分のクリーチャーを2体選ぶ', cands: oc, min: Math.min(2, oc.length), max: 2, by: x.q, score: u => -side(C(u)).cost}); await x.move(t, 'mana'); log(`クサビラガミ：${PN[x.q]}はクリーチャー${t.length}体をマナゾーンに置いた`, x.p); }
    const mc = zoneArr(x.p, 'mana').filter(u => isCreature(DB[C(u).cid].sides[0]));
    if (mc.length){ const t = await x.choose({q: 'マナゾーンから出すクリーチャー（2体まで）', cands: mc, min: 0, max: 2, score: u => DB[C(u).cid].sides[0].cost}); await x.put(t); }
  }

  // ターンのはじめ：ジャストダイバーの終わり、クサビラガミの「アンタップした時」
  wrapF('startTurn', o => async function(p){
    for (const c of inBattle(p)) delete c.m.jd;
    S.kusaUntapped = inBattle(p, c => c.tapped && !c.m.frozen && cname(c).includes('土神 クサビラガミ')).map(c => c.uid);
    return o.apply(this, arguments);
  });
  // ジャストダイバー：出た時、次の自分のターンのはじめまで、相手に選ばれず、攻撃されない
  wrapF('allyEnterHooks', o => async function(uid){
    const c = C(uid);
    if (c && c.zone === 'battle' && !NOFX && /(^|[\s　])ジャストダイバー(?!を)/.test(txt(c))){ c.m.jd = c.owner; log(`《${cname(c)}》：ジャストダイバー（次の${PN[c.owner]}のターンのはじめまで、相手に選ばれず、攻撃されない）`, c.owner); }
    return o.apply(this, arguments);
  });
  wrapF('makeCtx', o => function(srcUid, p){
    const x = o.apply(this, arguments);
    const ch = x.choose;
    x.choose = opts => { const by = opts.by || p; return ch({...opts, cands: (opts.cands || []).filter(u => !(C(u) && C(u).zone === 'battle' && C(u).m.jd && C(u).owner !== by))}); };
    return x;
  });
  wrapF('canAttack', o => function(c, target){
    const r = o.apply(this, arguments); if (r || NOFX) return r;
    if (target !== 'player' && C(target) && C(target).m.jd) return 'ジャストダイバー（次の持ち主のターンのはじめまで攻撃されない）';
    // 流星アーシュ：ドラゴンではないクリーチャーは、出たターンに攻撃できない（相手のアーシュはレッドゾーンで無視できる）
    if (c.enteredT === S.turn && !isDragonCard(c.uid)){
      const own = inBattle(c.owner, x => cname(x).includes('流星アーシュ')).length, opp = inBattle(other(c.owner), x => cname(x).includes('流星アーシュ')).length;
      if (own || (opp && !inBattle(c.owner, x => cname(x).includes('レッドゾーン')).length)) return '流星アーシュ：ドラゴンではないクリーチャーは出たターンに攻撃できない';
    }
    return null;
  });
  // 熊田すず：自分の生徒会組すべてにスレイヤー（バトルの後、相手クリーチャーを破壊）
  wrapF('battle', o => async function(a, b){
    const A = C(a), B = C(b);
    const sl = c => c && !NOFX && isSeitokai(c) && inBattle(c.owner, x => cname(x).includes('熊田すず')).length > 0;
    const sa = sl(A), sb = sl(B);
    await o.apply(this, arguments);
    if (sa && C(b) && C(b).zone === 'battle'){ log(`スレイヤー（熊田すず）：《${cname(C(b))}》を破壊`, A.owner); await leave(b, 'grave', {destroy: true}); }
    if (sb && C(a) && C(a).zone === 'battle'){ log(`スレイヤー（熊田すず）：《${cname(C(a))}》を破壊`, B.owner); await leave(a, 'grave', {destroy: true}); }
  });
  // ラフルル・ラブ：呪文を唱えられない
  wrapF('metaConfirm', o => async function(p, s){
    if (!NOFX && s && s.type === '呪文' && S.spellLock && (S.spellLock[p] || 0) >= S.turn){ log(`《音卿の精霊龍 ラフルル・ラブ》の効果で、${PN[p]}は呪文《${s.name}》を唱えられない`, p); return false; }
    return o.apply(this, arguments);
  });
  // 「処理を変更」で決めた誘発（カードのスクリプトや自動処理が無いタイミング）を起こす
  const ruled = (uid, ev) => !NOFX && C(uid) && hasRule(uid, ev);
  const T1D = /各ターンに[1１]度、自分のドラゴンが(バトルゾーンに)?出た時/;
  wrapF('allyEnterHooks', o => async function(uid){
    await o.apply(this, arguments);
    const c = C(uid); if (!c || c.zone !== 'battle' || S.winner || !isDragonCard(uid)) return;
    for (const x of inBattle(c.owner)) if (!T1D.test(txt(x)) && ruled(x.uid, 'allyDragon')) await resolveEffect(x.uid, 'allyDragon', {who: uid});
  });
  wrapF('leave', o => async function(uid){
    const c = C(uid); const was = c && c.zone === 'battle'; const p = c && c.owner;
    const r = await o.apply(this, arguments);
    if (was && C(uid) && C(uid).zone !== 'battle' && !S.winner){
      if (ruled(uid, 'leave')) await resolveEffect(uid, 'leave');
      for (const x of inBattle(p)) if (ruled(x.uid, 'allyLeave')) await resolveEffect(x.uid, 'allyLeave', {who: uid});
    }
    return r;
  });
  wrapF('attack', o => async function(attUid){
    const r = await o.apply(this, arguments);
    const A = C(attUid);
    if (r && r.ok && A && !S.winner) for (const x of inBattle(other(A.owner))) if (ruled(x.uid, 'oppAttack')) await resolveEffect(x.uid, 'oppAttack', {who: attUid});
    return r;
  });
  wrapF('resolveAttack', o => async function(a){
    const prev = S.__atk; S.__atk = {a: a.attUid, t: a.target};
    try { await o.apply(this, arguments); } finally { S.__atk = prev; }
    if (!S.winner && ruled(a.attUid, 'attackEnd') && !(scriptFor(a.attUid) || {}).attackEnd) await resolveEffect(a.attUid, 'attackEnd');
  });
  wrapF('battle', o => async function(a, b){
    if (S.__atk && S.__atk.a === a && S.__atk.t !== b && ruled(b, 'block')) await resolveEffect(b, 'block', {who: a});
    if (!C(a) || C(a).zone !== 'battle' || !C(b) || C(b).zone !== 'battle') return;
    return o.apply(this, arguments);
  });
  wrapF('useCard', o => async function(uid, opt){
    const c = C(uid); const p = c && c.owner; const si = (opt && opt.sideIdx) || 0; const sp = c && DB[c.cid].sides[si] && DB[c.cid].sides[si].type === '呪文';
    const r = await o.apply(this, arguments);
    if (sp && r && r.ok && !S.winner){
      for (const x of inBattle(other(p))) if (ruled(x.uid, 'oppSpell')) await resolveEffect(x.uid, 'oppSpell', {who: uid});
      for (const x of inBattle(p)) if (ruled(x.uid, 'allySpell')) await resolveEffect(x.uid, 'allySpell', {who: uid});
    }
    return r;
  });
})();


/* 学習デッキの誘発効果の追加 その2（2026-11-27）：蒼斬丸・逆転撃・ジャヴェール・7777777・アイアン・マンハッタン・富士山ッピング */
(function(){
  if (window.__trigfx2) return; window.__trigfx2 = true;
  const wrapF = (n, f) => { const o = window[n]; if (typeof o === 'function') window[n] = f(o); };
  const race = u => String(DB[C(u).cid].race || '');
  const names = us => us.map(u => '《' + cname(C(u)) + '》').join('');
  const s0 = u => DB[C(u).cid].sides[0];
  const isJ = u => /ジョーカーズ/.test(race(u));

  Object.assign(SCRIPTS, {
    // 出た時、カードを２枚まで引く
    '切札混成 光隼 蒼斬丸': { async etb(x){
      const n = Math.min(2, zoneArr(x.p, 'deck').length); if (!n) return;
      let k = n; if (x.p === 'me'){ const i = await x.mode('カードを何枚引きますか？', Array.from({length: n + 1}, (_, i) => `${n - i}枚`)); k = n - i; }
      if (k) x.draw(k);
    } },
    // 7777777：相手の山札の上から3枚を表向き、1枚選ぶ。同じコストの相手のクリーチャーすべてと3枚を山札の下へ
    '7777777': { async spell(x){
      const L = x.look(3, x.q); if (!L.length) return;
      log(`7777777：${PN[x.q]}の山札の上から${names(L)}を表向きにした`, x.p);
      const cnt = u => inBattle(x.q, c => isCreature(side(c)) && side(c).cost === s0(u).cost).length;
      const [t] = await x.choose({q: 'コストを決めるカードを1枚選ぶ', cands: L, min: 1, max: 1, score: cnt});
      const cost = s0(t).cost; const cs = inBattle(x.q, c => isCreature(side(c)) && side(c).cost === cost).map(c => c.uid);
      log(`7777777：コスト${cost}。${cs.length ? names(cs) + 'と' : ''}表向きの3枚を${PN[x.q]}の山札の下へ`, x.p);
      await x.move([...cs, ...L], 'deckBottom');
    } },
    // アイアン・マンハッタン：出た時、相手のシールドを２つ選ぶ。それ以外をすべてブレイク。その後、手札を１枚捨ててもよい。そうしたら次のターン、相手は２体以上出せない
    'アイアン・マンハッタン': { async etb(x){
      const sh = zoneArr(x.q, 'shield').slice();
      if (sh.length > 2){
        const keep = await x.choose({q: 'ブレイクしないシールドを2つ選ぶ', cands: sh, min: 2, max: 2, score: () => Math.random()});
        const br = sh.filter(u => !keep.includes(u)); const broken = [];
        for (const u of br){ const s = C(u); if (s.zone !== 'shield') continue; if (s.faceUp){ place(u, 'grave'); } else { place(u, 'hand'); broken.push(u); } }
        log(`アイアン・マンハッタン：${PN[x.q]}のシールドを${br.length}つブレイク`, x.p); render();
        for (const u of broken) await shieldTrigger(u, C(x.src));
      }
      const h = zoneArr(x.p, 'hand').slice(); if (!h.length || S.winner) return;
      if (!await x.optional('手札を1枚捨てて、次のターン相手がクリーチャーを2体以上出せないようにしますか？')) return;
      const [d] = await x.choose({q: '捨てる手札', cands: h, min: 1, max: 1, score: u => -s0(u).cost});
      if (!d) return; place(d, 'grave');
      (S.manLimit ||= {})[x.q] = S.turn + 1;
      log(`アイアン・マンハッタン：《${cname(C(d))}》を捨てた。次のターン、${PN[x.q]}はクリーチャーを2体以上出せない`, x.p);
    } },
    // ジャヴェールのスマッシュ・バースト／呪文側「ヤベーのを見たいか？」
    '邪幽 ジャヴェール': { async attack(x){
      if (!await x.optional('スマッシュ・バースト：「ヤベーのを見たいか？」を唱えますか？')) return;
      await yabe(x);
    } },
    '「ヤベーのを見たいか？」': { async spell(x){ await yabe(x); } },
  });
  async function yabe(x){
    x.topToMana(1, true);
    const mana = zoneArr(x.p, 'mana');
    if (!mana.length || !mana.every(u => /アビス/.test(race(u)))) return;
    const cs = inBattle(x.q, c => isCreature(side(c))).map(c => c.uid); if (!cs.length) return;
    const [t] = await x.choose({q: 'このターン、パワーを−2000するクリーチャー', cands: cs, min: 1, max: 1, score: u => -power(C(u))});
    if (!t) return; C(t).m.powerT = (C(t).m.powerT || 0) - 2000; log(`「ヤベーのを見たいか？」：《${cname(C(t))}》のパワー−2000`, x.p);
    if (power(C(t)) <= 0) await x.destroy(t);
  }

  // 相手のクリーチャーが攻撃する時：逆転撃、蒼斬丸の攻撃中止
  wrapF('attack', o => async function(attUid, target){
    const A0 = C(attUid); const before = new Set(A0 ? zoneArr(other(A0.owner), 'battle') : []);
    const r = await o.apply(this, arguments);
    const A = C(attUid); if (!r || !r.ok || !A || S.winner || NOFX) return r;
    const q = other(A.owner);
    // ウラ・ニンジャ・ストライクは本体のニンジャ・ストライク処理が行う
    // 逆転撃：クリーチャーが自分を攻撃する時、手札からコストなしで唱えてもよい（次の自分のターンのはじめに闇3を支払う。払えなければ負け）
    if (target === 'player' && !S.winner){
      const gk = zoneArr(q, 'hand').find(u => cname(C(u)).includes('深淵の逆転撃'));
      const cs = inBattle(A.owner, c => isCreature(side(c)) && !c.m.jd).map(c => c.uid);
      if (gk && cs.length){
        const y = makeCtx(gk, q);
        const canPay = zoneArr(q, 'mana').length >= 3 && zoneArr(q, 'mana').some(u => allCivs(C(u)).includes('闇'));
        const use = q === 'me' ? await y.optional('逆転撃：《深淵の逆転撃》をコストなしで唱えますか？（次の自分のターンのはじめに闇のマナ3を支払えなければ負け）') : (canPay && await y.optional('逆転撃：《深淵の逆転撃》を唱える'));
        if (use){
          detach(gk); C(gk).zone = 'limbo'; log(`${PN[q]}：逆転撃《深淵の逆転撃》を唱えた`, q);
          (S.debt ||= {})[q] = (S.debt[q] || 0) + 3;
          const [t] = await y.choose({q: 'このターン、パワーを−∞するクリーチャー', cands: cs, min: 1, max: 1, score: u => (u === attUid ? 1e9 : 0) + power(C(u))});
          if (t){ log(`深淵の逆転撃：《${cname(C(t))}》のパワー−∞（破壊）`, q); await y.destroy(t); }
          place(gk, 'grave');
        }
      }
    }
    // 蒼斬丸：各ターンに１度、相手のクリーチャーが攻撃する時、手札を１枚捨ててもよい。そうしたら攻撃を中止
    if (C(attUid) && C(attUid).zone === 'battle' && !S.winner) for (const z of inBattle(q, c => cname(c).includes('光隼 蒼斬丸') && c.m.t1Cancel !== S.turn && before.has(c.uid))){ // 攻撃中に出た蒼斬丸（ニンジャ・ストライク）は、この攻撃では誘発しない
      const h = zoneArr(q, 'hand').slice(); if (!h.length) break;
      const y = makeCtx(z.uid, q);
      const worth = target === 'player' || (C(target) && power(A) >= power(C(target)));
      if (!(q === 'me' ? await y.optional(`《${cname(A)}》の攻撃を、手札を1枚捨てて中止しますか？`) : (worth && await y.optional('蒼斬丸：手札を1枚捨てて攻撃を中止')))) continue;
      const [d] = await y.choose({q: '捨てる手札', cands: h, min: 1, max: 1, score: u => -s0(u).cost});
      if (!d) continue; place(d, 'grave'); z.m.t1Cancel = S.turn; C(attUid).m.atkCancel = true;
      log(`蒼斬丸：《${cname(C(d))}》を捨てて《${cname(A)}》の攻撃を中止`, q); break;
    }
    return r;
  });
  wrapF('resolveAttack', o => async function(a){
    const A = C(a.attUid); if (A && A.m.atkCancel){ delete A.m.atkCancel; log(`《${cname(A)}》の攻撃は中止された`); return; }
    return o.apply(this, arguments);
  });

  // ジャヴェール：各ターンに１度、自分のアビスが離れた時、相手は自身の手札を１枚選び、捨てる／富士山ッピング：各ターンに１度、自分の他のジョーカーズが離れた時、その「出た時」の能力を１つ使ってもよい
  wrapF('leave', o => async function(uid){
    const c = C(uid); const was = c && c.zone === 'battle'; const p = c && c.owner;
    const jav = was && /アビス/.test(race(uid)) ? inBattle(p, x => cname(x).includes('邪幽 ジャヴェール')).map(x => x.uid) : [];
    const fuji = was && isJ(uid) ? inBattle(p, x => x.uid !== uid && cname(x).includes('Dr.富士山ッピング') && x.m.t1Fuji !== S.turn).map(x => x.uid) : [];
    const r = await o.apply(this, arguments);
    if (!was || !C(uid) || C(uid).zone === 'battle' || S.winner || NOFX) return r;
    for (const j of jav){
      if (S.tf.used['jav' + j]) continue; S.tf.used['jav' + j] = true;
      const q = other(p); const h = zoneArr(q, 'hand').slice(); if (!h.length) continue;
      const y = makeCtx(j, p);
      const [d] = await y.choose({q: '捨てる手札を1枚選ぶ（ジャヴェール）', cands: h, min: 1, max: 1, by: q, score: u => -s0(u).cost});
      if (d){ place(d, 'grave'); log(`ジャヴェール：${PN[q]}は《${cname(C(d))}》を捨てた`, p); }
    }
    for (const f of fuji){
      if (!C(f) || C(f).zone !== 'battle') continue;
      const has = DB[C(uid).cid].sides.some(s => TRIG.etb.test(s.text)) || (scriptFor(uid) || {}).etb;
      if (!has) continue;
      const y = makeCtx(f, p);
      if (!await y.optional(`《${cname(C(uid))}》の「出た時」の能力を使いますか？（富士山ッピング）`)) continue;
      C(f).m.t1Fuji = S.turn; log(`富士山ッピング：《${cname(C(uid))}》の「出た時」の能力を使う`, p);
      await resolveEffect(uid, 'etb', {how: 'fuji'}); break;
    }
    return r;
  });

  // 富士山ッピング：自分のターンの終わりに手札にあれば、ジョーカーズ・クリーチャーをコスト合計7以上になるよう手札に戻して、これを出す
    wrapF('endTurn', o => async function(p){
    if (!NOFX && !S.winner){
      const fu = zoneArr(p, 'hand').find(u => cname(C(u)).includes('Dr.富士山ッピング'));
      const js = inBattle(p, c => isCreature(side(c)) && isJ(c.uid));
      if (fu && js.reduce((a, c) => a + side(c).cost, 0) >= 7){
        const y = makeCtx(fu, p);
        if (await y.optional('ジョーカーズ・クリーチャーをコスト合計7以上になるように手札に戻して、《Dr.富士山ッピング》を出しますか？')){
          let back;
          if (p === 'me'){
            for (;;){ back = await y.choose({q: '手札に戻すジョーカーズ（コスト合計7以上）', cands: js.map(c => c.uid), min: 1, max: 99}); if (!back.length || back.reduce((a, u) => a + side(C(u)).cost, 0) >= 7) break; await modal({title: '富士山ッピング', html: '<p>コストの合計が7以上になるように選んでください。</p>', buttons: [{label: 'もう一度', value: 1, primary: true}]}); }
          } else {
            back = []; let sum = 0;
            for (const c of js.slice().sort((a, b) => side(b).cost - side(a).cost)){ if (sum >= 7) break; back.push(c.uid); sum += side(c).cost; }
          }
          if (back && back.length && back.reduce((a, u) => a + side(C(u)).cost, 0) >= 7){
            await y.move(back, 'hand'); log(`富士山ッピング：${names(back)}を手札に戻して《Dr.富士山ッピング》を出す`, p);
            await putIntoBattle(fu, 'put');
          }
        }
      }
    }
    return o.apply(this, arguments);
  });
  // 逆転撃の支払い（次の自分のターンのはじめ）
  wrapF('startTurn', o => async function(p){
    const r = await o.apply(this, arguments);
    const n = (S.debt || {})[p];
    if (n && !S.winner){
      S.debt[p] = 0;
      if (payCost(p, ['闇'], n)) log(`逆転撃：闇のマナ${n}を支払った`, p);
      else tryLose(p, `逆転撃の支払い（闇${n}）ができなかった`);
      render();
    }
    return r;
  });
  // アイアン・マンハッタン：次のターン、相手は２体以上クリーチャーを出せない／ジョギラゴン：相手は自身の各ターンに１枚しか呪文を唱えられない
  wrapF('metaConfirm', o => async function(p, s){
    if (!NOFX && s && s.type === '呪文' && S.active === p && tfCount(p, 'spell') >= 1 && inBattle(other(p), c => cname(c).includes('ジョギラゴン＆ジョニー')).length){ log(`《ジョギラゴン＆ジョニー ～Jの旅路～》の効果で、${PN[p]}はこのターン2枚目の呪文を唱えられない`, p); return false; }
    if (!NOFX && s && isCreature(s) && S.manLimit && S.manLimit[p] === S.turn && tfCount(p, 'creature') >= 1){ log(`《アイアン・マンハッタン》の効果で、${PN[p]}はこのターン2体目のクリーチャーを出せない`, p); return false; }
    return o.apply(this, arguments);
  });
})();
