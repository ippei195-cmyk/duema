
/* ===== 処理の変更：条件・勝利・クリーチャーを出す（GitHub duema PR #12 と同じ、PC用 2026-11-26） ===== */
(function(){
if (typeof RULES==='undefined' || typeof runSteps!=='function' || typeof addStepWizard!=='function'){ console.warn('condfx: 処理の変更の仕組みがありません'); return; }
if (window.__condfx) return; window.__condfx = true;
EVLABEL.cond = '条件を満たした時'; if (!RULE_EVS.includes('cond')) RULE_EVS.push('cond');
const CZONE = {mana:'マナ', hand:'手札', grave:'墓地', battle:'バトルゾーン', shield:'シールド', deck:'山札'};
const COPN = {'>=':'以上', '<=':'以下', '==':'ちょうど'};
function condCount(cd, p){ const who = cd.who==='opp' ? other(p) : p; return zoneArr(who, cd.zone||'mana').filter(u => C(u) && passFilter(u, cd.filter)).length; }
function condOk(cd, p){ const k = condCount(cd, p), n = Number(cd.n||0); return cd.op==='<=' ? k<=n : cd.op==='==' ? k===n : k>=n; }
function condsOk(conds, p){ return (conds||[]).every(cd => condOk(cd, p)); }
function condLine(cd){ return `${cd.who==='opp'?'相手':'自分'}の${CZONE[cd.zone]||cd.zone}の${cd.filter?filterName(cd.filter):'カード'}が${Number(cd.n||0)}枚${COPN[cd.op]||'以上'}`; }
function condsLine(conds){ return (conds||[]).map(condLine).join('、かつ') || '（条件なし）'; }

window.condsOk = condsOk; window.condsLine = condsLine;
const norm = f => String(f).replace(/\s+/g,'');
const OLD = {"stepsHTML": "function stepsHTML(steps, depth=0){\n  if (!steps || !steps.length) return depth ? '' : '<div class=\"rs\">（何もしない）</div>';\n  return steps.map((st,i) => `<div class=\"rs\" style=\"padding-left:${depth*14}px\">${depth?'・':(i+1)+'. '}${esc(stepLine(st))}</div>` +\n    (st.do==='optional' ? stepsHTML(st.steps, depth+1) : '') +\n    (st.do==='mode' ? (st.options||[]).map(o => `<div class=\"rs\" style=\"padding-left:${(depth+1)*14}px\">［${esc(o.label)}］</div>` + stepsHTML(o.steps, depth+2)).join('') : '')).join('');\n}", "addStepWizard": "async function addStepWizard(steps){\n  const used = new Set(); (function walk(a){ for (const s of a||[]){ if (s.as) used.add(s.as); walk(s.steps); (s.options||[]).forEach(o=>walk(o.steps)); } })(steps);\n  const nextVar = () => { let i=1; while (used.has('X'+i)) i++; return 'X'+i; };\n  const kind = await modal({title:'手順を追加', html:'<p>何をしますか？</p>', buttons:[\n    {label:'カードを引く', value:'draw'}, {label:'山札の上から何枚か見る（表向きにする）', value:'look'},\n    {label:'カードを選んで移動・破壊など', value:'choose'}, {label:'条件に合うカードすべてを移動・破壊など', value:'all'},\n    {label:'見たカードの残りを移動', value:'rest'}, {label:'山札の上からマナ／シールドへ', value:'top'},\n    {label:'山札をシャッフル', value:'shuffle'}, {label:'メモを出す（手で処理）', value:'note'}, {label:'やめる', value:null}]});\n  if (!kind) return null;\n  const OPT = {id:'opt', type:'chk', label:'使うか毎回確認する（「〜てもよい」）', value:false};\n  const WHO = {id:'who', type:'sel', label:'だれの', options:[['me','自分'],['opp','相手']], value:'me'};\n  const FILT = [\n    {id:'type', type:'sel', label:'種類', options:[['','すべて'],['クリーチャー','クリーチャー'],['呪文','呪文'],['タマシード','タマシード'],['フィールド','フィールド']], value:''},\n    {id:'race', type:'text', label:'種族にふくむ（例：ドラゴン）', value:''},\n    {id:'civ', type:'sel', label:'文明', options:[['','指定なし'], ...CIVS.map(c=>[c,c])], value:''},\n    {id:'maxCost', type:'num', label:'コスト（以下）', value:''},\n    {id:'name', type:'text', label:'名前にふくむ', value:''}];\n  const mkFilter = v => { const f = {}; if (v.type) f.type=v.type; if (v.race) f.race=v.race; if (v.civ) f.civ=v.civ; if (v.maxCost!=null) f.maxCost=v.maxCost; if (v.name) f.name=v.name; return Object.keys(f).length ? f : undefined; };\n  let out = [], q = '';\n  if (kind==='draw'){\n    const v = await formModal('カードを引く', '', [WHO, {id:'n', type:'num', label:'枚数', value:1}, OPT]); if (!v) return null;\n    out = [{do:'draw', n:v.n||1, who:v.who}]; q = `${v.n||1}枚引きますか？`; if (v.opt) return [{do:'optional', q, steps:out}];\n    return out;\n  }\n  if (kind==='look'){\n    const v = await formModal('山札の上を見る', '「見たカード」として次の手順で使えます。', [WHO, {id:'n', type:'num', label:'枚数', value:3}]); if (!v) return null;\n    return [{do:'look', n:v.n||1, who:v.who, as:'L'}];\n  }\n  if (kind==='choose' || kind==='all'){\n    const fields = [{id:'from', type:'sel', label:'どこから', options:SRC_OPTS, value:'$L'}, ...FILT];\n    if (kind==='choose') fields.push({id:'max', type:'num', label:'最大枚数（99＝好きな数）', value:1}, {id:'min0', type:'chk', label:'選ばなくてもよい', value:true}, {id:'by', type:'sel', label:'選ぶ人', options:[['me','自分'],['opp','相手']], value:'me'});\n    fields.push({id:'act', type:'sel', label:'どうする', options:ACT_OPTS, value:'hand'}, OPT);\n    const v = await formModal(kind==='choose' ? 'カードを選ぶ' : '条件に合うカードすべて', '', fields); if (!v) return null;\n    const as = nextVar(); const f = mkFilter(v);\n    const sel = kind==='choose'\n      ? {do:'choose', q:`${(SRC_OPTS.find(o=>o[0]===v.from)||[])[1]||''}から選ぶ`, from:v.from, ...(f?{filter:f}:{}), min:v.min0?0:Math.min(1, v.max||1), max:v.max||1, as, ...(v.by==='opp'?{by:'opp'}:{})}\n      : {do:'filter', from:v.from, ...(f?{filter:f}:{}), as};\n    const a = actStep(v.act, '$'+as);\n    out = a ? [sel, a] : [sel];\n    if (v.opt) return [{do:'optional', q:`この効果（${stepLine(out[out.length-1])}）を使いますか？`, steps:out}];\n    return out;\n  }\n  if (kind==='rest'){\n    const chosen = [...used].filter(x => x!=='L');\n    const v = await formModal('見たカードの残り', '見たカードのうち、選ばなかったカードを移動します。', [{id:'act', type:'sel', label:'どこへ', options:ACT_OPTS.filter(o=>['hand','manaTapped','mana','grave','shield','deckTop','deckBottom'].includes(o[0])), value:'deckBottom'}]); if (!v) return null;\n    return [actStep(v.act, '$L', chosen.length ? chosen.map(x=>'$'+x) : undefined)];\n  }\n  if (kind==='top'){\n    const v = await formModal('山札の上から', '', [WHO, {id:'n', type:'num', label:'枚数', value:1}, {id:'to', type:'sel', label:'どこへ', options:[['manaT','マナ（タップ）'],['mana','マナ'],['shield','シールド']], value:'manaT'}, OPT]); if (!v) return null;\n    out = [v.to==='shield' ? {do:'topToShield', n:v.n||1, who:v.who} : {do:'topToMana', n:v.n||1, tapped:v.to==='manaT', who:v.who}];\n    if (v.opt) return [{do:'optional', q:`山札の上から${v.n||1}枚を${v.to==='shield'?'シールド化':'マナへ置き'}ますか？`, steps:out}];\n    return out;\n  }\n  if (kind==='shuffle') return [{do:'shuffle', who:'me'}];\n  if (kind==='note'){ const v = await formModal('メモ', '', [{id:'t', type:'area', label:'表示する内容', value:''}]); if (!v || !v.t) return null; return [{do:'note', text:v.t}]; }\n  return null;\n}"};
// stepLine
const _sl = window.stepLine;
window.stepLine = function(st){
    if (st.do==='if') return `もし${condsLine(st.conds)}なら：`;
    if (st.do==='win') return '自分（この能力の持ち主）がゲームに勝つ';
    if (st.do==='lose') return `${st.who==='opp' ? '相手' : '自分'}がゲームに負ける`;
  return _sl(st);
};
// stepsHTML
const _sh = window.stepsHTML;
if (norm(_sh)===norm(OLD.stepsHTML)) window.stepsHTML = function(steps, depth=0){
  if (!steps || !steps.length) return depth ? '' : '<div class="rs">（何もしない）</div>';
  return steps.map((st,i) => `<div class="rs" style="padding-left:${depth*14}px">${depth?'・':(i+1)+'. '}${esc(stepLine(st))}</div>` +
    (st.do==='optional' || st.do==='if' ? stepsHTML(st.steps, depth+1) : '') +
    (st.do==='if' && st.else && st.else.length ? `<div class="rs" style="padding-left:${depth*14}px">そうでなければ：</div>` + stepsHTML(st.else, depth+1) : '') +
    (st.do==='mode' ? (st.options||[]).map(o => `<div class="rs" style="padding-left:${(depth+1)*14}px">［${esc(o.label)}］</div>` + stepsHTML(o.steps, depth+2)).join('') : '')).join('');
};
else window.stepsHTML = function(steps, depth=0){
  if (!(steps||[]).some(s => s && s.do==='if')) return _sh(steps, depth);
  return steps.map(st => st.do!=='if' ? _sh([st], depth) : `<div class="rs" style="padding-left:${depth*14}px">${esc(stepLine(st))}</div>` + window.stepsHTML(st.steps, depth+1) + (st.else && st.else.length ? `<div class="rs" style="padding-left:${depth*14}px">そうでなければ：</div>` + window.stepsHTML(st.else, depth+1) : '')).join('');
};
// runSteps：条件・勝ち負けだけここで処理、ほかは元の処理
const _rs = window.runSteps;
window.runSteps = async function(steps, x, depth){
  if (depth > 6) return;
  for (const st of steps || []){
    if (S.winner) return;
    const who = st.who==='opp' ? x.q : x.p;
    if (st.do!=='if' && st.do!=='win' && st.do!=='lose'){ await _rs([st], x, depth); continue; }
    switch (st.do){
      case 'if': { const ok = condsOk(st.conds, x.p); log(`条件「${condsLine(st.conds)}」：${ok?'満たしている':'満たしていない'}`, x.p); await runSteps(ok ? st.steps : st.else, x, depth+1); break; }
      case 'win': endGame(x.p, `${C(x.src) ? '《'+cname(C(x.src))+'》' : 'カード'}の効果で勝利`); return;
      case 'lose': tryLose(who, `${PN[who]}が${C(x.src) ? '《'+cname(C(x.src))+'》' : 'カード'}の効果で負けた`); if (S.winner) return; break;
    }
    render();
  }
};
const condsWizard = async function(conds){
  conds = conds.slice();
  for (;;){
    const v = await formModal('条件を足す', conds.length ? `いまの条件：${esc(condsLine(conds))}<br>（すべて満たした時）` : '例：自分のマナが10枚以上、相手のシールドが0枚ちょうど、自分の墓地のクリーチャーが5枚以上', [
      {id:'who', type:'sel', label:'だれの', options:[['me','自分'],['opp','相手']], value:'me'},
      {id:'zone', type:'sel', label:'どこ', options:Object.entries(CZONE), value:'mana'},
      {id:'type', type:'sel', label:'数えるもの', options:[['','すべてのカード'],['クリーチャー','クリーチャー'],['呪文','呪文'],['タマシード','タマシード'],['フィールド','フィールド']], value:''},
      {id:'race', type:'text', label:'種族にふくむ（例：ドラゴン、空欄でよい）', value:''},
      {id:'civ', type:'sel', label:'文明', options:[['','指定なし'], ...CIVS.map(c=>[c,c])], value:''},
      {id:'name', type:'text', label:'名前にふくむ（空欄でよい）', value:''},
      {id:'n', type:'num', label:'枚数', value:5},
      {id:'op', type:'sel', label:'以上／以下', options:[['>=','以上'],['<=','以下'],['==','ちょうど']], value:'>='}], '条件を決める');
    if (!v) return conds.length ? conds : null;
    const f = {}; if (v.type) f.type=v.type; if (v.race) f.race=v.race; if (v.civ) f.civ=v.civ; if (v.name) f.name=v.name;
    conds.push({who:v.who, zone:v.zone, ...(Object.keys(f).length?{filter:f}:{}), op:v.op, n:v.n||0});
    const m = await modal({title:'条件', html:`<p>${esc(condsLine(conds))}</p>`, buttons:[{label:'これで決定', value:'ok', primary:true}, {label:'条件をもう1つ足す（両方満たす時）', value:'more'}]});
    if (m!=='more') return conds;
  }
};
const subStepsEditor = async function(title, steps){
  steps = steps.slice();
  for (;;){
    const btns = [{label:'決定', value:'ok', primary:true}, {label:'手順を追加', value:'add'}];
    if (steps.length) btns.push({label:'最後の手順を消す', value:'del'});
    btns.push({label:'やめる', value:null});
    const v = await modal({title, html:`<div class="rsteps">${stepsHTML(steps)}</div><p class="rtx">${steps.length ? '' : '「手順を追加」で、条件を満たした時にすることを決めてください（例：ゲームに勝つ、クリーチャーを出す）。'}</p>`, buttons:btns});
    if (!v) return null;
    if (v==='ok') return steps;
    if (v==='add'){ const a = await addStepWizard(steps); if (a) steps.push(...a); }
    if (v==='del') steps.pop();
  }
};
// addStepWizard
const _aw = window.addStepWizard;
if (norm(_aw)===norm(OLD.addStepWizard)) window.addStepWizard = async function(steps){
  const used = new Set(); (function walk(a){ for (const s of a||[]){ if (s.as) used.add(s.as); walk(s.steps); (s.options||[]).forEach(o=>walk(o.steps)); } })(steps);
  const nextVar = () => { let i=1; while (used.has('X'+i)) i++; return 'X'+i; };
  const kind = await modal({title:'手順を追加', html:'<p>何をしますか？</p>', buttons:[
    {label:'カードを引く', value:'draw'}, {label:'山札の上から何枚か見る（表向きにする）', value:'look'},
    {label:'カードを選んで移動・破壊など', value:'choose'}, {label:'条件に合うカードすべてを移動・破壊など', value:'all'},
    {label:'見たカードの残りを移動', value:'rest'}, {label:'山札の上からマナ／シールドへ', value:'top'},
    {label:'山札をシャッフル', value:'shuffle'},
    {label:'クリーチャーを出す（手札・墓地・マナ・山札の上などから）', value:'putc'},
    {label:'条件をつける（マナ○枚以上なら…など）', value:'if'}, {label:'ゲームに勝つ／負けさせる', value:'win'},
    {label:'メモを出す（手で処理）', value:'note'}, {label:'やめる', value:null}]});
  if (!kind) return null;
  if (kind==='if'){
    const conds = await condsWizard([]); if (!conds) return null;
    const inner = await subStepsEditor(`もし${condsLine(conds)}なら`, []); if (!inner) return null;
    const v = await modal({title:'条件を満たさない時', html:`<p>「${esc(condsLine(conds))}」を満たさない時に、別のことをしますか？</p>`, buttons:[{label:'何もしない', value:'n', primary:true}, {label:'別の手順を作る', value:'y'}]});
    let els = [];
    if (v==='y'){ els = await subStepsEditor('条件を満たさない時', []) || []; }
    return [{do:'if', conds, steps:inner, ...(els.length?{else:els}:{})}];
  }
  if (kind==='win'){
    const v = await modal({title:'ゲームの勝ち負け', buttons:[{label:'自分（この能力の持ち主）が勝つ', value:'win', primary:true}, {label:'相手が負ける', value:'loseOpp'}, {label:'自分が負ける', value:'loseMe'}, {label:'やめる', value:null}]});
    if (!v) return null;
    return [v==='win' ? {do:'win'} : {do:'lose', who: v==='loseOpp' ? 'opp' : 'me'}];
  }
  if (kind==='putc'){
    const v = await formModal('クリーチャーを出す', '', [
      {id:'from', type:'sel', label:'どこから', options:[['$MY_HAND','自分の手札'],['$MY_GRAVE','自分の墓地'],['$MY_MANA','自分のマナ'],['$MY_SHIELD','自分のシールド'],['top','自分の山札の上（見て選ぶ）']], value:'$MY_HAND'},
      {id:'topn', type:'num', label:'山札の上から見る枚数（山札の時）', value:3},
      ...FILT.slice(1), {id:'max', type:'num', label:'最大体数', value:1}, OPT]); if (!v) return null;
    const as = nextVar(); const f = {type:'クリーチャー', ...(mkFilter({...v, type:''})||{})};
    out = [];
    if (v.from==='top') out.push({do:'look', n:v.topn||3, who:'me', as:'L'});
    out.push({do:'choose', q:'バトルゾーンに出すクリーチャーを選ぶ', from: v.from==='top' ? '$L' : v.from, filter:f, min:0, max:v.max||1, as}, {do:'put', ids:'$'+as});
    if (v.from==='top') out.push({do:'move', ids:'$L', except:['$'+as], to:'deckBottom'});
    if (v.opt) return [{do:'optional', q:'クリーチャーを出しますか？', steps:out}];
    return out;
  }
  const OPT = {id:'opt', type:'chk', label:'使うか毎回確認する（「〜てもよい」）', value:false};
  const WHO = {id:'who', type:'sel', label:'だれの', options:[['me','自分'],['opp','相手']], value:'me'};
  const FILT = [
    {id:'type', type:'sel', label:'種類', options:[['','すべて'],['クリーチャー','クリーチャー'],['呪文','呪文'],['タマシード','タマシード'],['フィールド','フィールド']], value:''},
    {id:'race', type:'text', label:'種族にふくむ（例：ドラゴン）', value:''},
    {id:'civ', type:'sel', label:'文明', options:[['','指定なし'], ...CIVS.map(c=>[c,c])], value:''},
    {id:'maxCost', type:'num', label:'コスト（以下）', value:''},
    {id:'name', type:'text', label:'名前にふくむ', value:''}];
  const mkFilter = v => { const f = {}; if (v.type) f.type=v.type; if (v.race) f.race=v.race; if (v.civ) f.civ=v.civ; if (v.maxCost!=null) f.maxCost=v.maxCost; if (v.name) f.name=v.name; return Object.keys(f).length ? f : undefined; };
  let out = [], q = '';
  if (kind==='draw'){
    const v = await formModal('カードを引く', '', [WHO, {id:'n', type:'num', label:'枚数', value:1}, OPT]); if (!v) return null;
    out = [{do:'draw', n:v.n||1, who:v.who}]; q = `${v.n||1}枚引きますか？`; if (v.opt) return [{do:'optional', q, steps:out}];
    return out;
  }
  if (kind==='look'){
    const v = await formModal('山札の上を見る', '「見たカード」として次の手順で使えます。', [WHO, {id:'n', type:'num', label:'枚数', value:3}]); if (!v) return null;
    return [{do:'look', n:v.n||1, who:v.who, as:'L'}];
  }
  if (kind==='choose' || kind==='all'){
    const fields = [{id:'from', type:'sel', label:'どこから', options:SRC_OPTS, value:'$L'}, ...FILT];
    if (kind==='choose') fields.push({id:'max', type:'num', label:'最大枚数（99＝好きな数）', value:1}, {id:'min0', type:'chk', label:'選ばなくてもよい', value:true}, {id:'by', type:'sel', label:'選ぶ人', options:[['me','自分'],['opp','相手']], value:'me'});
    fields.push({id:'act', type:'sel', label:'どうする', options:ACT_OPTS, value:'hand'}, OPT);
    const v = await formModal(kind==='choose' ? 'カードを選ぶ' : '条件に合うカードすべて', '', fields); if (!v) return null;
    const as = nextVar(); const f = mkFilter(v);
    const sel = kind==='choose'
      ? {do:'choose', q:`${(SRC_OPTS.find(o=>o[0]===v.from)||[])[1]||''}から選ぶ`, from:v.from, ...(f?{filter:f}:{}), min:v.min0?0:Math.min(1, v.max||1), max:v.max||1, as, ...(v.by==='opp'?{by:'opp'}:{})}
      : {do:'filter', from:v.from, ...(f?{filter:f}:{}), as};
    const a = actStep(v.act, '$'+as);
    out = a ? [sel, a] : [sel];
    if (v.opt) return [{do:'optional', q:`この効果（${stepLine(out[out.length-1])}）を使いますか？`, steps:out}];
    return out;
  }
  if (kind==='rest'){
    const chosen = [...used].filter(x => x!=='L');
    const v = await formModal('見たカードの残り', '見たカードのうち、選ばなかったカードを移動します。', [{id:'act', type:'sel', label:'どこへ', options:ACT_OPTS.filter(o=>['hand','manaTapped','mana','grave','shield','deckTop','deckBottom'].includes(o[0])), value:'deckBottom'}]); if (!v) return null;
    return [actStep(v.act, '$L', chosen.length ? chosen.map(x=>'$'+x) : undefined)];
  }
  if (kind==='top'){
    const v = await formModal('山札の上から', '', [WHO, {id:'n', type:'num', label:'枚数', value:1}, {id:'to', type:'sel', label:'どこへ', options:[['manaT','マナ（タップ）'],['mana','マナ'],['shield','シールド']], value:'manaT'}, OPT]); if (!v) return null;
    out = [v.to==='shield' ? {do:'topToShield', n:v.n||1, who:v.who} : {do:'topToMana', n:v.n||1, tapped:v.to==='manaT', who:v.who}];
    if (v.opt) return [{do:'optional', q:`山札の上から${v.n||1}枚を${v.to==='shield'?'シールド化':'マナへ置き'}ますか？`, steps:out}];
    return out;
  }
  if (kind==='shuffle') return [{do:'shuffle', who:'me'}];
  if (kind==='note'){ const v = await formModal('メモ', '', [{id:'t', type:'area', label:'表示する内容', value:''}]); if (!v || !v.t) return null; return [{do:'note', text:v.t}]; }
  return null;
};
else window.addStepWizard = async function(steps){
  const k = await modal({title:'手順を追加', html:'<p>何をしますか？</p>', buttons:[{label:'条件をつける（マナ○枚以上なら…など）', value:'if'}, {label:'ゲームに勝つ／負けさせる', value:'win'}, {label:'そのほかの手順', value:'old', primary:true}, {label:'やめる', value:null}]});
  if (!k) return null;
  if (k==='old') return _aw(steps);
  if (k==='if'){
    const conds = await condsWizard([]); if (!conds) return null;
    const inner = await subStepsEditor(`もし${condsLine(conds)}なら`, []); if (!inner) return null;
    return [{do:'if', conds, steps:inner}];
  }
  const v = await modal({title:'ゲームの勝ち負け', buttons:[{label:'自分（この能力の持ち主）が勝つ', value:'win', primary:true}, {label:'相手が負ける', value:'loseOpp'}, {label:'自分が負ける', value:'loseMe'}, {label:'やめる', value:null}]});
  if (!v) return null;
  return [v==='win' ? {do:'win'} : {do:'lose', who: v==='loseOpp' ? 'opp' : 'me'}];
};
// stepsEditor：「条件を満たした時」だけ新しい編集画面
const _se = window.stepsEditor;
const seNew = async function(s, ev, steps){
  steps = JSON.parse(JSON.stringify(steps || []));
  let conds = JSON.parse(JSON.stringify(ruleOf(s.name, ev)?.conds || []));
  if (ev==='cond' && !conds.length){ conds = await condsWizard([]) || []; if (!conds.length) return false; }
  for (;;){
    const btns = [{label:'保存', value:'save', primary:true}, {label:'手順を追加', value:'add'}];
    if (ev==='cond') btns.push({label:'条件を直す', value:'conds'});
    if (steps.length) btns.push({label:'手順を消す', value:'del'});
    if (sampleFn) btns.push({label:'説明を書いて作り直す', value:'desc'});
    btns.push({label:'JSONで直す（上級）', value:'json'}, {label:'やめる', value:null});
    const condHtml = ev==='cond' ? `<p><b>いつ：</b>このカードがバトルゾーンにある間に「${esc(condsLine(conds))}」を満たした時（満たしていない状態から満たした時に1回）</p>` : '';
    const v = await modal({title:`《${s.name}》${EVLABEL[ev]}の手順`, html:`<p class="rtx">${esc(s.text)}</p>${condHtml}<div class="rsteps">${stepsHTML(steps)}</div><p class="rtx">上から順に処理します。「保存」すると次からこの手順で処理します。</p>`, buttons:btns});
    if (!v) return false;
    if (v==='conds'){ const c2 = await condsWizard([]); if (c2 && c2.length) conds = c2; continue; }
    if (v==='save'){ setRule(s.name, ev, {mode:'steps', steps, ...(ev==='cond'?{conds}:{})}); log(`《${s.name}》の${EVLABEL[ev]}の処理を変更しました`); return true; }
    if (v==='add'){ const add = await addStepWizard(steps); if (add) steps.push(...add); }
    if (v==='del'){ const i = await modal({title:'どの手順を消しますか？', buttons:[...steps.map((st,i)=>({label:`${i+1}. ${stepLine(st).slice(0,40)}`, value:i})), {label:'やめる', value:-1}]}); if (i>=0) steps.splice(i,1); }
    if (v==='desc'){
      const f = await formModal('説明から手順を作る', 'このカードを実際にどう処理するか、ふつうの言葉で書いてください。うまくいかなかった点も書くと直します。', [{id:'d', type:'area', label:'正しい処理', value:ruleOf(s.name, ev)?.memo || s.text, rows:6}], '作る');
      if (!f || !f.d) continue;
      const got = await stepsFromDescription(s, ev, f.d, steps.length ? steps : null);
      if (got){ steps = got; (RULES[s.name]?.[ev]||{}).memo = f.d; }
      else await modal({title:'作れませんでした', html:'<p>手順を作れませんでした。「手順を追加」から1つずつ組み立てることもできます。</p>', buttons:[{label:'閉じる', value:1}]});
    }
    if (v==='json'){
      const f = await formModal('JSONで直す', '', [{id:'j', type:'area', label:'steps', value:JSON.stringify(steps, null, 1), rows:12}]);
      if (!f) continue;
      try{ const a = JSON.parse(f.j); steps = Array.isArray(a) ? a : (a.steps||[]); }catch(e){ await modal({title:'読めません', html:`<p>${esc(e.message)}</p>`, buttons:[{label:'閉じる', value:1}]}); }
    }
  }
};
window.stepsEditor = function(s, ev, steps){ return ev==='cond' ? seNew(s, ev, steps) : _se(s, ev, steps); };
// drain の後に「条件を満たした時」を確認
const condTriggers = function(){
  const hit = []; const on = (S.condOn ||= {});
  for (const p of ['me','op']) for (const uid of zoneArr(p,'battle')){
    const c = C(uid); if (!c || c.cid===HIDDEN_CID) continue;
    const s = side(c); const r = ruleOf(s.name, 'cond');
    if (!r || !r.conds || !r.conds.length) continue;
    const ok = condsOk(r.conds, p);
    if (ok && !on[uid]) hit.push(uid);
    on[uid] = ok;
  }
  for (const k of Object.keys(on)){ const c = C(k); if (!c || c.zone!=='battle') delete on[k]; }
  return hit;
};
const drainCore = window.drain;
window.drain = async function(){
  await drainCore();
  for (let n=0; n<10 && !S.winner && !(typeof NOFX!=='undefined' && NOFX); n++){
    const hit = condTriggers(); if (!hit.length) break;
    for (const uid of hit){ if (S.winner) break; await resolveEffect(uid, 'cond'); await drainCore(); }
  }
};
})();
