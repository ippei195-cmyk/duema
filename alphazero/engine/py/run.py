"""自己対戦学習のループ（止めるまで回り続ける。途中で止めても続きから再開できる）。
  python run.py            … 開始・再開
  STOP というファイルを置く … 今の世代が終わったら止まる
世代ごと：①自己対戦（いま一番強いネット）→ ②学習 → ③一番強いネットと対戦して 55% 以上なら交代
         ④3世代ごとにフェーズ2版（ismcts:300）と対戦し、80% 以上で合格
記録：log.csv（世代ごと）、elo.csv（各ネットのレート）
"""
import csv, json, math, os, re, shutil, subprocess, sys, time

D = os.path.dirname(os.path.abspath(__file__))
EXE = '.exe' if os.name == 'nt' else ''
SELFPLAY = os.path.join(D, 'selfplay' + EXE)
ARENA = os.path.join(D, 'arena' + EXE)
TH = str(max(2, (os.cpu_count() or 4) - 2))
CFG = dict(games=2000, sims_full=200, sims_fast=40, p_full=0.25, window=6, epochs=2,
           eval_games=180, eval_sims=200, promote=0.55, phase2_every=3, phase2_games=180)
for d in ['samples', 'nets']:
    os.makedirs(os.path.join(D, d), exist_ok=True)
SP = os.path.join(D, 'state.json')


def load_state():
    if os.path.exists(SP):
        return json.load(open(SP, encoding='utf-8'))
    return {'gen': 0, 'best': 'none', 'best_elo': 0.0, 'latest_pt': None, 'passed': False}


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


def main():
    s = load_state()
    while not os.path.exists(os.path.join(D, 'STOP')):
        g = s['gen']
        t0 = time.time()
        smp = os.path.join(D, 'samples', f'gen{g:04d}.bin')
        out = run([SELFPLAY, s['best'], str(CFG['games']), TH, str(1000 + g), smp,
                   str(CFG['sims_full']), str(CFG['sims_fast']), str(CFG['p_full'])], f'selfplay gen{g}')
        nsamp = int(re.search(r'samples=(\d+)', out).group(1))
        t1 = time.time()
        data = [os.path.join(D, 'samples', f'gen{k:04d}.bin') for k in range(max(0, g - CFG['window'] + 1), g + 1)]
        newp = os.path.join(D, 'nets', f'net{g + 1:04d}')
        cmd = [sys.executable, os.path.join(D, 'train.py'), '--data', *data, '--out', newp, '--epochs', str(CFG['epochs'])]
        if s['latest_pt']:
            cmd += ['--init', s['latest_pt']]
        tout = run(cmd, f'train gen{g}')
        lv = re.search(r'value_loss=([\d.]+) policy_loss=([\d.]+)', tout)
        s['latest_pt'] = newp + '.pt'
        t2 = time.time()
        wr = winrate(run([ARENA, az(newp + '.bin'), az(s['best']), str(CFG['eval_games']), TH, str(g)], f'eval gen{g}'))
        promoted = wr >= CFG['promote']
        elo = s['best_elo'] + elo_diff(wr)
        append('elo.csv', [g + 1, f'net{g + 1:04d}', round(elo, 1), round(wr, 3), promoted], ['gen', 'net', 'elo', 'vs_best', 'promoted'])
        if promoted:
            s['best'] = newp + '.bin'
            s['best_elo'] = elo
        p2 = ''
        if (g + 1) % CFG['phase2_every'] == 0 and s['best'] != 'none':
            p2 = winrate(run([ARENA, az(s['best']), 'ismcts:300', str(CFG['phase2_games']), TH, str(g + 77)], f'phase2 gen{g}'))
            if p2 >= 0.80 and not s['passed']:
                s['passed'] = True
                with open(os.path.join(D, 'PASSED.txt'), 'w', encoding='utf-8') as f:
                    f.write(f'{time.ctime()} gen{g + 1} {s["best"]} vs ismcts:300 = {p2:.1%}\n')
        s['gen'] = g + 1
        save_state(s)
        append('log.csv', [g, nsamp, lv.group(1), lv.group(2), round(wr, 3), promoted, os.path.basename(s['best']),
                           p2 if p2 == '' else round(p2, 3), round(t1 - t0), round(t2 - t1), round(time.time() - t2), time.strftime('%Y-%m-%d %H:%M')],
               ['gen', 'samples', 'value_loss', 'policy_loss', 'vs_best', 'promoted', 'best', 'vs_phase2', 'selfplay_s', 'train_s', 'eval_s', 'time'])
        print(f'gen{g}: vs best {wr:.1%} promoted={promoted} phase2={p2}', flush=True)
        # 古い学習データは window より前を消さずに残す（容量が足りなくなったら手で消せる）
    print('STOP を見つけたので止まりました')


if __name__ == '__main__':
    main()
