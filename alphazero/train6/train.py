"""自己対戦データでネットを学習し、Rust が読む DMN1 形式で書き出す。
python train.py --data gen0.bin gen1.bin ... --init prev.pt --out net5 [--epochs 2]
→ net5.pt（続きの学習用）と net5.bin（selfplay/arena 用）
"""
import argparse, struct, time
import numpy as np
import torch, torch.nn as nn, torch.nn.functional as F

S, O, H, P = 1328, 296, 256, 128  # 2026-11-27 環境デッキ入り15デッキ（120種類）。features.rs の S・O と同じ


def read(path):
    b = open(path, 'rb').read()
    assert b[:4] == b'DMS1', path
    pos = 4
    states, zs, nopt, opt_rows, pis = [], [], [], [], []
    st_idx, st_val, op_idx, op_val, op_row = [], [], [], [], []
    n = 0
    m = 0
    L = len(b)
    while pos < L:
        k, z = struct.unpack_from('<Hf', b, pos); pos += 6
        (nz,) = struct.unpack_from('<H', b, pos); pos += 2
        a = np.frombuffer(b, dtype=np.dtype([('i', '<u2'), ('v', '<f4')]), count=nz, offset=pos); pos += 6 * nz
        st_idx.append(a['i'].astype(np.int64) + n * S); st_val.append(a['v'])
        for _ in range(k):
            pi, nz = struct.unpack_from('<fH', b, pos); pos += 6
            a = np.frombuffer(b, dtype=np.dtype([('i', '<u2'), ('v', '<f4')]), count=nz, offset=pos); pos += 6 * nz
            op_idx.append(a['i'].astype(np.int64) + m * O); op_val.append(a['v'])
            pis.append(pi); op_row.append(n)
            m += 1
        zs.append(z); nopt.append(k)
        n += 1
    st = np.zeros(n * S, np.float32)
    if st_idx: st[np.concatenate(st_idx)] = np.concatenate(st_val)
    op = np.zeros(m * O, np.float32)
    if op_idx: op[np.concatenate(op_idx)] = np.concatenate(op_val)
    return st.reshape(n, S), np.array(zs, np.float32), op.reshape(m, O), np.array(pis, np.float32), np.array(nopt, np.int64)


class Net(nn.Module):
    def __init__(self):
        super().__init__()
        self.l1 = nn.Linear(S, H); self.l2 = nn.Linear(H, H); self.v = nn.Linear(H, 1)
        self.p1 = nn.Linear(H + O, P); self.p2 = nn.Linear(P, 1)

    def forward(self, st, op, seg):
        h = F.relu(self.l2(F.relu(self.l1(st))))
        v = torch.tanh(self.v(h)).squeeze(1)
        z = F.relu(self.p1(torch.cat([h[seg], op], 1)))
        return v, self.p2(z).squeeze(1)


def seg_log_softmax(lg, seg, nb):
    mx = torch.full((nb,), -1e9, device=lg.device).scatter_reduce(0, seg, lg, 'amax')
    e = torch.exp(lg - mx[seg])
    s = torch.zeros(nb, device=lg.device).index_add(0, seg, e)
    return lg - mx[seg] - torch.log(s[seg])


def export(net, path):
    with open(path, 'wb') as f:
        f.write(struct.pack('<5I', 0x444d4e31, S, O, H, P))
        for l in [net.l1, net.l2, net.v, net.p1, net.p2]:
            f.write(l.weight.detach().cpu().numpy().astype('<f4').tobytes())
            f.write(l.bias.detach().cpu().numpy().astype('<f4').tobytes())


def load_bin(net, path):
    """DMN1 形式の重み（transplant で広げたものなど）から読み込む"""
    b = open(path, 'rb').read()
    hdr = struct.unpack_from('<5I', b, 0)
    assert hdr == (0x444d4e31, S, O, H, P), f'{path}: 大きさが違う {hdr}'
    a = np.frombuffer(b, dtype='<f4', offset=20)
    pos = 0
    with torch.no_grad():
        for l in [net.l1, net.l2, net.v, net.p1, net.p2]:
            n = l.weight.numel()
            l.weight.copy_(torch.from_numpy(a[pos:pos + n].copy()).view_as(l.weight)); pos += n
            n = l.bias.numel()
            l.bias.copy_(torch.from_numpy(a[pos:pos + n].copy())); pos += n
    assert pos == len(a)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--data', nargs='+', required=True)
    ap.add_argument('--init')
    ap.add_argument('--init-bin', help='最初の重みを .bin から（.pt が無い時）')
    ap.add_argument('--out', required=True)
    ap.add_argument('--epochs', type=float, default=2)
    ap.add_argument('--batch', type=int, default=1024)
    ap.add_argument('--lr', type=float, default=1e-3)
    a = ap.parse_args()
    dev = 'cuda' if torch.cuda.is_available() else 'cpu'
    t = time.time()
    parts = [read(p) for p in a.data]
    st = np.concatenate([x[0] for x in parts]); z = np.concatenate([x[1] for x in parts])
    op = np.concatenate([x[2] for x in parts]); pi = np.concatenate([x[3] for x in parts])
    nopt = np.concatenate([x[4] for x in parts])
    off = np.concatenate([[0], np.cumsum(nopt)])
    N = len(z)
    print(f'samples {N} options {len(pi)} read {time.time()-t:.1f}s device {dev}', flush=True)
    net = Net().to(dev)
    if a.init:
        net.load_state_dict(torch.load(a.init, map_location=dev))
    elif a.init_bin:
        load_bin(net, a.init_bin)
    opt = torch.optim.AdamW(net.parameters(), lr=a.lr, weight_decay=1e-4)
    stT = torch.from_numpy(st).to(dev); zT = torch.from_numpy(z).to(dev)
    opT = torch.from_numpy(op).to(dev); piT = torch.from_numpy(pi).to(dev)
    offT = torch.from_numpy(off).to(dev); noptT = torch.from_numpy(nopt).to(dev)
    steps = max(1, int(a.epochs * N / a.batch))
    sched = torch.optim.lr_scheduler.OneCycleLR(opt, max_lr=a.lr, total_steps=steps)
    lv = lp = 0.0
    for it in range(steps):
        idx = torch.randint(0, N, (min(a.batch, N),), device=dev)
        k = noptT[idx]
        seg = torch.repeat_interleave(torch.arange(len(idx), device=dev), k)
        start = torch.repeat_interleave(offT[idx], k)
        within = torch.arange(len(seg), device=dev) - torch.repeat_interleave(torch.cumsum(k, 0) - k, k)
        orow = start + within
        v, lg = net(stT[idx], opT[orow], seg)
        loss_v = F.mse_loss(v, zT[idx])
        loss_p = -(piT[orow] * seg_log_softmax(lg, seg, len(idx))).sum() / len(idx)
        loss = loss_v + loss_p
        opt.zero_grad(); loss.backward(); opt.step(); sched.step()
        lv = 0.98 * lv + 0.02 * loss_v.item() if it else loss_v.item()
        lp = 0.98 * lp + 0.02 * loss_p.item() if it else loss_p.item()
        if it % 500 == 0 or it == steps - 1:
            print(f'  step {it+1}/{steps} value {lv:.4f} policy {lp:.4f}', flush=True)
    torch.save(net.state_dict(), a.out + '.pt')
    export(net, a.out + '.bin')
    print(f'done value_loss={lv:.4f} policy_loss={lp:.4f} secs={time.time()-t:.1f}')


if __name__ == '__main__':
    main()
