use crate::core::*;
use crate::resources::*;
use crate::yaku::*;
use crate::components::*;

#[derive(Debug, Clone)]
pub struct HandResult {
    pub yaku_names: Vec<String>,
    pub dora_count: u8,
    pub ura_dora_count: u8,
    pub total_han: u8,
    pub total_fu: u8,
    pub is_yakuman: bool,
}

impl HandResult {
    pub fn shot_count_from_result(&self) -> u8 {
        if self.is_yakuman { return 3; }
        match self.total_han {
            0..=5 => 0,
            6..=7 => 1,    // haneman
            8..=10 => 2,   // baiman
            11..=12 => 2,  // sanbaiman
            13.. => 3,     // kazoe yakuman
        }
    }

    pub fn is_low_han(&self) -> bool {
        !self.is_yakuman && self.total_han < 2
    }
}

pub struct DoraCount {
    pub dora: u8,
    pub ura_dora: u8,
    pub additional_han: u8,
}

#[derive(Debug)]
pub struct ScorePayout {
    pub total_won: u32,
    pub oya_pays: u32,
    pub non_oya_pays: u32,
}

pub fn get_dora_from_indicator(indicator: &Tile) -> Tile {
    match indicator {
        Tile::Man(num) => Tile::Man((num % 9) + 1),
        Tile::Pin(num) => Tile::Pin((num % 9) + 1),
        Tile::Sou(num) => Tile::Sou((num % 9) + 1),
        Tile::Honor(Honor::East) => Tile::Honor(Honor::South),
        Tile::Honor(Honor::South) => Tile::Honor(Honor::West),
        Tile::Honor(Honor::West) => Tile::Honor(Honor::North),
        Tile::Honor(Honor::North) => Tile::Honor(Honor::East),
        Tile::Honor(Honor::White) => Tile::Honor(Honor::Green),
        Tile::Honor(Honor::Green) => Tile::Honor(Honor::Red),
        Tile::Honor(Honor::Red) => Tile::Honor(Honor::White),
    }
}

pub fn count_dora(combined_hand: &[Tile], dead_wall: &DeadWall, is_riichi: bool) -> DoraCount {
    let mut dora_count = DoraCount {
        dora: 0,
        ura_dora: 0,
        additional_han: 0,
    };
    for tile in combined_hand.iter() {
        for dora in dead_wall.dora_indicators.iter() {
            if *tile == get_dora_from_indicator(dora) {
                dora_count.dora += 1;
                dora_count.additional_han += 1;
            }
        }
        if is_riichi {
            for ura in dead_wall.ura_indicators.iter() {
                if *tile == get_dora_from_indicator(ura) {
                    dora_count.ura_dora += 1;
                    dora_count.additional_han += 1;
                }
            }
        }
    }
    dora_count
}


pub fn calculate_fu(
    result: &[Mentsu],
    winning_tile: &Tile,
    jikaze: &Wind,
    bakaze: &Wind,
    is_tsumo: bool,
    is_hand_closed: bool
) -> u8 {
    let mut fu: u8 = 20;

    if is_tsumo {
        fu += 2; // exception for pinfu tsumo later
    } else if is_hand_closed {
        fu += 10; // menzen ron
    }

    for mentsu in result.iter() {
        match mentsu {
            Mentsu::Koutsu(tiles, true) => {
                fu += if is_yaochuuhai(&tiles[0]) {8} else {4};
            }
            Mentsu::Koutsu(tiles, false) => {
                fu += if is_yaochuuhai(&tiles[0]) {4} else {2};
            }
            Mentsu::Ankan(tiles) => {
                fu += if is_yaochuuhai(&tiles[0]) {32} else {16};
            }
            Mentsu::Daiminkan(tiles) | Mentsu::Shouminkan(tiles) => {
                fu += if is_yaochuuhai(&tiles[0]) {16} else {8};
            }
            Mentsu::Jantou(tiles) => {
                let tile = &tiles[0];
                if let Tile::Honor(Honor::Red | Honor::Green | Honor::White) = tile {
                    fu += 2;
                }
                // these stack
                if let Tile::Honor(h) = tile {
                    if *h == jikaze.wind_to_honor() { 
                        fu += 2; 
                    }
                    if *h == bakaze.wind_to_honor() { 
                        fu += 2;
                    }
                }
            }
            _ => {} // shuntsu
        }
    }
    //? wait a minute, i can just do `if !is_ryanmen`...???
    if is_penchan_wait(result, winning_tile) || is_kanchan_wait(result, winning_tile) || is_tanki_wait(result, winning_tile) {
        fu += 2;
    }

    fu
}


