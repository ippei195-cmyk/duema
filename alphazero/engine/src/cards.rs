//! 学習用5デッキ（2026-11-24・11-26 一平のデッキシート）に入っている58種類のカード定義。
//! 効果文は /mnt/project-files/alphazero/card-texts.md、裁定は 裁定メモ.md。

pub const L: u8 = 1; // 光
pub const W: u8 = 2; // 水
pub const D: u8 = 4; // 闇
pub const F: u8 = 8; // 火
pub const N: u8 = 16; // 自然
pub const ZERO: u8 = 0; // ゼロ文明

// 種族（必要なものだけ）
pub const R_DRAGON: u16 = 1;
pub const R_JOKERS: u16 = 2;
pub const R_SEITOKAI: u16 = 4; // 生徒会組
pub const R_ABYSS: u16 = 8;
pub const R_COMMAND: u16 = 16;
pub const R_GIANT: u16 = 32;
pub const R_MAGIC: u16 = 64;
pub const R_ORACLE: u16 = 128;
pub const R_FUSION: u16 = 256; // フュージョナー
pub const R_REX: u16 = 512; // レクスターズ（鬼レクスターズを含む）
pub const R_DEMONIO: u16 = 1024; // デモニオ
pub const R_PANDORA: u16 = 2048;
pub const R_PUPPET: u16 = 4096; // デスパペット

// キーワード
pub const K_BLOCKER: u32 = 1;
pub const K_SA: u32 = 2; // スピードアタッカー
pub const K_MACH: u32 = 4; // マッハファイター
pub const K_STRIG: u32 = 8; // S・トリガー
pub const K_GSTRIKE: u32 = 16;
pub const K_JUSTDIVER: u32 = 32;
pub const K_EXLIFE: u32 = 64;
pub const K_WB: u32 = 128; // W・ブレイカー
pub const K_TB: u32 = 256; // T・ブレイカー
pub const K_WORLD: u32 = 512; // ワールド・ブレイカー
pub const K_SST: u32 = 1024; // スーパー・S・トリガー（K_STRIG と一緒に付ける）

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Creature,
    Spell,
    GCastle,
    Tama, // タマシード
}

