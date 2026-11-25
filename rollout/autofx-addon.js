/* ---------- 効果文の自動読み取り（2026-11-25） ----------
   専用の処理（SCRIPTS）が無いカードの「出た時」「攻撃する時」「破壊された時」「ターンのはじめ・終わり」「呪文」の能力を、
   効果文のよくある書き方から読み取って、その場で処理する。1つの能力の中に読み取れない文が1つでもあれば、
   その能力は読み取らない（今までどおり「手で処理」の案内か、Claude 対戦なら Claude に任せる）。
   読み取りの決まり（公式 総合ゲームルール準拠）：
   ・「〜選び、〜する」は選べる対象がいる限り選ぶ（強制）。「〜まで」「〜てもよい」は0体でもよい。
   ・「そうしたら」は直前の行動を実際にした時だけ続きをする。
   ・山札の上から置くマナは「タップして」と書いてある時だけタップ（多色はいつもタップ）。
   この部分は画面を使わない純粋な関数なので、node でも動く（読み取れる割合の計測用）。 */
(function (root) {
  const nf = t => String(t || '').normalize('NFKC');
  const CIVS = ['光', '水', '闇', '火', '自然'];
  const ACT = {
    '破壊する': 'destroy', '持ち主の手札に戻す': 'hand', '手札に戻す': 'hand', 'タップする': 'tap', 'アンタップする': 'untap',
    '持ち主のマナゾーンに置く': 'mana', 'マナゾーンに置く': 'mana', '持ち主の墓地に置く': 'grave', '墓地に置く': 'grave',
    '持ち主の山札の下に置く': 'deckBottom', '持ち主の山札の一番下に置く': 'deckBottom', '山札の下に置く': 'deckBottom',
    '持ち主のシールドゾーンに裏向きにして加える': 'shield', '持ち主のシールドゾーンに加える': 'shield',
  };
  const ACT_RE = Object.keys(ACT).sort((a, b) => b.length - a.length).map(k => k.replace(/[()]/g, '\\$&')).join('|');

  // クリーチャーの条件（「パワー6000以下の」「タップされている」など）。読めない言葉が残れば null
  function creFilter(t) {
    const f = {}; let r = t;
    const take = (re, fn) => { const m = r.match(re); if (m) { fn(m); r = r.replace(re, ''); return true; } return false; };
    for (let k = 0; k < 6; k++) {
      let hit = false;
      hit = take(/^(?:バトルゾーンにある)/, () => {}) || hit;
      hit = take(/^パワー(\d+)以下の/, m => f.maxPower = +m[1]) || hit;
      hit = take(/^パワー(\d+)以上の/, m => f.minPower = +m[1]) || hit;
      hit = take(/^コスト(\d+)以下の/, m => f.maxCost = +m[1]) || hit;
      hit = take(/^コスト(\d+)以上の/, m => f.minCost = +m[1]) || hit;
      hit = take(/^(?:タップされている|タップしている)/, () => f.tapped = true) || hit;
      hit = take(/^(?:アンタップされている|アンタップしている|タップされていない|タップしていない)/, () => f.untapped = true) || hit;
      hit = take(/^(?:進化ではない|進化でない)/, () => f.nonEvolution = true) || hit;
      hit = take(/^「ブロッカー」を持つ/, () => f.text = 'ブロッカー') || hit;
      hit = take(/^(光|水|闇|火|自然)の/, m => f.civ = m[1]) || hit;
      hit = take(/^他の/, () => f.other = true) || hit;
      if (!hit) break;
    }
    return r === '' ? f : null;
  }
  // カードの条件（「ジョーカーズ・クリーチャー」「コスト3以下の呪文」「光のカード」など）
  function cardFilter(t) {
    const m = t.match(/^(.*?)(カード|クリーチャー|呪文|エレメント)$/); if (!m) return null;
    let pre = m[1]; const f = {};
    if (m[2] === 'クリーチャー') f.type = 'クリーチャー'; else if (m[2] === '呪文') f.type = '呪文';
    const rm = pre.match(/^(?:.*?の)?([^\sの・]+(?:・[^\sの・]+)*)・$/);
    if (rm && !/^(パワー|コスト)/.test(rm[1])) { f.race = rm[1]; pre = pre.slice(0, pre.length - rm[1].length - 1); }
    const cf = creFilter(pre); if (!cf) return null;
    return Object.assign(f, cf);
  }
  const who = w => (w === '相手' ? 'q' : 'p');
  const N = s => +s;

  // 1つの文を処理に変える。返り値 {op, opt} か null
  // op: {k:種類, ...} を JSON のまま持つ（画面側の run で実行）
  function sentence(s, prev) {
    let m;
    s = s.replace(/^バトルゾーンにある(?=(相手|自分)の|クリーチャー)/, '').replace(/ひとつ/g, '1つ').replace(/ふたつ/g, '2つ').replace(/枚目を、/, '枚目を').replace(/を自分のマナゾーンに置/, 'をマナゾーンに置').replace(/選んで捨てる$/, '選び、捨てる').replace(/^その後、/, '').replace(/^(?:そうしたら|そうした場合)、/, '').replace(/^そのターン、/, 'このターン、').replace(/(\d+)(体|枚|つ)、(破壊|手札|墓地|マナ)/, '$1$2$3')
      .replace(/^(相手|自分)の(.*?)クリーチャーを(\d+)体選んで(タップ|破壊)する$/, '$1の$2クリーチャーを$3体選び、$4する')
      .replace(/新しいシールドとして(?:自分の)?シールドゾーンに置く$/, 'シールド化する').replace(/を裏向きのまま、?シールド化する$/, 'をシールド化する');
    if (!s) return { op: { k: 'nop' } };
    // 「〜てもよい」を外して読む（あとで「使うか」を聞く。選ぶ処理なら0体でもよいにする）
    const MAY = [['破壊してもよい', '破壊する'], ['戻してもよい', '戻す'], ['置いてもよい', '置く'], ['タップしてもよい', 'タップする'], ['アンタップしてもよい', 'アンタップする'], ['加えてもよい', '加える'], ['捨ててもよい', '捨てる'], ['出してもよい', '出す'], ['シールド化してもよい', 'シールド化する']];
    if (!/^その中から/.test(s)) for (const [a, b] of MAY) if (s.endsWith(a) && !/^カードを\d+枚(まで)?引/.test(s)) {
      const r = sentence(s.slice(0, -a.length) + b, prev); if (!r) return null;
      if (['pick', 'zoneToHand', 'putFromHand', 'all', 'actSel'].includes(r.op.k)) { if ('min' in r.op) r.op.min = 0; else r.opt = true; } else r.opt = true;
      if (r.op.k === 'all' || r.op.k === 'actSel') r.opt = true;
      return r;
    }
    // 山札
    if ((m = s.match(/^カードを(\d+)枚(まで)?引(く|いてもよい)$/))) return { op: { k: 'draw', n: N(m[1]), upTo: !!m[2] }, opt: m[3] !== 'く' };
    if ((m = s.match(/^カードを(\d+)枚引き、(?:その後、)?自分の手札を(\d+)枚捨てる$/))) return { op: { k: 'seq', ops: [{ k: 'draw', n: N(m[1]) }, { k: 'discard', n: N(m[2]) }] } };
    if ((m = s.match(/^(?:自分の)?山札の上から(\d+)枚(?:目)?を、?(タップして)?マナゾーンに置(く|いてもよい)$/))) return { op: { k: 'topMana', n: N(m[1]), tapped: !!m[2] }, opt: m[3] !== 'く' };
    if ((m = s.match(/^(?:自分の)?山札の上から(\d+)枚(?:目)?を墓地に置(く|いてもよい)$/))) return { op: { k: 'mill', n: N(m[1]) }, opt: m[2] !== 'く' };
    if ((m = s.match(/^(?:自分の)?山札の上から(\d+)枚(?:目)?を(?:シールド化する|シールドゾーンに加える|裏向きのままシールドゾーンに加える)$/))) return { op: { k: 'topShield', n: N(m[1]) } };
    if ((m = s.match(/^(?:自分の)?山札の上から(\d+)枚(?:目)?を(?:見る|表向きにする)$/))) return { op: { k: 'look', n: N(m[1]) } };
    if ((m = s.match(/^自分の山札を見る$/))) return { op: { k: 'search' } };
    if ((m = s.match(/^(?:その後、)?(?:自分の)?山札をシャッフルする$/))) return { op: { k: 'shuffle' } };
    if ((m = s.match(/^その中から、?(.+?)を(\d+|好きな)(?:枚|体)(まで)?(?:選び)?、?(相手に見せてから|表向きにして|表向きにし、)?(手札に加え|バトルゾーンに出|墓地に置|マナゾーンに置|手札に加えてもよい|バトルゾーンに出してもよい)(る|す|く|)$/))) {
      const f = cardFilter(m[1]); if (!f || !prev.looked) return null;
      const dest = { '手札に加え': 'hand', '手札に加えてもよい': 'hand', 'バトルゾーンに出': 'battle', 'バトルゾーンに出してもよい': 'battle', '墓地に置': 'grave', 'マナゾーンに置': 'mana' }[m[5]];
      const any = m[2] === '好きな'; const opt = /もよい/.test(m[5]);
      if (dest === 'battle' && !f.type) f.type = 'クリーチャー';
      return { op: { k: 'pickLooked', f, n: any ? 99 : N(m[2]), min: (any || m[3] || opt) ? 0 : N(m[2]), dest } };
    }
    if ((m = s.match(/^その中から、?(.+?)を(\d+|好きな)枚(まで)?、?(相手に見せてから|表向きにして)?手札に加え、残りを(好きな順序で)?(?:自分の)?山札の(?:一番)?下に置く$/))) {
      const f = cardFilter(m[1]); if (!f || !prev.looked) return null;
      const any = m[2] === '好きな';
      return { op: { k: 'seq', ops: [{ k: 'pickLooked', f, n: any ? 99 : N(m[2]), min: (any || m[3]) ? 0 : N(m[2]), dest: 'hand' }, { k: 'restLooked', dest: 'deckBottom' }] } };
    }
    if ((m = s.match(/^残り(?:のカード)?を(?:好きな順序で)?(?:自分の)?(山札の(?:一番)?下に置く|墓地に置く|マナゾーンに置く|タップしてマナゾーンに置く|山札の上に戻す|好きな順序で山札の上に戻す|シャッフルして山札の下に置く)$/))) {
      if (!prev.looked) return null;
      const d = /下/.test(m[1]) ? 'deckBottom' : /墓地/.test(m[1]) ? 'grave' : /マナ/.test(m[1]) ? 'mana' : 'deckTop';
      return { op: { k: 'restLooked', dest: d } };
    }
    // 手札
    if ((m = s.match(/^自分の手札を(\d+)枚(?:まで)?捨て(る|てもよい)$/))) return { op: { k: 'discard', n: N(m[1]) }, opt: m[2] !== 'る' };
    if ((m = s.match(/^相手は自身の手札を(\d+)枚選び、捨てる$/))) return { op: { k: 'oppDiscard', n: N(m[1]) } };
    if ((m = s.match(/^相手の手札を(?:見ないで)?(\d+)枚(?:見ないで)?選び、捨てさせる$/))) return { op: { k: 'randDiscard', n: N(m[1]) } };
    if ((m = s.match(/^(?:カードを)?(\d+)枚、自分のマナゾーンから手札に戻(す|してもよい)$/)) || (m = s.match(/^カードを(\d+)枚、自分のマナゾーンから手札に戻(す|してもよい)$/))) return { op: { k: 'zoneToHand', zone: 'mana', f: {}, n: N(m[1]), min: m[2] === 'す' ? N(m[1]) : 0 } };
    if ((m = s.match(/^自分の(墓地|マナゾーン)から(.+?)を(\d+)(?:枚|体)(まで)?選び、手札に(?:戻す|加える)$/)) || (m = s.match(/^(.+?)を(\d+)(?:枚|体)(まで)?、自分の(墓地|マナゾーン)から手札に(?:戻す|加える)$/))) {
      const [zone, ft, n, upto] = m[1] === '墓地' || m[1] === 'マナゾーン' ? [m[1], m[2], m[3], m[4]] : [m[4], m[1], m[2], m[3]];
      const f = cardFilter(ft); if (!f) return null;
      return { op: { k: 'zoneToHand', zone: zone === '墓地' ? 'grave' : 'mana', f, n: N(n), min: upto ? 0 : N(n) } };
    }
    if ((m = s.match(/^自分の(墓地|マナゾーン)から(.+?)を(\d+)(?:枚|体)(まで)?選び、手札に(?:戻|加え)してもよい$/))) {
      const f = cardFilter(m[2]); if (!f) return null;
      return { op: { k: 'zoneToHand', zone: m[1] === '墓地' ? 'grave' : 'mana', f, n: N(m[3]), min: 0 } };
    }
    // 手札から出す
    if ((m = s.match(/^(.+?)を(\d+)体(まで)?、自分の手札から(?:バトルゾーンに)?出(す|してもよい)$/)) || (m = s.match(/^自分の手札から(.+?)を(\d+)体(まで)?(?:バトルゾーンに)?出(す|してもよい)$/))) {
      const f = cardFilter(m[1]); if (!f) return null; if (!f.type) f.type = 'クリーチャー';
      return { op: { k: 'putFromHand', f, n: N(m[2]), min: 0 } };
    }
    // シールド
    if ((m = s.match(/^自分のシールドを(\d+)つ(?:選び、)?、?手札に加える$/))) return { op: { k: 'shieldToHand', n: N(m[1]) } };
    if ((m = s.match(/^相手のシールドを(\d+)つ選び、(?:持ち主の)?(手札に加える|墓地に置く)$/))) return { op: { k: 'oppShield', n: N(m[1]), dest: /手札/.test(m[2]) ? 'hand' : 'grave' } };
    if ((m = s.match(/^自分の手札を(\d+)枚捨て、(?:その後、)?カードを(\d+)枚引く$/))) return { op: { k: 'seq', ops: [{ k: 'discard', n: N(m[1]) }, { k: 'draw', n: N(m[2]) }] } };
    if ((m = s.match(/^相手の手札を見て(\d+)枚選び、捨てさせる$/))) return { op: { k: 'peekDiscard', n: N(m[1]) } };
    if ((m = s.match(/^自分の手札を(\d+)枚(?:、)?マナゾーンに置く$/))) return { op: { k: 'handToMana', n: N(m[1]) } };
    if ((m = s.match(/^自分のマナゾーンにあるカードをすべてアンタップする$/))) return { op: { k: 'untapMana' } };
    if ((m = s.match(/^自分のシールドを(\d+)つ墓地に置く$/))) return { op: { k: 'shieldToGrave', n: N(m[1]) } };
    if ((m = s.match(/^相手は自身の(.*?)クリーチャーを(\d+)体選び、(破壊する|持ち主の手札に戻す|手札に戻す|墓地に置く|マナゾーンに置く)$/))) {
      const f = creFilter(m[1]); if (!f) return null;
      return { op: { k: 'pick', side: 'q', by: 'q', f, n: N(m[2]), min: N(m[2]), act: ACT[m[3]] } };
    }
    if ((m = s.match(/^自分の(.*?)クリーチャーを(\d+)体破壊する$/))) { const f = creFilter(m[1]); if (!f) return null; return { op: { k: 'pick', side: 'p', f, n: N(m[2]), min: N(m[2]), act: 'destroy', mine: true } }; }
    if ((m = s.match(new RegExp('^(相手|自分)?の?(.*?)(エレメント|カード)を(\\d+)(?:つ|枚)(まで)?選び、(' + ACT_RE + ')$'))) && (m[1] || m[3] === 'エレメント')) {
      const f = creFilter(m[2].replace(/^バトルゾーンにある/, '')); if (!f) return null; f.elem = true;
      return { op: { k: 'pick', side: m[1] ? who(m[1]) : 'any', f, n: N(m[4]), min: m[5] ? 0 : N(m[4]), act: ACT[m[6]] } };
    }
    if ((m = s.match(new RegExp('^(.*?)クリーチャーを(\\d+)体(まで)?選び、(' + ACT_RE + ')$'))) && !/^(相手|自分)の/.test(s)) {
      const f = creFilter(m[1]); if (!f) return null;
      return { op: { k: 'pick', side: 'any', f, n: N(m[2]), min: m[3] ? 0 : N(m[2]), act: ACT[m[4]] } };
    }
    if (s === 'このクリーチャーを破壊する') return { op: { k: 'selfDestroy' } };
    if (/^(?:このクリーチャーを)?自分の手札に戻す$/.test(s) || s === 'このクリーチャーを持ち主の手札に戻す') return { op: { k: 'selfHand' } };
    if (/^(?:ただし、)?その「S・トリガー」は使えない$/.test(s) && prev.noTrig) return { op: { k: 'nop' } };
    if (/^(?:S・トリガー|ブロッカー|スピードアタッカー|[WTQ]・ブレイカー|ワールド・ブレイカー)$/.test(s)) return { op: { k: 'nop' } };
    if (s === '自分の手札をすべて捨てる') return { op: { k: 'discardAll' } };
    if ((m = s.match(/^カードを(\d+)枚、自分のマナゾーンから墓地に置く$/))) return { op: { k: 'manaToGrave', n: N(m[1]) } };
    if ((m = s.match(/^このターン、自分のクリーチャー(\d+)体のパワーを([+-]\d+)する$/))) return { op: { k: 'seq', ops: [{ k: 'pick', side: 'p', f: {}, n: N(m[1]), min: N(m[1]), act: null, mine: true, buff: true }, { k: 'selPower', d: N(m[2]) }] }, sel: true };
    if ((m = s.match(new RegExp('^(パワー\\d+以下の|コスト\\d+以下の)?クリーチャーをすべて(' + ACT_RE + ')$')))) { const f = creFilter(m[1] || ''); if (!f) return null; return { op: { k: 'all', side: 'any', f, act: ACT[m[2]] } }; }
    // このクリーチャー
    if ((m = s.match(/^このクリーチャーを(アンタップ|タップ)する$/))) return { op: { k: m[1] === 'タップ' ? 'selfTap' : 'selfUntap' } };
    if ((m = s.match(/^このターン、このクリーチャーのパワーを([+-]\d+)する$/))) return { op: { k: 'selfPower', d: N(m[1]) } };
    if ((m = s.match(/^このターン、自分のクリーチャーすべてのパワーを([+-]\d+)する$/))) return { op: { k: 'allPower', d: N(m[1]) } };
    // クリーチャーを選ぶ
    const T = '(相手|自分)の(.*?)クリーチャー';
    if ((m = s.match(new RegExp('^' + T + 'を(\\d+)体(まで)?選び、(' + ACT_RE + ')$')))) {
      const f = creFilter(m[2]); if (!f) return null;
      return { op: { k: 'pick', side: who(m[1]), f, n: N(m[3]), min: m[4] ? 0 : N(m[3]), act: ACT[m[5]] } };
    }
    if ((m = s.match(new RegExp('^' + T + 'を(\\d+)体(まで)?(選ぶ|選んでもよい)$')))) {
      const f = creFilter(m[2]); if (!f) return null;
      return { op: { k: 'pick', side: who(m[1]), f, n: N(m[3]), min: (m[4] || m[5] !== '選ぶ') ? 0 : N(m[3]), act: null }, sel: true };
    }
    if ((m = s.match(new RegExp('^' + T + 'を(\\d+)体(' + ACT_RE + ')$')))) {
      const f = creFilter(m[2]); if (!f) return null;
      return { op: { k: 'pick', side: who(m[1]), f, n: N(m[3]), min: N(m[3]), act: ACT[m[4]] } };
    }
    if ((m = s.match(new RegExp('^' + T + 'をすべて(' + ACT_RE + ')$')))) {
      const f = creFilter(m[2]); if (!f) return null;
      return { op: { k: 'all', side: who(m[1]), f, act: ACT[m[3]] } };
    }
    if ((m = s.match(/^(?:その選んだクリーチャー|そのクリーチャー|それ|それら|選んだクリーチャー)を(.+)$/)) && prev.sel) {
      const a = ACT[m[1]]; if (a) return { op: { k: 'actSel', act: a } };
      return null;
    }
    if ((m = s.match(/^このターン、(?:その選んだクリーチャー|そのクリーチャー)のパワー(?:を|は)([+-]\d+)(?:する|される)$/)) && prev.sel) return { op: { k: 'selPower', d: N(m[1]) } };
    if ((m = s.match(/^このターン、(?:その選んだクリーチャー|そのクリーチャー)は攻撃(?:も)?できない$/)) && prev.sel) return { op: { k: 'selNoAttack' } };
    if ((m = s.match(/^次の相手のターンのはじめに、(?:その選んだクリーチャー|そのクリーチャー|それ|それら)はアンタップしない$/)) && prev.sel) return { op: { k: 'selFreeze' } };
    if ((m = s.match(/^(?:その選んだクリーチャーとこのクリーチャー|このクリーチャーとその選んだクリーチャー|このクリーチャーとそのクリーチャー)をバトルさせる$/)) && prev.sel) return { op: { k: 'battleSel' } };
    return null;
  }

  // 1つの能力の本文（「、」の後）を処理の列に
  function ability(body, modeLines) {
    body = body.replace(/（[^）]*）|\([^)]*\)/g, '').trim();
    const sens = body.replace(/ひとつ/g, '1つ').split('。').map(x => x.trim()).filter(x => x && !/^(?:マナゾーンに置く時、このカードはタップして置く|チャージャー|S・トリガー|スーパー・S・トリガー)$/.test(x));
    const ops = []; const prev = { looked: false, sel: false };
    for (let i = 0; i < sens.length; i++) {
      const s = sens[i];
      // モード選択
      const mm = s.match(/^次の(?:うち|中から)(?:いずれか)?(\d+)(?:つ|回)(?:まで)?を?選ぶ$/) || s.match(/^次の中から(\d+)つ選ぶ$/) || s.match(/^次の中から(\d+)回選ぶ$/);
      if (mm) {
        if (i !== sens.length - 1 || !modeLines || !modeLines.length) return null;
        const opts = [];
        for (const ln of modeLines) { const a = ability(ln.replace(/^[▶►・]\s*/, ''), null); if (!a) return null; opts.push({ label: ln.replace(/^[▶►・]\s*/, '').slice(0, 40), ops: a }); }
        ops.push({ k: 'mode', times: +mm[1], opts }); return ops;
      }
      const r = sentence(s, prev); if (!r) { if (root.AUTOFX_DEBUG) root.AUTOFX_DEBUG(s); return null; }
      if (r.op.k === 'look' || r.op.k === 'search') prev.looked = true;
      if (r.sel || (r.op.k === 'pick' && r.op.act)) prev.sel = true;
      if (r.op.k === 'pick' && r.op.act) prev.sel = true;
      const cond = /^(?:そうしたら|そうした場合)、/.test(s);
      if (r.op.k === 'oppShield' || r.op.k === 'shieldToHand') prev.noTrig = true;
      ops.push(Object.assign({}, r.op, r.opt ? { opt: true, text: s } : {}, cond ? { ifDid: true } : {}));
    }
    // 「見る」「表向きにする」の後に残りの行き先がなければ読み取らない（山札がずれる）
    if (prev.looked && !JSON.stringify(ops).includes('restLooked') && !ops.some(o => o.k === 'search')) {
      if (ops.some(o => o.k === 'look')) return null;
    }
    if (ops.some(o => o.k === 'search') && !ops.some(o => o.k === 'shuffle')) ops.push({ k: 'shuffle' });
    return ops;
  }

  // カードの効果文 → {etb:[{summonOnly, ops}], attack, destroyed, turnStart, turnEnd, spell, fail:{ev:true}}
  const HEADS = [
    [/^(?:このクリーチャー|このタマシード|このエレメント)が(召喚によって)?(?:バトルゾーンに)?出た時(?:または(?:このクリーチャーが)?攻撃する時)?、/, 'etb'],
    [/^(?:このクリーチャー|このタマシード|このエレメント)が(召喚によって)?(?:バトルゾーンに)?出た時、/, 'etb'],
    [/^このクリーチャーが攻撃する時、/, 'attack'],
    [/^(?:このクリーチャー|このタマシード)が破壊された時、/, 'destroyed'],
    [/^自分のターンのはじめに、/, 'turnStart'],
    [/^自分のターンの終わりに、/, 'turnEnd'],
  ];
  const KW = /^(?:スーパー・S・トリガー|S・トリガー|ブロッカー|スピードアタッカー|[WTQ]・ブレイカー|ワールド・ブレイカー|パワード・ブレイカー|ジャストダイバー|マッハファイター|スレイヤー|G・ストライク|シンパシー[^、。]*|\{[A-Za-z]+\}\s*)/;
  function parseText(text, type) {
    const out = { fail: {} };
    let t = nf(text).replace(/\{[A-Za-z]+\}\s*/g, '').replace(/S\(シールド\)・トリガー/g, 'S・トリガー')
      .replace(/\([^()]*\)/g, '').replace(/出たとき/g, '出た時').replace(/(?<!も)てよい/g, 'てもよい').replace(/このクリーチャーを(?:バトルゾーンに)?出した時、/g, 'このクリーチャーが出た時、')
      .replace(/相手の手札を見てその中から(\d+)枚選び、捨てさせる/g, '相手の手札を見て$1枚選び、捨てさせる');
    // キーワードの直後に能力が続いている行を分ける
    t = t.replace(/(ブロッカー|スピードアタッカー|[WTQ]・ブレイカー|ワールド・ブレイカー|ジャストダイバー|マッハファイター|スレイヤー|S・トリガー|G・ストライク)(?=このクリーチャー|自分の|相手の|このタマシード)/g, '$1\n');
    const lines = t.split('\n').map(x => x.trim()).filter(Boolean);
    const add = (ev, a, raw) => { if (!a) { out.fail[ev] = true; return; } (out[ev] ||= []).push(a); };
    if (/呪文/.test(type || '')) {
      const body = []; let modeLines = [];
      for (const ln of lines) {
        if (/^[▶►・]/.test(ln)) { modeLines.push(ln); continue; }
        if (/^(?:マナゾーンに置く時、このカードはタップして置く|チャージャー|スーパー・S・トリガー)$/.test(ln) || /^\(/.test(ln)) continue;
        let l = ln; while (KW.test(l)) l = l.replace(KW, '').trim();
        if (/^\(/.test(l)) continue;
        if (l) body.push(l);
      }
      const ops = body.length ? ability(body.join(''), modeLines) : null;
      add('spell', ops && { summonOnly: false, ops });
      return out;
    }
    for (let i = 0; i < lines.length; i++) {
      let ln = lines[i]; while (KW.test(ln)) ln = ln.replace(KW, '').trim();
      for (const [re, ev] of HEADS) {
        const m = ln.match(re); if (!m) continue;
        const body = ln.slice(m[0].length);
        const modeLines = []; for (let j = i + 1; j < lines.length && /^[▶►・]/.test(lines[j]); j++) modeLines.push(lines[j]);
        const ops = ability(body, modeLines);
        add(ev, ops && { summonOnly: !!m[1], ops });
        if (ev === 'etb' && /または(?:このクリーチャーが)?攻撃する時/.test(m[0])) add('attack', ops && { summonOnly: false, ops });
        break;
      }
    }
    return out;
  }
  // 常在のコスト軽減：「自分の〇〇を召喚するコストを1少なくする」（バトルゾーンにある時）
  function costReducer(text) {
    const t = nf(text);
    const m = t.match(/自分の(.*?)(クリーチャー|呪文)を(召喚する|唱える)(?:ための)?コストを(\d+)少なくする(?:。ただし、コストは(\d+)以下にはならない)?/);
    if (!m) return null;
    if (/(ターン|時、|なら|あれば|1体につき|枚につき|につき)/.test(m[0]) || /(?:^|\n)[^\n]*(?:時、|なら、)[^\n]*召喚するコストを/.test(t)) return null;
    let pre = m[1]; const f = { type: m[2] };
    const civ = pre.match(/^(光|水|闇|火|自然)の$/); const race = pre.match(/^([^の]+)・?$/);
    if (pre === '') {} else if (civ) f.civ = civ[1]; else if (race && !/(コスト|パワー)/.test(pre)) f.race = race[1].replace(/・$/, ''); else return null;
    return { f, n: +m[4], floor: m[5] ? +m[5] + 1 : 1 };
  }
  root.AUTOFX_PARSE = { parseText, costReducer, cardFilter, creFilter, sentence, ability };
  if (typeof module !== 'undefined') module.exports = root.AUTOFX_PARSE;
})(typeof window !== 'undefined' ? window : globalThis);
/* ---------- 効果文の自動読み取り：対戦卓での実行（2026-11-25） ----------
   専用の処理が無いカードは、AUTOFX_PARSE で読み取れた能力をここで実行する。読み取れなかった能力は今までどおり
   （AI対戦では「手で処理」の案内、Claude 対戦では Claude が処理）。設定「カード効果：全部手で処理」の時は何もしない。
   コスト軽減：「自分の〇〇を召喚するコストを1少なくする」を持つカードがバトルゾーンにあれば軽減する。
   簡易AIもコスト軽減を使って召喚する（今までは軽減を使わず、払えるのに出せないことがあった）。 */
(() => {
  const nofx = () => (typeof NOFX !== 'undefined' && NOFX);
  const P = window.AUTOFX_PARSE; if (!P) return;
  const wrap = (name, f) => { const orig = window[name]; if (typeof orig !== 'function') { console.warn('autofx: missing ' + name); return; } window[name] = f(orig); };
  const fm = () => !!(window.DMAI_FULL_MANUAL && window.DMAI_FULL_MANUAL());
  const cache = new Map();
  const sideOf = (c, i) => DB[c.cid].sides[i ?? c.side ?? 0] || DB[c.cid].sides[0];
  function parsedFor(c, i) {
    const s = sideOf(c, i); const k = c.cid + '|' + (i ?? c.side ?? 0);
    if (!cache.has(k)) { let r = null; try { r = P.parseText(s.text, s.type); } catch (e) { console.error(e); } cache.set(k, r); }
    return cache.get(k);
  }
  window.AUTOFX_FOR = (uid, i) => { const c = C(uid); return c ? parsedFor(c, i) : null; };
  const val = u => { const c = C(u); const s = side(c); return (s.cost || 0) * 10000 + (isCreature(s) && c.zone === 'battle' ? power(c) : (s.power || 0)); };

  function passF(u, f, x) {
    const c = C(u); if (!c) return false;
    if (!passFilter(u, f)) return false;
    if (f.other && u === x.src) return false;
    const inB = c.zone === 'battle';
    if (f.maxPower != null && !(inB && power(c) <= f.maxPower)) return false;
    if (f.minPower != null && !(inB && power(c) >= f.minPower)) return false;
    if (f.tapped && !c.tapped) return false;
    if (f.untapped && c.tapped) return false;
    return true;
  }
  async function act(x, us, a) {
    us = [].concat(us).filter(u => C(u)); if (!us.length) return;
    if (a === 'destroy') return x.destroy(us);
    if (a === 'tap') { us.forEach(u => C(u).tapped = true); return; }
    if (a === 'untap') { us.forEach(u => C(u).tapped = false); return; }
    for (const u of us) await x.move(u, a);
  }
  const names = us => us.map(u => '《' + cname(C(u)) + '》').join('');
  const isBot = p => p === 'op' && oppMode() !== 'online' && oppMode() !== 'manual';

  async function runOps(ops, x, env, depth = 0) {
    for (const o of ops) {
      if (S.winner) return;
      if (o.ifDid && !env.did) continue;
      if (o.opt && !(await x.optional(o.text || 'この効果を使いますか？'))) { env.did = false; continue; }
      env.did = true;
      await op(o, x, env, depth);
      render();
    }
  }
  async function op(o, x, env, depth) {
    const p = x.p, q = x.q;
    switch (o.k) {
      case 'nop': return;
      case 'seq': return runOps(o.ops, x, env, depth + 1);
      case 'draw': return x.draw(o.n);
      case 'discard': {
        const h = zoneArr(p, 'hand').slice(); if (!h.length) { env.did = false; return; }
        const ps = await x.choose({ q: `捨てる手札（${o.n}枚）`, cands: h, min: Math.min(o.n, h.length), max: o.n, score: u => -val(u) });
        for (const u of ps) place(u, 'grave'); if (ps.length) log(`${PN[p]}：${names(ps)}を捨てた`, p); env.did = ps.length > 0; return;
      }
      case 'topMana': return x.topToMana(o.n, o.tapped);
      case 'mill': { const L = zoneArr(p, 'deck').slice(0, o.n); for (const u of L) place(u, 'grave'); log(`${PN[p]}：山札の上から${L.length}枚を墓地へ`, p); return; }
      case 'topShield': return x.topToShield(o.n);
      case 'look': env.L = x.look(o.n); return;
      case 'search': env.L = zoneArr(p, 'deck').slice(); log(`${PN[p]}：山札を見た`, p); return;
      case 'shuffle': return x.shuffle();
      case 'pickLooked': {
        const L = (env.L || []).filter(u => C(u) && C(u).zone === 'deck');
        const cands = L.filter(u => passF(u, o.f, x));
        const ps = cands.length ? await x.choose({ q: `${o.dest === 'battle' ? '出す' : o.dest === 'hand' ? '手札に加える' : '置く'}カード（${o.min ? o.n + '枚' : o.n === 99 ? '好きな枚数' : o.n + '枚まで'}）`, cands, min: Math.min(o.min, cands.length), max: o.n, score: val }) : [];
        if (ps.length) { log(`${PN[p]}：${p === 'me' || o.dest !== 'hand' ? names(ps) : ps.length + '枚'}を${o.dest === 'battle' ? 'バトルゾーンに出す' : o.dest === 'hand' ? '手札に加える' : ZL[o.dest] + 'へ'}`, p); if (o.dest === 'battle') await x.put(ps); else await x.move(ps, o.dest); }
        env.did = ps.length > 0; return;
      }
      case 'restLooked': {
        const rest = (env.L || []).filter(u => C(u) && C(u).zone === 'deck');
        if (rest.length) { await x.move(rest, o.dest === 'mana' ? 'mana' : o.dest); log(`残り${rest.length}枚を${o.dest === 'deckBottom' ? '山札の下' : ZL[o.dest] || o.dest}へ`, p); }
        env.L = []; return;
      }
      case 'oppDiscard': {
        const h = zoneArr(q, 'hand').slice(); if (!h.length) return;
        const ps = await x.choose({ q: `捨てる手札（${o.n}枚）`, cands: h, min: Math.min(o.n, h.length), max: o.n, by: q, score: u => -val(u) });
        for (const u of ps) place(u, 'grave'); log(`${PN[q]}：${names(ps)}を捨てた`, q); return;
      }
      case 'randDiscard': {
        const ps = []; for (let i = 0; i < o.n; i++) { const h = zoneArr(q, 'hand'); if (!h.length) break; const u = h[Math.floor(Math.random() * h.length)]; place(u, 'grave'); ps.push(u); }
        if (ps.length) log(`${PN[q]}の手札から${names(ps)}を捨てさせた`, p); return;
      }
      case 'peekDiscard': {
        const h = zoneArr(q, 'hand').slice(); if (!h.length) return;
        let ps;
        if (p === 'me' && oppMode() !== 'online' && !(window.DMAI && window.DMAI.selfplay)) { const r = await modal({ title: `《${x.name}》`, html: `<p>相手の手札です。捨てさせる${o.n}枚を選んでください。</p>`, pick: { uids: h, min: Math.min(o.n, h.length), max: o.n, hide: () => false }, buttons: [{ label: '決定', value: 1, primary: true, needPick: true }] }); ps = (r && r.picked || []).slice(0, o.n); }
        else ps = await x.choose({ q: '捨てさせる手札', cands: h, min: Math.min(o.n, h.length), max: o.n, score: val });
        for (const u of ps) place(u, 'grave'); if (ps.length) log(`${PN[q]}の手札を見て${names(ps)}を捨てさせた`, p); return;
      }
      case 'zoneToHand': {
        const cs = zoneArr(p, o.zone).filter(u => passF(u, o.f, x)); if (!cs.length) { env.did = false; return; }
        const ps = await x.choose({ q: `${ZL[o.zone]}から手札に戻すカード`, cands: cs, min: Math.min(o.min, cs.length), max: o.n, score: val });
        for (const u of ps) place(u, 'hand'); if (ps.length) log(`${PN[p]}：${ZL[o.zone]}から${names(ps)}を手札へ`, p); env.did = ps.length > 0; return;
      }
      case 'putFromHand': {
        const cs = zoneArr(p, 'hand').filter(u => { const s = DB[C(u).cid].sides[0]; return passF(u, o.f, x) && isCreature(s) && !(/進化/.test(s.type) && !/NEO/.test(s.type)); });
        if (!cs.length) { env.did = false; return; }
        const ps = await x.choose({ q: '手札から出すクリーチャー', cands: cs, min: Math.min(o.min, cs.length), max: o.n, score: val });
        if (ps.length) { log(`${PN[p]}：手札から${names(ps)}を出す`, p); await x.put(ps); } env.did = ps.length > 0; return;
      }
      case 'handToMana': {
        const h = zoneArr(p, 'hand').slice(); if (!h.length) { env.did = false; return; }
        const ps = await x.choose({ q: `マナゾーンに置く手札（${o.n}枚）`, cands: h, min: Math.min(o.n, h.length), max: o.n, score: u => -val(u) });
        for (const u of ps) place(u, 'mana'); if (ps.length) log(`${PN[p]}：手札から${ps.length}枚をマナへ`, p); env.did = ps.length > 0; return;
      }
      case 'untapMana': zoneArr(p, 'mana').forEach(u => C(u).tapped = false); log(`${PN[p]}：マナをすべてアンタップ`, p); return;
      case 'shieldToHand': case 'shieldToGrave': {
        const sh = zoneArr(p, 'shield').slice(); if (!sh.length) { env.did = false; return; }
        const ps = await x.choose({ q: `${o.k === 'shieldToHand' ? '手札に加える' : '墓地に置く'}自分のシールド（${o.n}つ）`, cands: sh, min: Math.min(o.n, sh.length), max: o.n, score: () => Math.random() });
        for (const u of ps) place(u, o.k === 'shieldToHand' ? 'hand' : 'grave'); log(`${PN[p]}：シールドを${ps.length}つ${o.k === 'shieldToHand' ? '手札' : '墓地'}へ`, p); return;
      }
      case 'oppShield': {
        const sh = zoneArr(q, 'shield').slice(); if (!sh.length) { env.did = false; return; }
        const ps = await x.choose({ q: `${PN[q]}のシールドを選ぶ（${o.n}つ）`, cands: sh, min: Math.min(o.n, sh.length), max: o.n, score: () => Math.random() });
        for (const u of ps) place(u, o.dest); log(`${PN[q]}のシールドを${ps.length}つ${o.dest === 'hand' ? '手札' : '墓地'}へ`, p); return;
      }
      case 'selfTap': case 'selfUntap': { const c = C(x.src); if (c && c.zone === 'battle') c.tapped = o.k === 'selfTap'; return; }
      case 'selfPower': { const c = C(x.src); if (c && c.zone === 'battle') c.m.powerT = (c.m.powerT || 0) + o.d; return; }
      case 'allPower': inBattle(p, c => isCreature(side(c))).forEach(c => c.m.powerT = (c.m.powerT || 0) + o.d); return;
      case 'selfDestroy': return x.destroy(x.src);
      case 'selfHand': { const c = C(x.src); if (c && c.zone === 'battle') await x.move(x.src, 'hand'); return; }
      case 'discardAll': { const h = zoneArr(p, 'hand').slice(); for (const u of h) place(u, 'grave'); if (h.length) log(`${PN[p]}：手札を${h.length}枚すべて捨てた`, p); env.did = h.length > 0; return; }
      case 'manaToGrave': {
        const mn = zoneArr(p, 'mana').slice(); if (!mn.length) { env.did = false; return; }
        const ps = await x.choose({ q: `墓地に置く自分のマナ（${o.n}枚）`, cands: mn, min: Math.min(o.n, mn.length), max: o.n, score: u => -val(u) });
        for (const u of ps) place(u, 'grave'); if (ps.length) log(`${PN[p]}：マナから${names(ps)}を墓地へ`, p); return;
      }
      case 'pick': case 'all': {
        const sides = o.side === 'any' ? [q, p] : [o.side === 'q' ? q : p];
        const pool = sides.flatMap(w => o.f.elem ? x.elems(w) : x.creatures(w));
        const cs = pool.filter(u => passF(u, o.f, x));
        if (!cs.length) { env.X = []; env.did = false; return; }
        let ps;
        if (o.k === 'all') ps = cs;
        else {
          const by = o.by === 'q' ? q : p;
          // 選ぶ人から見て、相手のものは強い順・自分のものは弱い順
          const score = u => (o.buff ? 1 : C(u).owner === by ? -1 : 1) * val(u) + (o.act === 'tap' && C(u).tapped ? -1e9 : 0);
          ps = await x.choose({ q: `${o.act ? ({ destroy: '破壊する', hand: '手札に戻す', tap: 'タップする', untap: 'アンタップする', mana: 'マナゾーンに置く', grave: '墓地に置く', deckBottom: '山札の下に置く', shield: 'シールド化する' }[o.act]) : '選ぶ'}クリーチャー（${o.min ? o.n + '体' : o.n + '体まで'}）`, cands: cs, min: Math.min(o.min, cs.length), max: o.n, by, score });
        }
        env.X = ps; env.did = ps.length > 0;
        if (o.act && ps.length) { log(`${names(ps)}を${({ destroy: '破壊', hand: '手札に戻す', tap: 'タップ', untap: 'アンタップ', mana: 'マナへ', grave: '墓地へ', deckBottom: '山札の下へ', shield: 'シールド化' }[o.act])}`, p); await act(x, ps, o.act); }
        return;
      }
      case 'actSel': if (env.X && env.X.length) { log(`${names(env.X)}を処理`, p); await act(x, env.X, o.act); } else env.did = false; return;
      case 'selPower': for (const u of env.X || []) { const c = C(u); if (c && c.zone === 'battle') { c.m.powerT = (c.m.powerT || 0) + o.d; if (power(c) <= 0) await x.destroy(u); } } return;
      case 'selNoAttack': for (const u of env.X || []) if (C(u)) C(u).m.noAttack = true; return;
      case 'selFreeze': for (const u of env.X || []) if (C(u)) { C(u).tapped = true; C(u).m.frozen = true; } return;
      case 'battleSel': { const t = (env.X || [])[0]; const a = C(x.src); if (t && a && a.zone === 'battle' && C(t) && C(t).zone === 'battle') await battle(x.src, t); return; }
      case 'mode': {
        const labels = o.opts.map(m => m.label);
        for (let k = 0; k < o.times; k++) {
          let i;
          if (isBot(p)) {
            const sc = m => m.ops.reduce((a, s) => a + ((s.k === 'pick' || s.k === 'all') ? ((s.side === 'q' || s.side === 'any') && x.creatures(q).length ? 3 : 0) : s.k === 'draw' ? 2 : 1), 0);
            i = o.opts.map((m, j) => [sc(m), j]).sort((a, b) => b[0] - a[0])[0][1];
            log(`相手：《${x.name}》のモード「${labels[i]}」`, p);
          } else i = await x.mode(`次の中から選ぶ（${k + 1}/${o.times}）`, labels);
          await runOps(o.opts[i]?.ops || [], x, env, depth + 1);
        }
        return;
      }
    }
  }
  window.AUTOFX_RUN = async (uid, ev, extra = {}) => {
    const c = C(uid); if (!c) return false;
    const r = parsedFor(c, extra.sideIdx); const list = r && r[ev]; if (!list || !list.length || r.fail[ev]) return false;
    const x = makeCtx(uid, c.owner, extra);
    const s = sideOf(c, extra.sideIdx);
    let ran = false;
    for (const a of list) {
      if (a.summonOnly && extra.how !== 'summon') continue;
      ran = true;
      log(`《${s.name}》の${EVLABEL[ev] || ev}の能力（効果文から自動処理）`, c.owner);
      try { await runOps(a.ops, x, {}); } catch (e) { console.error(e); log('効果処理エラー：' + e.message); }
    }
    return ran || true;
  };
  const EVS = ['etb', 'spell', 'attack', 'destroyed', 'turnStart', 'turnEnd', 'manaLeave', 'rev0', 'allyEnter', 'oppEnter', 'allyDragon', 'shieldPlaced', 'attackEnd'];
  // 専用の処理が無いカード：読み取れた能力があれば「専用の処理」の代わりを返す。読み取れなかった能力は元の流れ（Claude／手で処理）へ
  wrap('scriptFor', o => function (uid) {
    const sc = o.apply(this, arguments); if (sc) return sc;
    const c = C(uid); if (!c || c.cid === HIDDEN_CID || NET.on || nofx() || fm()) return null;
    const r = parsedFor(c); if (!r) return null;
    const anyOk = DB[c.cid].sides.some((_, i) => { const p = parsedFor(c, i); return p && EVS.some(e => p[e] && p[e].length && !p.fail[e]); });
    if (!anyOk) return null;
    const out = {};
    for (const ev of EVS) out[ev] = async x => {
      const cc = C(uid); const si = cc ? (cc.side || 0) : 0; const pr = parsedFor(cc, si);
      if (pr && pr[ev] && pr[ev].length && !pr.fail[ev]) { await window.AUTOFX_RUN(uid, ev, { how: x.how, sideIdx: si, evolved: x.evolved }); loseCheck(); return; }
      // 元の流れ（resolveEffect の専用処理が無い時と同じ）
      const s = sideOf(cc, si);
      if (ev !== 'spell' && ev !== 'manaLeave' && !(TRIG[ev] || /^$/).test(s.text)) return;
      if (ev === 'etb' && x.how !== 'summon' && /召喚によって(バトルゾーンに)?出た時/.test(s.text) && !TRIG.etb.test(s.text.replace(/召喚によって(バトルゾーンに)?出た時/g, ''))) return;
      await claudeResolve(uid, ev, s, { how: x.how, sideIdx: si });
      loseCheck();
    };
    out.autofx = true;
    return out;
  });

  // 常在のコスト軽減
  const redCache = new Map();
  const reducerOf = c => { if (!redCache.has(c.cid)) redCache.set(c.cid, P.costReducer(side(c).text)); return redCache.get(c.cid); };
  wrap('costMods', o => function (p, c, s, baseUid) {
    const r = o.apply(this, arguments);
    if (nofx() || fm() || !s) return r;
    let red = 0, floor = 1; const notes = r.notes.slice();
    for (const b of inBattle(p, x => x.uid !== c.uid)) {
      const cr = reducerOf(b); if (!cr) continue;
      if (cr.f.type === 'クリーチャー' && !isCreature(s)) continue;
      if (cr.f.type === '呪文' && s.type !== '呪文') continue;
      if (cr.f.civ && !(s.civs || DB[c.cid].civs).includes(cr.f.civ)) continue;
      if (cr.f.race && !String(DB[c.cid].race || '').includes(cr.f.race)) continue;
      red += cr.n; floor = Math.max(floor, cr.floor); notes.push(`《${cname(b)}》 −${cr.n}`);
    }
    if (!red) return r;
    return { cost: Math.max(Math.min(floor, r.cost), r.cost - red), notes };
  });
  // 簡易AI：コストを決めずに使うカードはコスト軽減をかけて払う
  wrap('useCard', o => async function (uid, opt = {}) {
    const c = C(uid);
    if (c && c.owner === 'op' && c.zone === 'hand' && !opt.free && !opt.paid && !nofx() && !fm() && oppMode() !== 'online') {
      const s = DB[c.cid].sides[opt.sideIdx || 0];
      if (s && s.type !== '呪文' && (opt.cost === undefined || opt.cost === s.cost)) { const m = costMods('op', c, s, opt.base); if (m.cost < s.cost) { opt = Object.assign({}, opt, { cost: m.cost }); log(`相手：《${s.name}》をコスト${s.cost}→${m.cost}で召喚（${m.notes.join('、')}）`, 'op'); } }
    }
    return o.call(this, uid, opt);
  });
  // 簡易AIの「払えるか」の見積もりにもコスト軽減を入れる
  let inBot = 0;
  wrap('payPlan', o => function (p, civs, cost, uid) {
    if (inBot && p === 'op' && uid && C(uid) && C(uid).zone === 'hand' && !nofx()) {
      const c = C(uid); const s = side(c);
      if (s && s.type !== '呪文' && cost === s.cost) cost = costMods('op', c, s).cost;
    }
    return o.call(this, p, civs, cost, uid);
  });
  wrap('botTurn', o => function () { inBot++; try { return o.apply(this, arguments); } finally { inBot--; } });
})();
/* ---------- ジョーカーズのカード（2026-11-25） ----------
   効果文の自動読み取りでは読めない、ジョーカーズ（学習用デッキ）の能力。学習エンジンの実装と同じ処理（裁定は alphazero/裁定メモ.md）。
   まだ無いもの：J・O・E（コストを少なくして召喚）、ブランドの G・G・G、富士山ッピングの手札から出る能力と離れた時の能力、
   アイアン・マンハッタンのブレイク、ジョギラゴンの呪文制限。 */
(() => {
  const nofx = () => (typeof NOFX !== 'undefined' && NOFX);
  const s0 = u => DB[C(u).cid].sides[0];
  const isJ = u => /ジョーカーズ/.test(DB[C(u).cid].race || '') || DB[C(u).cid].sides.some(s => /ジョーカーズ/.test(s.race || ''));
  const isCre = u => isCreature(s0(u));
  const val = u => (side(C(u)).cost || 0) * 10000 + (C(u).zone === 'battle' ? power(C(u)) : (s0(u).power || 0));
  const names = us => us.map(u => '《' + cname(C(u)) + '》').join('');
  const jCount = p => inBattle(p, c => isJ(c.uid)).length + zoneArr(p, 'mana').filter(isJ).length;
  const putable = u => { const s = s0(u); return isCreature(s) && !(/進化/.test(s.type) && !/NEO/.test(s.type)); };
  const bottom = async (x, L) => { const rest = L.filter(u => C(u) && C(u).zone === 'deck'); if (rest.length) { await x.move(rest, 'deckBottom'); log(`残り${rest.length}枚を山札の下へ`, x.p); } };
  const etbAbility = u => { const c = C(u); if (!c) return false; const sc = scriptFor(u); if (sc && typeof sc.etb === 'function' && !sc.autofx) return true; const r = window.AUTOFX_FOR && window.AUTOFX_FOR(u, 0); return !!(r && r.etb && r.etb.length && !r.fail.etb); };
  async function useEtbOf(x, u, why) {
    log(`${why}：《${cname(C(u))}》の出た時の能力を使う`, x.p);
    await resolveEffect(u, 'etb', { how: 'summon' });
  }

  Object.assign(SCRIPTS, {
    'Dr.富士山ッピング': { async etb(x) {
      const L = x.look(3); const js = L.filter(isJ);
      if (js.length) { await x.move(js, 'hand'); log(`富士山ッピング：${names(js)}を手札へ`, x.p); }
      await bottom(x, L);
    }},
    'キング・ザ・スロットン７': { async etb(x) {
      const L = x.look(3);
      if (L.length === 3 && L.every(isJ)) {
        const cs = L.filter(u => isCre(u) && putable(u));
        const [u] = await x.choose({ q: '出すクリーチャー（そのターン、相手プレイヤーを攻撃できる）', cands: cs, min: Math.min(1, cs.length), max: 1, score: val });
        if (u) { log(`スロットン７：3枚ともジョーカーズ。《${cname(C(u))}》を出す`, x.p); await x.put([u]); const c = C(u); if (c && c.zone === 'battle') c.sick = false; }
      } else log('スロットン７：3枚すべてがジョーカーズではなかった', x.p);
      await bottom(x, L);
    }},
    '“↑↑”ブランド': { async etb(x) {
      const h = zoneArr(x.p, 'hand').slice(); const ts = () => x.creatures(x.q).filter(u => power(C(u)) <= 6000);
      if (!h.length || !ts().length) return;
      const n = x.p === 'op' ? Math.min(h.length, ts().length) : h.length;
      const ps = await x.choose({ q: '捨てる手札（1枚につき、相手のパワー6000以下のクリーチャーを1体破壊）', cands: h, min: 0, max: n, score: u => -val(u) });
      for (const u of ps) place(u, 'grave'); if (ps.length) log(`ブランド：${names(ps)}を捨てた`, x.p);
      for (let i = 0; i < ps.length; i++) { const cs = ts(); if (!cs.length) break; const [t] = await x.choose({ q: '破壊する相手のパワー6000以下のクリーチャー', cands: cs, min: 1, max: 1, score: val }); if (t) { log(`ブランド：《${cname(C(t))}》を破壊`, x.p); await x.destroy(t); } }
    }},
    'ジョギメラゴン＆メラビート': { async etb(x) {
      const L = x.look(4); const js = L.filter(isJ);
      const [u] = await x.choose({ q: '手札に加えるジョーカーズ・カード', cands: js, min: 0, max: 1, score: val });
      if (u) { await x.move(u, 'hand'); log(`ジョギメラゴン：《${cname(C(u))}》を手札へ`, x.p); }
      await bottom(x, L);
    }},
    'メラベンチャー・ザ・ジョニー': {
      async etb(x) {
        const cs = zoneArr(x.p, 'hand').filter(u => isJ(u) && putable(u));
        const ps = await x.choose({ q: '手札から出すジョーカーズ・クリーチャー（2体まで。スピードアタッカー、ターンの終わりに山札の下へ置き1枚引く）', cands: cs, min: 0, max: 2, score: val });
        if (!ps.length) return;
        log(`メラベンチャー：${names(ps)}を出す`, x.p); await x.put(ps);
        for (const u of ps) { const c = C(u); if (c && c.zone === 'battle') { c.sick = false; c.m.sa = true; c.m.melaEOT = true; } }
        (S.melaEOT ||= []).push(...ps.filter(u => C(u) && C(u).zone === 'battle').map(u => [u, x.p]));
      },
      async attack(x) {
        if (jCount(x.p) < 7) return;
        const bs = inBattle(x.q, c => isBlocker(c)).map(c => c.uid); if (!bs.length) return;
        log(`メラベンチャー：ジョーカーズが7つ以上。${names(bs)}（ブロッカー）をすべて破壊`, x.p); await x.destroy(bs);
      },
    },
    'ジョギラゴン＆ジョニー': { async etb(x) {
      x.draw(1);
      const h = zoneArr(x.p, 'hand').slice(); if (!h.length) return;
      const [d] = await x.choose({ q: '捨てる手札（ジョーカーズなら、その「出た時」の能力を1つ使える）', cands: h, min: 1, max: 1, score: u => (isJ(u) && etbAbility(u) ? 1e6 : 0) - val(u) });
      if (!d) return; place(d, 'grave'); log(`ジョギラゴン：《${cname(C(d))}》を捨てた`, x.p);
      if (isJ(d) && etbAbility(d) && await x.optional(`《${cname(C(d))}》の「出た時」の能力を使いますか？`)) await useEtbOf(x, d, 'ジョギラゴン');
    }},
    '勝敵逆夢王 モウジャキンガ': {
      async etb(x) {
        const out = [];
        const [sh] = await x.choose({ q: '墓地に置かせる相手のシールド', cands: zoneArr(x.q, 'shield').slice(), min: 1, max: 1, score: () => Math.random() }); if (sh) out.push(sh);
        const [cr] = await x.choose({ q: '墓地に置かせる相手のクリーチャー', cands: x.creatures(x.q), min: 1, max: 1, score: val }); if (cr) out.push(cr);
        const h = zoneArr(x.q, 'hand'); if (h.length) out.push(h[Math.floor(Math.random() * h.length)]);
        if (!out.length) return;
        log(`モウジャキンガ：相手のシールド・クリーチャー・手札をそれぞれ1枚墓地へ`, x.p);
        for (const u of out) { if (C(u).zone === 'battle') await leave(u, 'grave'); else place(u, 'grave'); }
      },
      async attackEnd(x) {
        x.draw(1);
        const h = zoneArr(x.p, 'hand').slice(); if (!h.length) return;
        const [d] = await x.choose({ q: '捨てる手札（多色ならこのクリーチャーをアンタップ）', cands: h, min: 1, max: 1, score: u => (DB[C(u).cid].civs.length > 1 ? 1e6 : 0) - val(u) });
        if (!d) return; const multi = DB[C(d).cid].civs.length > 1; place(d, 'grave'); log(`モウジャキンガ：《${cname(C(d))}》を捨てた`, x.p);
        const c = C(x.src); if (multi && c && c.zone === 'battle') { c.tapped = false; log('モウジャキンガ：多色を捨てたのでアンタップ', x.p); }
      },
    },
    'にやにYAH': { async etb(x) {
      const others = inBattle(x.p, c => c.uid !== x.src && isJ(c.uid) && !/にやにYAH/.test(cname(c)) && etbAbility(c.uid)).map(c => c.uid);
      const labels = ['相手のクリーチャーをすべてタップする', 'Jビッグ1（自分の他のジョーカーズの出た時の能力を1つ使う）'];
      let i;
      if (x.p === 'op' && oppMode() !== 'online' && oppMode() !== 'manual') i = (x.creatures(x.q).some(u => !C(u).tapped) || !others.length) ? 0 : 1;
      else i = await x.mode('次の中から1回選ぶ', labels);
      if (i === 0) { x.creatures(x.q).forEach(u => C(u).tapped = true); log('にやにYAH：相手のクリーチャーをすべてタップ', x.p); return; }
      const [u] = await x.choose({ q: 'Jビッグ1：能力を使うジョーカーズ', cands: others, min: Math.min(1, others.length), max: 1, score: val });
      if (u) await useEtbOf(x, u, 'にやにYAH（Jビッグ1）');
    }},
    '勝熱と弾丸と自由の決断': { async spell(x) {
      const labels = ['コスト3以下のカードを1枚、持ち主の山札の下へ', 'このターン、自分のクリーチャー1体は相手プレイヤーを攻撃できる', '相手のクリーチャー1体の能力をこのターン無視（手で処理）'];
      for (let k = 0; k < 2; k++) {
        let i;
        if (x.p === 'op' && oppMode() !== 'online' && oppMode() !== 'manual') { const t = [...x.elems(x.q)].filter(u => side(C(u)).cost <= 3); i = t.length ? 0 : 1; }
        else i = await x.mode(`${k + 1}回目を選ぶ（同じものを選んでもよい）`, labels);
        if (i === 0) {
          const cs = [...x.elems(x.q), ...x.elems(x.p)].filter(u => side(C(u)).cost <= 3);
          const [u] = await x.choose({ q: '山札の下に置くコスト3以下のカード', cands: cs, min: Math.min(1, cs.length), max: 1, score: u => (C(u).owner === x.p ? -1 : 1) * val(u) });
          if (u) { log(`勝熱と弾丸と自由の決断：《${cname(C(u))}》を山札の下へ`, x.p); await x.move(u, 'deckBottom'); }
        } else if (i === 1) {
          const cs = x.creatures(x.p).filter(u => C(u).sick);
          const [u] = await x.choose({ q: 'このターン相手プレイヤーを攻撃できるクリーチャー', cands: cs, min: Math.min(1, cs.length), max: 1, score: val });
          if (u) { C(u).sick = false; log(`《${cname(C(u))}》はこのターン攻撃できる`, x.p); }
        } else {
          if (x.p === 'me') S.pend.me.push({ kind: 'note', src: x.src, text: '勝熱と弾丸と自由の決断：選んだ相手のクリーチャーの能力を、このターン無視してください。' });
          log('勝熱と弾丸と自由の決断：能力の無視は手で処理', x.p);
        }
      }
    }},
    '最終兵ッキー': {
      async etb(x) { await sakki(x); },
      async attack(x) { await sakki(x); },
    },
  });
  async function sakki(x) {
    const bot = x.p === 'op' && oppMode() !== 'online' && oppMode() !== 'manual';
    const cs = (bot ? x.creatures(x.q) : [...x.creatures(x.q), ...x.creatures(x.p)]).filter(u => u !== x.src);
    const [t] = await x.choose({ q: '持ち主の山札の下に置くクリーチャー（選ばなくてもよい）', cands: cs, min: 0, max: 1, score: u => (C(u).owner === x.p ? -1e9 : 0) + val(u) });
    if (!t) return;
    const owner = C(t).owner; const cost = side(C(t)).cost;
    log(`最終兵ッキー：《${cname(C(t))}》を持ち主の山札の下へ。持ち主は1枚引く`, x.p);
    await x.move(t, 'deckBottom'); drawN(owner, 1);
    const hs = zoneArr(owner, 'hand').filter(u => putable(u) && s0(u).cost <= cost);
    if (!hs.length) return;
    const y = makeCtx(x.src, owner);
    const [u] = await y.choose({ q: `手札から出すコスト${cost}以下のクリーチャー（出さなくてもよい）`, cands: hs, min: 0, max: 1, score: val });
    if (u) { log(`${PN[owner]}：《${cname(C(u))}》を出す`, owner); await y.put([u]); }
  }

  // スマッシュ・バースト（ザ・マグニー）：攻撃する時、呪文側「相手のコスト3以下のエレメントを1つ破壊」をバトルゾーンに置いたまま唱えてもよい
  SCRIPTS['銃車逆夢 ザ・マグニー'] = { async attack(x) {
    const cs = x.elems(x.q).filter(u => side(C(u)).cost <= 3); if (!cs.length) return;
    if (!await x.optional('スマッシュ・バースト：《ジョリー・ザ・スクラッパー》を唱えますか？（相手のコスト3以下のエレメントを1つ破壊）')) return;
    const [t] = await x.choose({ q: '破壊する相手のコスト3以下のエレメント', cands: cs, min: 1, max: 1, score: val });
    if (t) { log(`ジョリー・ザ・スクラッパー：《${cname(C(t))}》を破壊`, x.p); await x.destroy(t); }
  }};
  SCRIPTS['ジョリー・ザ・スクラッパー'] = { async spell(x) {
    const cs = x.elems(x.q).filter(u => side(C(u)).cost <= 3);
    const [t] = await x.choose({ q: '破壊する相手のコスト3以下のエレメント', cands: cs, min: Math.min(1, cs.length), max: 1, score: val });
    if (t) { log(`ジョリー・ザ・スクラッパー：《${cname(C(t))}》を破壊`, x.p); await x.destroy(t); }
  }};

  const wrap = (name, f) => { const orig = window[name]; if (typeof orig !== 'function') return; window[name] = f(orig); };
  // ジョギメラゴン：ジョーカーズとマナのジョーカーズ・カードが合計4つ以上なら召喚コスト −3
  wrap('costMods', o => function (p, c, s) {
    const r = o.apply(this, arguments);
    if (!nofx() && s && /ジョギメラゴン＆メラビート/.test(s.name) && jCount(p) >= 4) return { cost: Math.max(1, r.cost - 3), notes: r.notes.concat('ジョーカーズ4つ以上 −3') };
    return r;
  });
  // モウジャキンガの攻撃の終わり
  wrap('resolveAttack', o => async function (a) {
    const r = await o.apply(this, arguments);
    const c = a && C(a.attUid);
    if (!nofx() && !S.winner && c && c.zone === 'battle' && cname(c) === '勝敵逆夢王 モウジャキンガ') { await resolveEffect(c.uid, 'attackEnd'); await drain(); }
    return r;
  });
  // メラベンチャーで出したクリーチャー：ターンの終わりに山札の下へ。置いた数だけ引く
  wrap('endTurn', o => async function (p) {
    try {
      const L = (S.melaEOT || []).filter(([u, w]) => w === p); S.melaEOT = (S.melaEOT || []).filter(([u, w]) => w !== p);
      const us = L.map(([u]) => u).filter(u => C(u) && C(u).zone === 'battle');
      if (us.length && !S.winner) {
        const x = makeCtx(us[0], p); await x.move(us, 'deckBottom');
        drawN(p, us.length); log(`メラベンチャー：${us.length}体を山札の下へ置き、${us.length}枚引いた`, p);
      }
    } catch (e) { console.error(e); }
    return o.apply(this, arguments);
  });
})();