pub fn calculate_score(han: u8, fu: u8, is_oya: bool, is_tsumo: bool, is_yakuman: bool, yaku_names: &[String]) -> ScorePayout {
    if is_yakuman {
        if is_oya { 
            if is_tsumo {
                return ScorePayout {
                    total_won: (yaku_names.len() * 48000) as u32, // TODO: multiple by 3 (no this stays the same)
                    oya_pays: 0,
                    non_oya_pays: (yaku_names.len() * 48000 / 3) as u32,
                }; 
            } else {
                return ScorePayout {
                    total_won: (yaku_names.len() * 48000) as u32,
                    oya_pays: 0,
                    non_oya_pays: (yaku_names.len() * 48000) as u32,
                };
            }
        } else { 
            if is_tsumo {
                return ScorePayout {
                    total_won: (yaku_names.len() * 32000) as u32,
                    oya_pays: (yaku_names.len() * 32000 / 2) as u32,
                    non_oya_pays: ((yaku_names.len() * 32000 / 2) / 2) as u32,
                }; 
            } else {
                return ScorePayout {
                    total_won: (yaku_names.len() * 32000) as u32,
                    oya_pays: 0,
                    non_oya_pays: (yaku_names.len() * 32000) as u32,
                }; 
            }
           
        } 
    }

    // https://riichi.wiki/Japanese_mahjong_scoring_rules
    // vanilla: base = fu as u32 * 2_u32.pow((han + 2 ).into());
    let mut base = fu as u32 * 2_u32.pow((han + 2 ).into()); 

    if base > 2000 || han > 5 || (han >= 4 && fu >= 40) || (han >= 3 && fu >= 70) {
        match han {
            0..=5 => base = 2000,
            6..=7 => base = 3000,
            8..=10 => base = 4000,
            11..=12 => base = 6000,
            _ =>  base = 8000, // ? this is probably unnecessary
        }
    }

    if is_oya && is_tsumo {
        let non_oya_payout = (base * 2).div_ceil(100) * 100;
        ScorePayout{
            total_won: non_oya_payout * 3,
            oya_pays: 0,
            non_oya_pays: non_oya_payout,
        }
    } else if is_oya && !is_tsumo {
        let ron_payout = (base * 6).div_ceil(100) * 100;
        ScorePayout{
            total_won: ron_payout,
            oya_pays: 0,
            non_oya_pays: ron_payout,
        }
    } else if  !is_oya && is_tsumo {
        let oya_payout = (base * 2).div_ceil(100) * 100;
        let non_oya_payout = base.div_ceil(100) * 100;
        ScorePayout{
            total_won: oya_payout + (non_oya_payout * 2),
            oya_pays: oya_payout,
            non_oya_pays: non_oya_payout,
        }
    } else {
        let ron_payout = (base * 4).div_ceil(100) * 100;
        ScorePayout{
            total_won: ron_payout,
            oya_pays: ron_payout,
            non_oya_pays: ron_payout,
        }
    }
}