#[derive(Clone, Copy, Debug)]
pub struct Side {
    pub name: &'static str,
    pub kind: Kind,
    pub civ: u8,
    pub cost: u8,
    pub power: i32,
    pub race: u16,
    pub kw: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Evo {
    None,
    Evo,   // 進化（下が必要）
    Neo,   // NEO進化（下は任意）
    GNeo,  // G-NEO進化（下は任意、離れる時に下が身代わり）
}

#[derive(Clone, Copy, Debug)]
pub struct CardDef {
    pub id: &'static str,
    pub main: Side,
    pub spell: Option<Side>, // ツインパクトの呪文側
    pub hyper: Option<(i32, u32)>, // ハイパーモードのパワーと追加キーワード
    pub evo: Evo,
}

macro_rules! side {
    ($n:expr, $k:ident, $c:expr, $cost:expr, $p:expr, $r:expr, $kw:expr) => {
        Side { name: $n, kind: Kind::$k, civ: $c, cost: $cost, power: $p, race: $r, kw: $kw }
    };
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum C {
    // リースバイク
    Kiyuri, Humpty, Caddy, Magnum, Chakra, Dagura, RedZone, PPP, Fujin, Antiham, Cotton,
    // ハッスル・キャッスル
    Hustle, Gyakuten, Javert, Gyai, Gaiash, DogiGyaku, Ash, Suzu, Van, DarkMasters, Kernel, Sozanmaru, Laffrull, Kusabira, Shacogail, Madan,
    // ジョーカーズ
    JoJoJo, Magny, Fujisan, Slotton, Brand, Jogimera, Melaventure, Jogiragon, Moujakinga, Niyani, Kesshin, Sakki, Manhattan,
    // 白緑ウィリデ（2026-11-26 追加）
    SaintCastle, Zaze, Ichion, Everlast, GrandCross, Willide, Emotional,
    // ゴルギーオージャー（2026-11-26 追加）
    Gorgi, Canzone, Akane, Heracles, Takasugi, Jasmine, Kohaku, Gindan, Aofesy, Cineraria, Vt5000,
    // DMラボ環境デッキ 第1段階（2026-11-26 追加）
    Odotta, Jolagon, Stella, Deaddam, Seikimatsu, Sekkei, Konpeki, Seihou, Paradiso, SixFall,
    // 第2段階：デアリデモニオ
    Helcop, Canbero, Greatest, Momo, Cry, Bakuro, DeathLost, Dmc, Ichiou, Sangan, Shurai, Chitsujo,
    // 第2段階：創世竜
    Drache, Esmel, Chiryu, Trap, Pearly, GreatBlue, Eringi, Atlantis, Scorpion, Levya, Valhalla, ManhattanT, Wokanna, Emeral, Alspa,
    // 第3段階：4Cギリガザミ
    EBrain, Kurin, Elborom, Azafaust, Risou, Grimoire, Shinkirou, Kakugo, Petens, Giri, LoveAbyss, Dolma, Dokin, Kamigami, Akashic, Efc,
    // 第3段階：4Cドギラゴン王道刈
    Tensai, GNext, Katsu, DogiRev, Nibai, Oudou, Dogidante, Felzark, Kouten,
}
pub const NCARDS: usize = 120;

pub fn all() -> [C; NCARDS] {
    use C::*;
    [Kiyuri, Humpty, Caddy, Magnum, Chakra, Dagura, RedZone, PPP, Fujin, Antiham, Cotton,
     Hustle, Gyakuten, Javert, Gyai, Gaiash, DogiGyaku, Ash, Suzu, Van, DarkMasters, Kernel, Sozanmaru, Laffrull, Kusabira, Shacogail, Madan,
     JoJoJo, Magny, Fujisan, Slotton, Brand, Jogimera, Melaventure, Jogiragon, Moujakinga, Niyani, Kesshin, Sakki, Manhattan,
     SaintCastle, Zaze, Ichion, Everlast, GrandCross, Willide, Emotional,
     Gorgi, Canzone, Akane, Heracles, Takasugi, Jasmine, Kohaku, Gindan, Aofesy, Cineraria, Vt5000,
     Odotta, Jolagon, Stella, Deaddam, Seikimatsu, Sekkei, Konpeki, Seihou, Paradiso, SixFall,
     Helcop, Canbero, Greatest, Momo, Cry, Bakuro, DeathLost, Dmc, Ichiou, Sangan, Shurai, Chitsujo,
     Drache, Esmel, Chiryu, Trap, Pearly, GreatBlue, Eringi, Atlantis, Scorpion, Levya, Valhalla, ManhattanT, Wokanna, Emeral, Alspa,
     EBrain, Kurin, Elborom, Azafaust, Risou, Grimoire, Shinkirou, Kakugo, Petens, Giri, LoveAbyss, Dolma, Dokin, Kamigami, Akashic, Efc,
     Tensai, GNext, Katsu, DogiRev, Nibai, Oudou, Dogidante, Felzark, Kouten]
}

pub fn def(c: C) -> &'static CardDef {
    &DEFS[c as usize]
}

const NONE: u16 = 0;

pub static DEFS: [CardDef; NCARDS] = [
    CardDef { id: "dm22ex1-037", main: side!("キユリのASMラジオ", Spell, N, 4, 0, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm24ex1-050", main: side!("ハンプティ・ルピア", Creature, L | D | F, 3, 3500, NONE, K_GSTRIKE), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm22rp1-049", main: side!("キャディ・ビートル", Creature, N, 2, 1000, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmd27-003", main: side!("単騎連射 マグナム", Creature, F, 3, 3000, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26rp1-S002", main: side!("世界のY チャクラ・デル・フィン", Creature, L, 3, 4000, R_DRAGON, 0), spell: None, hyper: Some((7500, K_BLOCKER | K_WB)), evo: Evo::None },
    CardDef { id: "dm26rp1-S009", main: side!("轟腕のR ダグラジャパニカン", Creature, N, 3, 3000, NONE, 0), spell: None, hyper: Some((6000, K_MACH | K_WB)), evo: Evo::None },
    CardDef { id: "dm25rp3-DM1", main: side!("轟く邪道 レッドゾーン", Creature, F, 6, 12000, R_COMMAND, K_TB), spell: None, hyper: None, evo: Evo::GNeo },
    CardDef { id: "dm25rp3-006", main: side!("PP-「P」", Creature, N, 3, 3000, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmrp22-037", main: side!("鬼星の絆 フウジン天＆ライジン天", Creature, F, 2, 2000, NONE, K_GSTRIKE), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm24ex1-051", main: side!("ピザスターのアンティハムト", Creature, L | F | N, 3, 3000, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm23ex3-012", main: side!("神判のカルマ コットン", Creature, L, 2, 2000, NONE, 0), spell: Some(side!("ジャッジ・水晶チャージャー", Spell, L, 3, 0, NONE, 0)), hyper: None, evo: Evo::None },

    CardDef { id: "dm26rp1-047", main: side!("我竜塔第七層 ハッスル・キャッスル", GCastle, N, 3, 0, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm24rp3-005", main: side!("深淵の逆転撃", Spell, D, 4, 0, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26ex1-N004", main: side!("邪幽 ジャヴェール", Creature, D | N, 5, 8000, R_DRAGON | R_ABYSS, K_MACH | K_WB), spell: Some(side!("「ヤベーのを見たいか？」", Spell, D | N, 2, 0, NONE, 0)), hyper: None, evo: Evo::None },
    CardDef { id: "dm26ex3-033", main: side!("地封院ギャイ＜まかしとき！＞", Creature, N, 5, 8000, R_DRAGON | R_COMMAND | R_SEITOKAI, K_MACH | K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmex17-021", main: side!("流星のガイアッシュ・カイザー", Creature, W | N, 6, 8000, R_DRAGON | R_COMMAND, K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26rp2-DM001", main: side!("烈しき切札 ドギラゴン逆", Creature, L | N, 8, 15000, R_DRAGON | R_COMMAND, K_BLOCKER | K_MACH | K_TB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26ex3-007", main: side!("流星アーシュ＜私が主役⁉＞", Creature, W, 3, 4000, R_DRAGON | R_COMMAND | R_SEITOKAI, K_JUSTDIVER), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26ex3-057", main: side!("熊田すず＜わらわは天才！＞", Creature, D, 4, 4000, R_DRAGON | R_COMMAND | R_SEITOKAI, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmx13-s02", main: side!("「修羅」の頂 VAN・ベートーベン", Creature, ZERO, 11, 14000, R_DRAGON | R_COMMAND, K_TB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmr17-006", main: side!("悪魔龍 ダークマスターズ", Creature, D, 7, 7000, R_DRAGON | R_COMMAND, K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25ex2-081", main: side!("青寂の精霊龍 カーネル", Creature, L | W, 5, 3500, R_DRAGON, K_STRIG | K_BLOCKER), spell: Some(side!("サイレント・ジェラシー", Spell, D, 2, 0, NONE, 0)), hyper: None, evo: Evo::None },
    CardDef { id: "dm26ex4-010", main: side!("切札混成 光隼 蒼斬丸", Creature, L | W, 6, 6000, R_DRAGON, K_EXLIFE | K_BLOCKER | K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmex17-020", main: side!("音卿の精霊龍 ラフルル・ラブ", Creature, L | W, 7, 7000, R_DRAGON | R_COMMAND, K_WB), spell: Some(side!("「未来から来る、だからミラクル」", Spell, L | W, 6, 0, NONE, 0)), hyper: None, evo: Evo::None },
    CardDef { id: "dm26ex4-009", main: side!("土神 クサビラガミ", Creature, N, 6, 12000, R_DRAGON, K_MACH | K_TB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmrp03-s03", main: side!("水上第九院 シャコガイル", Creature, W, 9, 13000, NONE, K_TB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25rp1-S03", main: side!("大崩壊炉 マーダン=ダイロウ", Creature, D, 7, 7000, R_ABYSS, K_BLOCKER | K_WB), spell: None, hyper: None, evo: Evo::Neo },

    CardDef { id: "dmsd01-011", main: side!("ジョジョジョ・ジョーカーズ", Spell, ZERO, 1, 0, R_JOKERS, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26rp3-042", main: side!("銃車逆夢 ザ・マグニー", Creature, F, 5, 5000, R_JOKERS, K_EXLIFE), spell: Some(side!("ジョリー・ザ・スクラッパー", Spell, F, 2, 0, R_JOKERS, 0)), hyper: None, evo: Evo::None },
    CardDef { id: "dm26rp3-S003", main: side!("Dr.富士山ッピング", Creature, W, 5, 6000, R_JOKERS, K_BLOCKER | K_JUSTDIVER | K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmrp06-s01", main: side!("キング・ザ・スロットン７", Creature, ZERO, 7, 7777, R_JOKERS, K_WB), spell: Some(side!("7777777", Spell, ZERO, 3, 0, R_JOKERS, K_STRIG)), hyper: None, evo: Evo::None },
    CardDef { id: "dm26rp3-S005", main: side!("“↑↑”ブランド", Creature, F, 7, 9000, R_JOKERS, K_SA | K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26ex1-N002", main: side!("ジョギメラゴン＆メラビート ～Jの焔舞～", Creature, F, 7, 10000, R_DRAGON | R_JOKERS, K_GSTRIKE | K_SA | K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25ex2-008", main: side!("メラベンチャー・ザ・ジョニー", Creature, F, 7, 10000, R_JOKERS, K_GSTRIKE | K_WB), spell: None, hyper: None, evo: Evo::Evo },
    CardDef { id: "dmex12-s03", main: side!("ジョギラゴン＆ジョニー ～Jの旅路～", Creature, ZERO, 8, 10000, R_DRAGON | R_JOKERS, K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26rp3-DM001", main: side!("勝敵逆夢王 モウジャキンガ", Creature, W | D | F, 8, 11000, R_DRAGON | R_JOKERS, K_EXLIFE | K_BLOCKER | K_SA | K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26rp3-013", main: side!("にやにYAH", Creature, L, 6, 3000, R_JOKERS, K_STRIG), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmex19-003", main: side!("勝熱と弾丸と自由の決断", Spell, ZERO, 4, 0, R_JOKERS, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmrp04s-s01", main: side!("最終兵ッキー", Creature, ZERO, 5, 5000, R_JOKERS, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmrp05-s08", main: side!("アイアン・マンハッタン", Creature, F, 9, 8000, R_JOKERS, K_SA | K_WB), spell: None, hyper: None, evo: Evo::None },

    CardDef { id: "dm26rp1-057", main: side!("我竜塔第一層 セイント・キャッスル", GCastle, L, 1, 0, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm24rp3-S02", main: side!("楯教の求道者 ザゼ・ゼーン", Creature, L, 2, 8000, NONE, K_BLOCKER | K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25rp2-012", main: side!("一音の妖精", Creature, L, 2, 2500, R_GIANT, K_GSTRIKE), spell: None, hyper: None, evo: Evo::Neo },
    CardDef { id: "dm26rp2-S002", main: side!("天V王 エバーラスト", Creature, L, 7, 12000, R_DRAGON | R_COMMAND, K_BLOCKER | K_TB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26rp1-024", main: side!("魂が休む庭園 グリーン・グランドクロス", GCastle, L | N, 2, 0, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm24rp1-S02", main: side!("ウィリデ・ゴル・ゲルス", Creature, L, 6, 8000, R_COMMAND, K_BLOCKER | K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmex12-s04", main: side!("神聖龍 エモーショナル・ハードコア", Creature, L, 7, 9500, R_DRAGON | R_ORACLE, K_BLOCKER | K_WB), spell: None, hyper: None, evo: Evo::None },

    CardDef { id: "dm25rp2-S07", main: side!("轟䡛合体 ゴルギーオージャー", Creature, L | W | N, 9, 14500, R_DRAGON, K_BLOCKER | K_MACH | K_TB), spell: None, hyper: None, evo: Evo::GNeo },
    CardDef { id: "dm25rp2-S01", main: side!("華謡の精霊カンツォーネ", Creature, L, 4, 5500, R_MAGIC | R_COMMAND, K_BLOCKER), spell: None, hyper: None, evo: Evo::Neo },
    CardDef { id: "dm25rp4-050", main: side!("大集合！アカネ&アサギ&コハク", Creature, L | W | N, 3, 3000, R_GIANT, 0), spell: None, hyper: None, evo: Evo::Neo },
    CardDef { id: "dm25rp2-024", main: side!("観覧！ホールインランド・ヘラクレス", Creature, L | W | N, 4, 6000, R_GIANT, K_BLOCKER | K_WB), spell: None, hyper: None, evo: Evo::Neo },
    CardDef { id: "dm25rp3-049", main: side!("TAKASUGI-死合乱闘3000", Creature, N, 3, 4000, R_GIANT, 0), spell: None, hyper: None, evo: Evo::Neo },
    CardDef { id: "dmsd20-015", main: side!("ジャスミンの地版", Tama, N, 2, 0, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25rp2-013", main: side!("ソウルサンライト コハク", Creature, L, 2, 2000, R_GIANT, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25rp4-030", main: side!("吟弾の妖精", Creature, L, 3, 3500, R_GIANT, K_STRIG), spell: None, hyper: None, evo: Evo::Neo },
    CardDef { id: "dm23rp4-014", main: side!("巨魔天 アオフェシー", Creature, W, 4, 6000, R_GIANT | R_MAGIC, K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm23rp4-030", main: side!("アシステスト・シネラリア", Creature, W, 3, 2000, R_GIANT, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm23ex1-003", main: side!("飛翔龍 5000VT", Creature, W, 8, 12000, R_MAGIC, K_JUSTDIVER | K_TB), spell: None, hyper: None, evo: Evo::None },

    CardDef { id: "dm26ex4-092", main: side!("チキン・オドッ太", Creature, F, 3, 3000, R_JOKERS, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26rp3-S002", main: side!("ジョット・首魁・ジョラゴン", Creature, L, 7, 11500, R_DRAGON | R_JOKERS, K_WB), spell: None, hyper: None, evo: Evo::Neo },
    CardDef { id: "dm25ex1-006", main: side!("天革の騎令嬢 ミラクルステラ", Creature, W, 6, 8000, R_DRAGON | R_COMMAND, K_GSTRIKE | K_BLOCKER | K_JUSTDIVER | K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmbd10-002", main: side!("SSS級天災 デッドダムド", Creature, W | D | N, 8, 11000, R_COMMAND, K_WB), spell: None, hyper: None, evo: Evo::Evo },
    CardDef { id: "dm25rp3-S06", main: side!("～世紀末の善悪～", Creature, L | W | D, 5, 6500, NONE, K_BLOCKER | K_WB), spell: None, hyper: None, evo: Evo::GNeo },
    CardDef { id: "dm25rp1-047", main: side!("超魂設計図", Spell, N, 2, 0, NONE, K_GSTRIKE), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25rp2-015", main: side!("～紺碧の奇術師～", Creature, W, 3, 3000, NONE, K_STRIG | K_BLOCKER), spell: None, hyper: None, evo: Evo::Neo },
    CardDef { id: "dm25rp1-S02", main: side!("～西方より来る激流の竜騎公～", Creature, W, 6, 6000, R_DRAGON, K_WB), spell: None, hyper: None, evo: Evo::Neo },
    CardDef { id: "dmrp09-s04", main: side!("パラディソ・シエル", Creature, W, 2, 13000, NONE, K_TB), spell: None, hyper: None, evo: Evo::Neo },
    CardDef { id: "dm25rp2-S02", main: side!("六番龍 シックスフォール Par滝", Creature, W, 4, 4000, R_DRAGON | R_GIANT | R_FUSION, 0), spell: None, hyper: None, evo: Evo::None },

    CardDef { id: "dmsd20-016", main: side!("ヘルコプ太の心絵", Tama, N, 1, 0, R_JOKERS | R_REX, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmrp19-s05", main: side!("キャンベロ <レッゾ.Star>", Creature, F, 5, 7000, R_JOKERS | R_COMMAND | R_REX, K_WB), spell: None, hyper: None, evo: Evo::Evo },
    CardDef { id: "dm26ex2-016", main: side!("鬼黒皇グレイテスト・ジャオウガ", Creature, D | F | N, 5, 7000, R_DEMONIO | R_REX, K_MACH | K_WB), spell: None, hyper: None, evo: Evo::Evo },
    CardDef { id: "dmrp19-km1", main: side!("禁断英雄 モモキングダムX", Creature, D | F, 2, 99999, R_JOKERS | R_REX, K_TB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26ex2-SPR005", main: side!("CRY-S-MAX ジャオウガ", Creature, D | F | N, 5, 8000, R_DEMONIO | R_REX, K_MACH | K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26ex2-065", main: side!("バクロ法師の封", Tama, N, 4, 0, R_DEMONIO | R_REX, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25ex2-026", main: side!("デス・ザ・ロストの黒像", Tama, D, 4, 0, R_DRAGON | R_COMMAND | R_REX, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm23rp4-S02", main: side!("DARK MATERIAL COMPLEX", Creature, D, 1, 25000, R_PANDORA, K_WORLD), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26ex2-034", main: side!("一王二命三眼槍の封", Tama, D | F | N, 5, 0, R_DEMONIO | R_REX, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmrp21-2B", main: side!("邪王来混沌三眼鬼", Creature, F, 5, 7000, R_DEMONIO | R_REX, K_WB), spell: None, hyper: None, evo: Evo::Evo },
    CardDef { id: "dmsd21-001", main: side!("終来王鬼 ジャオウガ", Creature, F, 5, 8000, R_DEMONIO | R_REX, K_WB), spell: None, hyper: None, evo: Evo::Evo },
    CardDef { id: "dmex12-035", main: side!("秩序の意志", Spell, D, 4, 0, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    // ---- 第2段階：創世竜（2026-11-26） ----
    CardDef { id: "dm25rp4-S07", main: side!("創世竜 Drache der'Zen", Creature, L | W | N, 3, 3500, R_MAGIC | R_COMMAND | R_DRAGON, 0), spell: None, hyper: None, evo: Evo::GNeo },
    CardDef { id: "dmex12-016", main: side!("天体妖精エスメル", Creature, N, 2, 2000, NONE, 0), spell: Some(side!("「お茶はいかがですか？」", Spell, L, 4, 0, NONE, K_STRIG)), hyper: None, evo: Evo::None },
    CardDef { id: "dmrp20-055", main: side!("地龍神の魔陣", Spell, W | N, 2, 0, NONE, K_GSTRIKE), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmrp21-074", main: side!("トラップの地版", Tama, N, 3, 0, R_GIANT | R_REX, K_STRIG), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmrp22-044", main: side!("パーリ騎士の心絵", Tama, N, 3, 0, R_JOKERS | R_REX, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25rp4-016", main: side!("グレートブルーの海幻", Tama, W, 3, 0, R_DRAGON | R_FUSION, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm24bd5-006", main: side!("宇宙妖精エリンギ", Creature, L | W | N, 5, 4500, NONE, K_STRIG | K_BLOCKER), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmrp22-013", main: side!("アトランティスの海幻", Tama, W, 5, 0, R_REX, K_STRIG), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm22rp1-8A", main: side!("轟廻！グランドスラム・スコーピオン", Tama, N, 5, 0, R_GIANT, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmrp22-025", main: side!("レヴィヤの地版", Tama, N, 5, 0, R_DRAGON | R_COMMAND | R_REX, K_STRIG), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25ex1-054", main: side!("ヴァルハラの天宝", Tama, L, 3, 0, R_DRAGON | R_COMMAND, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmrp22-024", main: side!("マンハッタンの心絵", Tama, N, 6, 0, R_JOKERS | R_REX, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm24ex1-038", main: side!("冥土人形ウォカンナ・ピエール", Creature, W, 3, 2000, R_PUPPET, K_STRIG | K_BLOCKER), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmrp22-031", main: side!("エメラルの海幻", Tama, W, 3, 0, R_REX, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm24rp4-002", main: side!("♪立ち上がる 悪魔に天使 堕ちるかな", Spell, W, 4, 0, NONE, K_STRIG), spell: None, hyper: None, evo: Evo::None },
    // ---- 第3段階：4Cギリガザミ（2026-11-26） ----
    CardDef { id: "dmex18-018", main: side!("エターナル・ブレイン", Spell, W, 2, 0, NONE, 0), spell: Some(side!("ブレイン珊瑚の仙樹", Spell, W, 8, 0, NONE, 0)), hyper: None, evo: Evo::None },
    CardDef { id: "dm26ex1-N017", main: side!("冥土人形クリン・ラビィ", Creature, W, 3, 3000, R_PUPPET, K_JUSTDIVER), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm24rp2-006", main: side!("超光喜 エルボロム", Creature, L | W, 4, 6000, R_COMMAND, K_BLOCKER | K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25rp3-018", main: side!("魔誕の覇王アザファウスト", Creature, F, 4, 4000, R_COMMAND, K_STRIG), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm24rp2-S10", main: side!("理想と平和の決断", Spell, L | W, 4, 0, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26rp3-015", main: side!("裏魔導書グリモア-Ⅰ章", Spell, W, 4, 0, NONE, K_STRIG | K_SST), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25rp1-S07", main: side!("真気楼と誠偽感の決断", Spell, L | W | D, 5, 0, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26ex2-031", main: side!("覚悟の決闘", Spell, L | W | F, 6, 0, NONE, K_STRIG), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25rp4-010", main: side!("ペテンズ・ゲート", Spell, L | W | D, 8, 0, NONE, K_STRIG), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmrp04m-010", main: side!("唸る鉄腕 ギリガザミ", Creature, W, 8, 2000, NONE, K_STRIG | K_SST), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25rp4-004", main: side!("LOVE＆ABYSS", Spell, D, 9, 0, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmex01-072", main: side!("ドルマゲドン・ビッグバン", Spell, D | F, 9, 0, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmrp18-km3", main: side!("禁時混成王 ドキンダンテXXII", Creature, L | W | F, 9, 99999, R_COMMAND | R_DRAGON, K_EXLIFE | K_TB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm05-011", main: side!("神々の逆流", Spell, W, 9, 0, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm23ex2-009", main: side!("完全水中要塞 アカシック3", Tama, W, 10, 0, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmex12-020", main: side!("エンドレス・フローズン・カーニバル", Spell, L | W | N, 15, 0, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    // ---- 第3段階：4Cドギラゴン王道刈（2026-11-26） ----
    CardDef { id: "dm25ex1-057", main: side!("裏斬隠 テンサイ・ハート", Creature, W, 3, 3000, NONE, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25rp4-012", main: side!("創世竜 ゴルギーネクスト", Creature, L, 3, 4000, R_DRAGON, 0), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dmex15-b002", main: side!("切札勝太＆カツキング -熱血の物語-", Creature, W | F | N, 5, 5000, R_DRAGON, K_MACH), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm24bd1-001", main: side!("王道の革命 ドギラゴン", Creature, L | F | N, 5, 5000, R_COMMAND | R_DRAGON, K_BLOCKER | K_SA), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26rp1-S011", main: side!("斬隠将撃龍ニバイケン", Creature, W | F, 6, 6000, R_DRAGON, K_BLOCKER | K_SA | K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25rp4-DM1", main: side!("剣轟の団長 ドギラゴン王道", Creature, L | F | N, 8, 13000, R_COMMAND | R_DRAGON, K_TB), spell: None, hyper: None, evo: Evo::GNeo },
    CardDef { id: "dm26ex4-014", main: side!("切札混成 ドギラダンテ刈", Creature, L | W | F | N, 8, 13000, R_COMMAND | R_DRAGON, K_EXLIFE | K_TB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm26ex4-037", main: side!("切札電融 デス・フェルザーク", Creature, D | F | N, 6, 9000, R_DRAGON, K_EXLIFE | K_SA | K_WB), spell: None, hyper: None, evo: Evo::None },
    CardDef { id: "dm25rp4-S09", main: side!("皇天闘と炎天華の決断", Spell, L | F | N, 3, 0, NONE, 0), spell: None, hyper: None, evo: Evo::None },
];

pub fn by_id(id: &str) -> Option<C> {
    all().into_iter().find(|&c| def(c).id == id)
}

/// 学習用の3デッキ（decks.json と同じ）
pub fn deck(name: &str) -> Vec<C> {
    use C::*;
    let list: &[(u8, C)] = match name {
        "leasebike" => &[(4, Kiyuri), (4, Humpty), (4, Caddy), (1, Magnum), (4, Chakra), (4, Dagura), (4, RedZone), (4, PPP), (3, Fujin), (4, Antiham), (4, Cotton)],
        "hustle" => &[(4, Hustle), (4, Gyakuten), (4, Javert), (4, Gyai), (3, Gaiash), (3, DogiGyaku), (4, Ash), (2, Suzu), (1, Van), (1, DarkMasters), (4, Kernel), (2, Sozanmaru), (1, Laffrull), (1, Kusabira), (1, Shacogail), (1, Madan)],
        "jokers" => &[(4, JoJoJo), (4, Magny), (4, Fujisan), (4, Slotton), (4, Brand), (4, Jogimera), (4, Melaventure), (4, Jogiragon), (2, Moujakinga), (3, Niyani), (1, Kesshin), (1, Sakki), (1, Manhattan)],
        "willide" => &[(1, SaintCastle), (4, Zaze), (4, Chakra), (4, Hustle), (1, Ichion), (2, Everlast), (4, DogiGyaku), (3, Cotton), (4, Gyai), (4, Caddy), (4, GrandCross), (4, Willide), (1, Emotional)],
        "gorgi" => &[(4, Gorgi), (3, Canzone), (4, Akane), (4, Heracles), (4, Ichion), (3, Takasugi), (4, Jasmine), (4, Kohaku), (1, Gindan), (4, Aofesy), (3, Cineraria), (1, Vt5000), (1, Shacogail)],
        // ---- DMラボ環境デッキ（2026-11-26、代表リスト /mnt/project-files/meta-decks/）----
        "zaze" => &[(1, SaintCastle), (4, Zaze), (4, Chakra), (4, Cotton), (3, Caddy), (4, Hustle), (3, Dagura), (1, Emotional), (4, Gyai), (4, Everlast), (2, Ichion), (2, PPP), (4, DogiGyaku)],
        "seitokai" => &[(4, Ash), (3, Suzu), (4, Gyai), (4, DogiGyaku), (4, Hustle), (4, Gaiash), (3, Stella), (1, Deaddam), (3, DarkMasters), (4, Gyakuten), (4, Javert), (2, PPP)],
        "dogi4c" => &[(4, Hustle), (4, Javert), (4, Gyakuten), (4, Kernel), (4, Gyai), (4, Seikimatsu), (3, Stella), (1, Laffrull), (1, Van), (3, Gaiash), (3, Sozanmaru), (2, PPP), (3, DogiGyaku)],
        "jokers_meta" => &[(4, JoJoJo), (4, Odotta), (4, Fujisan), (4, Brand), (4, Jogimera), (4, Melaventure), (4, Slotton), (4, Jogiragon), (3, Niyani), (2, Moujakinga), (1, Sakki), (1, Jolagon), (1, Manhattan)],
        "gorgi_trv" => &[(4, Gorgi), (4, Heracles), (4, Akane), (4, Kohaku), (4, Ichion), (4, Canzone), (4, Sekkei), (4, Caddy), (2, Konpeki), (1, Seihou), (1, Shacogail), (3, Paradiso), (1, SixFall)],
        "demonio" => &[(4, Helcop), (4, Canbero), (4, Jasmine), (4, Greatest), (4, Momo), (4, Cry), (4, Bakuro), (3, DeathLost), (1, Dmc), (3, Ichiou), (2, Sangan), (1, Shurai), (2, Chitsujo)],
        "sousei" => &[(4, Akane), (4, Drache), (3, Jasmine), (1, Esmel), (4, Chiryu), (4, Trap), (4, Pearly), (2, GreatBlue), (4, Eringi), (2, Atlantis), (2, Scorpion), (1, Levya), (1, Valhalla), (1, ManhattanT), (1, Wokanna), (1, Emeral), (1, Alspa)],
        "giri4c" => &[(2, EBrain), (4, Kurin), (3, Elborom), (1, Azafaust), (3, Risou), (4, Grimoire), (4, Shinkirou), (4, Kakugo), (4, Petens), (4, Giri), (1, LoveAbyss), (1, Dolma), (1, Dokin), (1, Kamigami), (1, Shacogail), (1, Akashic), (1, Efc)],
        "oudou4c" => &[(4, Tensai), (4, Hustle), (4, GNext), (4, Akane), (3, PPP), (3, Katsu), (3, DogiRev), (3, Nibai), (3, Oudou), (3, Dogidante), (2, Felzark), (2, Brand), (1, Kouten), (1, Laffrull)],
        "bike_meta" => &[(4, Kiyuri), (4, Antiham), (4, Cotton), (4, Humpty), (4, Caddy), (4, Chakra), (4, Dagura), (4, RedZone), (3, PPP), (4, Fujin), (1, Magnum)],
        _ => panic!("unknown deck {name}"),
    };
    let mut v = Vec::new();
    for &(n, c) in list {
        for _ in 0..n {
            v.push(c);
        }
    }
    assert_eq!(v.len(), 40, "{name}");
    v
}

/// 0〜4 は学習中（train5）の5デッキ。5〜 は DMラボの環境デッキ（2026-11-26 追加、train6 から）
pub const DECKS: [&str; 15] = ["leasebike", "hustle", "jokers", "willide", "gorgi", "zaze", "seitokai", "dogi4c", "jokers_meta", "gorgi_trv", "bike_meta", "demonio", "sousei", "giri4c", "oudou4c"];
