"""自己対戦学習のループ（止めるまで回り続ける。途中で止めても続きから再開できる）。
  python run.py            … 開始・再開
  STOP というファイルを置く … 今の世代が終わったら止まる
世代ごと：①自己対戦（いま一番強いネット）→ ②学習 → ③一番強いネットと対戦して 55% 以上なら交代
         ④3世代ごとにフェーズ2版（ismcts:300）と先読み1200回で対戦し、80% 以上（540局で確認）で合格。自己対戦は15デッキ（学習用5＋環境デッキ10）。3割はリースバイク・デモニオ・創世竜・ギリガザミ・王道刈のどれかを入れる（focus）。
         15%はギリガザミ・創世竜の決めパターン直前の局面から始め、note の定石を方策の後押しに使う（scenario・bias）。
         2026-11-27 一平の判断で創世竜・ゴルギー・ゴルギー（環境版）は自己学習から外す（skip_decks）。
         交代判定は残り12デッキ、合格判定（ismcts:300 に80%）は学習用のうちゴルギーを除く4デッキ（リースバイク・ハッスル・ジョーカーズ・白緑ウィリデ）
記録：log.csv（世代ごと）、elo.csv（各ネットのレート）
古い学習データ（直近10世代より前）と、交代しなかったネットは自動で消す
"""
import csv, json, math, os, re, shutil, subprocess, sys, time

D = os.path.dirname(os.path.abspath(__file__))
EXE = '.exe' if os.name == 'nt' else ''
SELFPLAY = os.path.join(D, 'selfplay' + EXE)
ARENA = os.path.join(D, 'arena' + EXE)
TH = str(max(2, (os.cpu_count() or 4) - 2))
CFG = dict(games=2000, sims_full=200, sims_fast=40, p_full=0.25, window=6, epochs=2,
           eval_games=180, eval_sims=200, promote=0.55, phase2_every=3, phase2_games=180, phase2_sims=1200, focus=0.3, focus_decks='0,11,13,14', scenario=0.15, bias=1.0, teach=0, skip_decks='4,9,12',
           eval_decks='0,1,2,3,5,6,7,8,10,11,13,14', phase2_decks='0,1,2,3',
           keep_samples=10)  # 学習データは直近この世代数だけ残す（学習に使うのは window 世代）
for d in ['samples', 'nets']:
    os.makedirs(os.path.join(D, d), exist_ok=True)
SP = os.path.join(D, 'state.json')


def load_state():
    if os.path.exists(SP):
        return json.load(open(SP, encoding='utf-8'))
    # 最初の重み（train5 の第14世代を transplant で15デッキ用に広げたもの）があれば、そこから始める
    seed = os.path.join(D, 'nets', 'net0000.bin')
    best = seed if os.path.exists(seed) else 'none'
    return {'gen': 0, 'best': best, 'best_elo': 0.0, 'latest_pt': None, 'passed': False}