pub fn evaluate_yaku(
    results: &[Vec<Mentsu>],
    thirteen_tiles: &[Tile],
    raw_hand: &[Tile],
    combined_hand: &[Tile],
    open_mentsu: &[Mentsu],
    is_hand_closed: bool,
    is_oya: bool,
    is_riichi: bool,
    is_double_riichi: bool,
    is_ippatsu: bool,
    bakaze: &Wind,
    jikaze: &Wind,
    kawa: &Kawa,
    winning_tile: &Tile,
    is_tsumo: bool,
    is_rinshan: bool,
    is_chankan: bool,
    wall: &Wall,
    dead_wall: &DeadWall,
    calls_made: bool,
) -> HandResult {
    let mut best = HandResult {
        yaku_names: vec![],
        dora_count: 0,
        ura_dora_count: 0,
        total_han: 0,
        total_fu: 0,
        is_yakuman: false,
    };

    // path 1
    if is_hand_closed && kokushi_musou(raw_hand) {
        let mut eval = HandResult {
            yaku_names: vec!["Kokushi Musou".to_string()],
            dora_count: 0,
            ura_dora_count: 0,
            total_han: 0, 
            total_fu: 0, 
            is_yakuman: true,
        };
        add_situational_yakuman(&mut eval, kawa, is_oya, is_tsumo, calls_made);
        return eval;
    }

    // 2
    for result in results {
        let eval = evaluate_standard(
            result, thirteen_tiles, combined_hand, open_mentsu,
            is_hand_closed, is_oya, is_riichi, is_double_riichi,
            is_ippatsu, bakaze, jikaze, kawa, winning_tile,
            is_tsumo, is_rinshan, is_chankan, wall, calls_made
        );
        if is_better(&eval, &best) { 
            best = eval; 
        }
    }

    // 3
    if is_hand_closed && chiitoitsu(raw_hand) {
        let eval = evaluate_chiitoitsu(
            raw_hand, is_riichi, is_double_riichi, is_ippatsu,
            is_tsumo, is_chankan, is_oya, kawa, wall, calls_made
        );
        if is_better(&eval, &best) { 
            best = eval; 
        }
    }

    if !best.yaku_names.is_empty() && !best.is_yakuman {
        let dora_count = count_dora(combined_hand, dead_wall, is_riichi);
        best.dora_count = dora_count.dora;
        best.ura_dora_count = dora_count.ura_dora;
        best.total_han += dora_count.additional_han;
    }

    best
}


pub fn evaluate_standard(
    result: &[Mentsu],
    thirteen_tiles: &[Tile],
    combined_hand: &[Tile],
    open_mentsu: &[Mentsu],
    is_hand_closed: bool,
    is_oya: bool,
    is_riichi: bool,
    is_double_riichi: bool,
    is_ippatsu: bool,
    bakaze: &Wind,
    jikaze: &Wind,
    kawa: &Kawa,
    winning_tile: &Tile,
    is_tsumo: bool,
    is_rinshan: bool,
    is_chankan: bool,
    wall: &Wall,
    calls_made: bool,
) -> HandResult {
    let mut eval = HandResult {
        yaku_names: vec![],
        dora_count: 0,
        ura_dora_count: 0,
        total_han: 0, 
        total_fu: 0, 
        is_yakuman: false,
    };


    // yakuman
    if is_hand_closed && chuuren_poutou(combined_hand) {
        eval.yaku_names.push("Chuuren Poutou".to_string());
        eval.is_yakuman = true;
    }

    if suuankou(result, winning_tile, is_tsumo) {
        eval.yaku_names.push("Suuankou".to_string());
        eval.is_yakuman = true;
    }

    if daisuushii(result) {
        eval.yaku_names.push("Daisuushii".to_string());
        eval.is_yakuman = true;
    }

    if shousuushii(result) {
        eval.yaku_names.push("Shousuushii".to_string());
        eval.is_yakuman = true;
    }

    if daisangen(result) {
        eval.yaku_names.push("Daisangen".to_string());
        eval.is_yakuman = true;
    }

    if tsuuisou(combined_hand) {
        eval.yaku_names.push("Tsuuiisou".to_string());
        eval.is_yakuman = true;
    }

    if chinroutou(combined_hand) {
        eval.yaku_names.push("Chinroutou".to_string());
        eval.is_yakuman = true;
    }

    if ryuuiisou(combined_hand) {
        eval.yaku_names.push("Ryuuiisou".to_string());
        eval.is_yakuman = true;
    }

    if suukantsu(open_mentsu) {
        eval.yaku_names.push("Suukantsu".to_string());
        eval.is_yakuman = true;
    }

    add_situational_yakuman(&mut eval, kawa, is_oya, is_tsumo, calls_made);

    if eval.is_yakuman {
        return eval;
    }

    // upgradable yaku
    if chinitsu(combined_hand) {
        eval.yaku_names.push("Chinitsu".to_string());
        eval.total_han += if is_hand_closed { 6 } else { 5 };
    } else if honitsu(combined_hand) {
        eval.yaku_names.push("Honitsu".to_string());
        eval.total_han += if is_hand_closed { 3 } else { 2 };
    }

    if honroutou(combined_hand) {
        eval.yaku_names.push("Honroutou".to_string());
        eval.total_han += 2;
    } else if junchan(result) {
        eval.yaku_names.push("Junchan".to_string());
        eval.total_han += if is_hand_closed { 3 } else { 2 };
    } else if chanta(result) {
        eval.yaku_names.push("Chanta".to_string());
        eval.total_han += if is_hand_closed { 2 } else { 1 };
    }

    if sankantsu(open_mentsu) {
        eval.yaku_names.push("Sankantsu".to_string());
        eval.total_han += 2;
    } else if ryankantsu(open_mentsu) { // ! custom yaku
        eval.yaku_names.push("Ryankantsu".to_string());
        eval.total_han += 1;
    }

    if is_hand_closed {
        if ryanpeikou(result) {
            eval.yaku_names.push("Ryanpeikou".to_string());
            eval.total_han += 3;
        } else if iipeikou(result) {
            eval.yaku_names.push("Iipeikou".to_string());
            eval.total_han += 1;
        }
    }

    // kuitan
    if tanyao(combined_hand) {
        eval.yaku_names.push("Tanyao".to_string());
        eval.total_han += 1;
    }

    if ittsuu(result) {
        eval.yaku_names.push("Ittsuu".to_string());
        eval.total_han += if is_hand_closed { 2 } else { 1 };
    }

    if sanshoku_doujun(result) {
        eval.yaku_names.push("Sanshoku Doujun".to_string());
        eval.total_han += if is_hand_closed { 2 } else { 1 };
    }

    if sanshoku_doukou(result) {
        eval.yaku_names.push("Sanshoku Doukou".to_string());
        eval.total_han += 2;
    }

    if toitoi(result) {
        eval.yaku_names.push("Toitoi".to_string());
        eval.total_han += 2;
    }

    if sanankou(result, winning_tile, is_tsumo, thirteen_tiles) {
        eval.yaku_names.push("Sanankou".to_string());
        eval.total_han += 2;
    }

    if shousangen(result) {
        eval.yaku_names.push("Shousangen".to_string());
        eval.total_han += 2;
    }


    if is_hand_closed && pinfu(result, winning_tile, jikaze, bakaze) {
        eval.yaku_names.push("Pinfu".to_string());
        eval.total_han += 1;
    }

    let yakuhai = yakuhai(result, jikaze, bakaze);
    if yakuhai > 0 {
        eval.yaku_names.push(format!("Yakuhai ({} sets)", yakuhai));
        eval.total_han += yakuhai;
    }

    // situational
    add_situational(&mut eval, is_hand_closed, is_riichi, is_double_riichi,
        is_ippatsu, is_tsumo, is_rinshan, is_chankan, wall);
    
    if eval.yaku_names.contains(&"Pinfu".to_string()) {
        eval.total_fu = if is_tsumo { 20 } else { 30 };
    } else {
        let raw_fu = calculate_fu(result, winning_tile, jikaze, bakaze, is_tsumo, is_hand_closed);
        eval.total_fu = raw_fu.div_ceil(10) * 10; //(raw_fu + 9) / 10 * 10;
    }
    
    eval
}