def save_state(s):
    json.dump(s, open(SP + '.tmp', 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
    os.replace(SP + '.tmp', SP)


def run(args, log):
    print('>', ' '.join(args), flush=True)
    p = subprocess.run(args, capture_output=True, text=True, encoding='utf-8', errors='replace')
    with open(os.path.join(D, 'run_output.log'), 'a', encoding='utf-8') as f:
        f.write(f'\n## {time.ctime()} {log}\n' + p.stdout + p.stderr[-4000:])
    if p.returncode != 0:
        print(p.stdout, p.stderr[-3000:])
        raise SystemExit(f'失敗: {log}')
    return p.stdout


def winrate(out):
    return float(re.search(r'A win rate ([\d.]+)%', out).group(1)) / 100


def elo_diff(p):
    p = min(max(p, 0.01), 0.99)
    return 400 * math.log10(p / (1 - p))


def az(path):
    return f"az:{CFG['eval_sims']}:{path}"


def append(name, row, head):
    fp = os.path.join(D, name)
    new = not os.path.exists(fp)
    with open(fp, 'a', newline='', encoding='utf-8') as f:
        w = csv.writer(f)
        if new:
            w.writerow(head)
        w.writerow(row)


def cleanup(s):
    """古い学習データと、交代しなかったネットを消す。
    残すもの：直近 keep_samples 世代の学習データ、交代したネット（歴代の一番強い版）、いま一番強い版、続きの学習に使う最新の .pt"""
    g = s['gen']
    for f in os.listdir(os.path.join(D, 'samples')):
        m = re.match(r'gen(\d+)\.bin$', f)
        if m and int(m.group(1)) < g - CFG['keep_samples']:
            os.remove(os.path.join(D, 'samples', f))
    keep = {os.path.basename(s['best']).split('.')[0], os.path.basename(s['latest_pt'] or '').split('.')[0]}
    ep = os.path.join(D, 'elo.csv')
    if os.path.exists(ep):
        for r in csv.DictReader(open(ep, encoding='utf-8')):
            if r.get('promoted') == 'True':
                keep.add(r['net'])
    for f in os.listdir(os.path.join(D, 'nets')):
        m = re.match(r'(net\d+)\.(bin|pt)$', f)
        if m and m.group(1) not in keep and int(m.group(1)[3:]) < g:
            os.remove(os.path.join(D, 'nets', f))


def main():
    s = load_state()
    while not os.path.exists(os.path.join(D, 'STOP')):
        g = s['gen']
        t0 = time.time()
        smp = os.path.join(D, 'samples', f'gen{g:04d}.bin')
        out = run([SELFPLAY, s['best'], str(CFG['games']), TH, str(1000 + g), smp,
                   str(CFG['sims_full']), str(CFG['sims_fast']), str(CFG['p_full']), str(CFG['focus']), CFG['focus_decks'], str(CFG['scenario']), str(CFG['bias']), str(CFG['teach']), CFG['skip_decks']], f'selfplay gen{g}')
        nsamp = int(re.search(r'samples=(\d+)', out).group(1))
        t1 = time.time()
        data = [os.path.join(D, 'samples', f'gen{k:04d}.bin') for k in range(max(0, g - CFG['window'] + 1), g + 1)]
        newp = os.path.join(D, 'nets', f'net{g + 1:04d}')
        cmd = [sys.executable, os.path.join(D, 'train.py'), '--data', *data, '--out', newp, '--epochs', str(CFG['epochs'])]
        if s['latest_pt']:
            cmd += ['--init', s['latest_pt']]
        elif s['best'] != 'none':
            cmd += ['--init-bin', s['best']]  # train5 の重みを広げたもの（transplant）から始める
        tout = run(cmd, f'train gen{g}')
        lv = re.search(r'value_loss=([\d.]+) policy_loss=([\d.]+)', tout)
        s['latest_pt'] = newp + '.pt'
        t2 = time.time()
        wr = winrate(run([ARENA, az(newp + '.bin'), az(s['best']), str(CFG['eval_games']), TH, str(g), str(CFG['eval_decks'])], f'eval gen{g}'))
        promoted = wr >= CFG['promote']
        elo = s['best_elo'] + elo_diff(wr)
        append('elo.csv', [g + 1, f'net{g + 1:04d}', round(elo, 1), round(wr, 3), promoted], ['gen', 'net', 'elo', 'vs_best', 'promoted'])
        if promoted:
            s['best'] = newp + '.bin'
            s['best_elo'] = elo
        p2 = ''
        if (g + 1) % CFG['phase2_every'] == 0 and s['best'] != 'none':
            p2 = winrate(run([ARENA, f"az:{CFG['phase2_sims']}:{s['best']}", 'ismcts:300', str(CFG['phase2_games']), TH, str(g + 77), CFG['phase2_decks']], f'phase2 gen{g}'))
            if p2 >= 0.80 and not s['passed']:
                # 180局だとばらつきが大きいので、3倍の局数で測り直して 80% 以上の時だけ合格にする
                p2c = winrate(run([ARENA, f"az:{CFG['phase2_sims']}:{s['best']}", 'ismcts:300', str(CFG['phase2_games'] * 3), TH, str(g + 7777), CFG['phase2_decks']], f'phase2 confirm gen{g}'))
                append('phase2_confirm.csv', [g + 1, os.path.basename(s['best']), round(p2, 3), round(p2c, 3), time.strftime('%Y-%m-%d %H:%M')],
                       ['gen', 'net', 'vs_phase2_180', 'vs_phase2_540', 'time'])
                p2 = p2c
            if p2 >= 0.80 and not s['passed']:
                s['passed'] = True
                with open(os.path.join(D, 'PASSED.txt'), 'w', encoding='utf-8') as f:
                    f.write(f'{time.ctime()} gen{g + 1} {s["best"]} vs ismcts:300 = {p2:.1%}（{CFG["phase2_games"] * 3}局で確認）\n')
        s['gen'] = g + 1
        save_state(s)
        append('log.csv', [g, nsamp, lv.group(1), lv.group(2), round(wr, 3), promoted, os.path.basename(s['best']),
                           p2 if p2 == '' else round(p2, 3), round(t1 - t0), round(t2 - t1), round(time.time() - t2), time.strftime('%Y-%m-%d %H:%M')],
               ['gen', 'samples', 'value_loss', 'policy_loss', 'vs_best', 'promoted', 'best', 'vs_phase2', 'selfplay_s', 'train_s', 'eval_s', 'time'])
        try:
            cleanup(s)
        except Exception as e:
            print('cleanup failed:', e, flush=True)
        print(f'gen{g}: vs best {wr:.1%} promoted={promoted} phase2={p2}', flush=True)
    print('STOP を見つけたので止まりました')


if __name__ == '__main__':
    main()