pub fn evaluate_chiitoitsu(
    raw_hand: &[Tile],
    is_riichi: bool,
    is_double_riichi: bool,
    is_ippatsu: bool,
    is_tsumo: bool,
    is_chankan: bool,
    is_oya: bool,
    kawa: &Kawa,
    wall: &Wall,
    calls_made: bool,
) -> HandResult {
    let mut eval = HandResult {
        yaku_names: vec!["Chiitoitsu".to_string()],
        dora_count: 0,
        ura_dora_count: 0,
        total_han: 2,
        total_fu: 25, // always fixed
        is_yakuman: false,
    };

    // yakuman 
    if tsuuisou(raw_hand) {
        eval.yaku_names.clear();
        eval.yaku_names.push("Tsuuiisou".to_string());
        eval.is_yakuman = true;
    }

    add_situational_yakuman(&mut eval, kawa, is_oya, is_tsumo, calls_made);

    if eval.is_yakuman {
        eval.yaku_names.remove(0);
        return eval;
    }

    // compatible yaku only
    if tanyao(raw_hand) {
        eval.yaku_names.push("Tanyao".to_string());
        eval.total_han += 1;
    }

    if chinitsu(raw_hand) {
        eval.yaku_names.push("Chinitsu".to_string());
        eval.total_han += 6;
    } else if honitsu(raw_hand) {
        eval.yaku_names.push("Honitsu".to_string());
        eval.total_han += 3;
    }

    if honroutou(raw_hand) {
        eval.yaku_names.push("Honroutou".to_string());
        eval.total_han += 2;
    }

    // chiitoitsu is always closed, rinshan impossible (no kan)
    add_situational(&mut eval, true, is_riichi, is_double_riichi,
        is_ippatsu, is_tsumo, false, is_chankan, wall);

    eval
}


pub fn add_situational_yakuman(eval: &mut HandResult, kawa: &Kawa, is_oya: bool, is_tsumo: bool, calls_made: bool) {
    
    if tenhou(kawa, is_oya, is_tsumo, calls_made) {
        eval.yaku_names.push("Tenhou".to_string());
        eval.is_yakuman = true;
    }

    if chiihou(kawa, is_oya, is_tsumo, calls_made) {
        eval.yaku_names.push("Chiihou".to_string());
        eval.is_yakuman = true;
    }

}


pub fn add_situational(
    eval: &mut HandResult,
    is_hand_closed: bool,
    is_riichi: bool,
    is_double_riichi: bool,
    is_ippatsu: bool,
    is_tsumo: bool,
    is_rinshan: bool,
    is_chankan: bool,
    wall: &Wall,
) {

    if is_hand_closed && is_tsumo {
        eval.yaku_names.push("Menzen Tsumo".to_string());
        eval.total_han += 1;
    }

    // TODO: add is_double flag
    if is_double_riichi {
        eval.yaku_names.push("Double Riichi".to_string());
        eval.total_han += 2;
    } else if is_riichi {
        eval.yaku_names.push("Riichi".to_string());
        eval.total_han += 1;
    }

    if is_riichi && is_ippatsu {
        eval.yaku_names.push("Ippatsu".to_string());
        eval.total_han += 1;
    }

    if is_rinshan && is_tsumo {
        eval.yaku_names.push("Rinshan Kaihou".to_string());
        eval.total_han += 1;
    }

    if is_chankan && !is_tsumo {
        eval.yaku_names.push("Chankan".to_string());
        eval.total_han += 1;
    }

    if haitei(wall, is_tsumo) {
        eval.yaku_names.push("Haitei".to_string());
        eval.total_han += 1;
    }

    if houtei(wall, is_tsumo) {
        eval.yaku_names.push("Houtei".to_string());
        eval.total_han += 1;
    }

}


pub fn is_better(new: &HandResult, old: &HandResult) -> bool {
    if new.is_yakuman && !old.is_yakuman { 
        return true; 
    }
    if !new.is_yakuman && old.is_yakuman { 
        return false; 
    }
    if new.is_yakuman && old.is_yakuman {
        return new.yaku_names.len() > old.yaku_names.len();
    }
    if new.total_han != old.total_han {
        return new.total_han > old.total_han;
    }
    new.total_fu > old.total_fu
}