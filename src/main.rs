// TODO remaining todos
// Dora counting (not a yaku but affects scoring) (Done?)
// Fu calculation (done?)
// Han → Score conversion table (done?)
// custom yaku and rules later
// ? 途中流局


// TODO: return options for some of these (no)

use bevy::prelude::*;
use rand::{RngExt, seq::SliceRandom};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
enum Tile {
    Honor(Honor),
    Man(u8),
    Pin(u8),
    Sou(u8),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
enum Honor {
    White,
    Red,
    Green,
    North,
    West,
    East,
    South,
}

// TODO: test and change to array later 
#[derive(PartialEq, Eq, Clone, PartialOrd, Ord)]
enum Mentsu {
    Jantou(Vec<Tile>),
    Koutsu(Vec<Tile>, bool), // true = closed
    Shuntsu(Vec<Tile>, bool),
    Ankan(Vec<Tile>),
    Daiminkan(Vec<Tile>),
    Shouminkan(Vec<Tile>),
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Wind {
    East,
    South,
    West,
    North
}

#[derive(PartialEq, Eq)]
enum ChiTilePos { // tile drawn/discarded
    Left,  
    Middle, 
    Right,  
}

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
enum TurnState {
    #[default]
    Setup,
    StartNewRound,      
    Draw,           
    MainPhase,      
    CallWindow,     
    AdvanceTurn,   
    RinshanDraw,
    RoundEnd,
}

enum RoundEndReason {
    OyaWin,             // renchan
    NonOyaWin,
    RyuukyokuOyaTenpai, // renchan
    RyuukyokuOyaNoten,
    TochuuRyuukyoku,
}

#[derive(Resource)]
struct RoundResult(RoundEndReason);


#[derive(Resource)]
struct CurrentTurn(Entity); // id of the current tsumo

#[derive(Resource)]
struct GameState {
    rounds: u8,
    honba: u8,
    bakaze: Wind,
    bullet: u8,
    calls_made: bool,  // ! IMPORTANT: removed after the first call
    riichi_points: u32,
    pending_kan_dora: bool,
    pending_rinshan: bool,
}

#[derive(Resource)]
struct Wall(Vec<Tile>);

#[derive(Resource)]
struct DeadWall {
    pub dora_indicators: Vec<Tile>,
    pub ura_indicators: Vec<Tile>,
    pub rinshan_tiles: Vec<Tile>,
    pub filler_tiles: Vec<Tile>, // The remaining face-down tiles to maintain the 14 count
}


// components
#[derive(Component)]
struct PlayerTag;

#[derive(Component)]
struct Jikaze(Wind);

#[derive(Component)]
struct Points(i32);

#[derive(Component)]
struct Hand(Vec<Tile>);

#[derive(Component)]
struct OpenMentsu(Vec<Mentsu>);

// markers
#[derive(Component)]
struct Oya;

#[derive(Component)]
struct ClosedHand;

#[derive(Component)]
struct Tenpai(Vec<Tile>);

#[derive(Component)]
struct Furiten;

#[derive(Component)]
struct Riichi {
    turns_since: u8,
    is_ippatsu_alive: bool,
    is_double: bool,
}

#[derive(Component)]
struct Ippatsu;

#[derive(Component)]
struct DoubleRiichi;

#[derive(Component)]
struct Alive;

#[derive(Component)]
struct Kawa(Vec<Tile>);

// !a component to an entity (each tile is its own entity)
#[derive(Component)] 
struct DiscardedTile(Tile);

#[derive(Component)]
struct DiscardedBy(Entity);

#[derive(Component)]
struct CurrentDiscard;


#[derive(Message)]
struct DeclarePonMessage {
    player: Entity,       // gets the specific player (important)
    tile: Tile,           
}

#[derive(Message)]
struct DeclareChiMessage {
    player: Entity,       
    tile: Tile,           
    pos: ChiTilePos,  
    discarded_by: Entity,    
}

#[derive(Message)]
struct DeclareKanMessage {
    player: Entity,       
    tile: Tile,           
    is_discard: bool,
}

#[derive(Message)]
struct DeclareRiichiMessage {
    player: Entity,       
}

// for ron
#[derive(Component)]
struct Chankan;

//
#[derive(Component)]
struct RonOption {
    discarded_by: Entity,
    result: HandResult,
}

#[derive(Message)]
struct DeclareRonMessage {
    player: Entity,
    discarded_by: Entity,
    result: HandResult,
}

#[derive(Component)]
struct TsumoOption {
    result: HandResult,
}

#[derive(Message)]
struct DeclareTsumoMessage {
    player: Entity,
    result: HandResult,
}

#[derive(Message)]
struct DeclareKyuushuMessage {
    player: Entity,
}

#[derive(Component)]
struct DrawnTile(Tile);

#[derive(Message)]
struct DiscardTileMessage {
    player: Entity,
    tile: Tile,
    is_tsumogiri: bool, 
}

#[derive(Resource)]
struct CallWindowTimer(Timer);

#[derive(Debug, Clone)]
struct HandResult {
    yaku_names: Vec<String>,
    total_han: u8,
    total_fu: u8,
    is_yakuman: bool,
}

#[derive(Debug)]
struct ScorePayout {
    pub total_won: u32,
    pub oya_pays: u32,
    pub non_oya_pays: u32,
}


fn check_ryuukyoku(
    oya_tenpai_query: Single<Has<Tenpai>, With<Oya>>,
    wall: Res<Wall>, 
    mut next_state: ResMut<NextState<TurnState>>,
    mut commands: Commands,
) {
    if wall.0.is_empty() {
        if *oya_tenpai_query {
            commands.insert_resource(RoundResult(RoundEndReason::RyuukyokuOyaTenpai));
        } else {
            commands.insert_resource(RoundResult(RoundEndReason::RyuukyokuOyaNoten))
        }
        next_state.set(TurnState::RoundEnd);
    }
}

fn is_furiten(kawa: &Kawa, tenpai: &Tenpai) -> bool {
    tenpai.0.iter().any(|wait| kawa.0.contains(wait))
}


fn get_dora_from_indicator(indicator: &Tile) -> Tile {
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


fn count_dora(combined_hand: &[Tile], dead_wall: &DeadWall, is_riichi: bool) -> u8 {
    let mut additional_han = 0;
    for tile in combined_hand.iter() {
        for dora in dead_wall.dora_indicators.iter() {
            if *tile == get_dora_from_indicator(dora) {
                additional_han += 1 
            }
        }
        if is_riichi {
            for ura in dead_wall.ura_indicators.iter() {
                if *tile == get_dora_from_indicator(ura) {
                    additional_han += 1 
                }
            }
        }
    }
    additional_han
}


fn calculate_score(han: u8, fu: u8, is_oya: bool, is_tsumo: bool, is_yakuman: bool, yaku_names: &[String]) -> ScorePayout {
    if is_yakuman {
        if is_oya { 
            if is_tsumo {
                return ScorePayout {
                    total_won: (yaku_names.len() * 48000) as u32,
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
                    oya_pays: (yaku_names.len() * 32000) as u32,
                    non_oya_pays: (yaku_names.len() * 32000) as u32,
                }; 
            }
           
        } 
    }

    // https://riichi.wiki/Japanese_mahjong_scoring_rules
    let mut base: u32 = fu as u32 * 2_u32.pow((han + 2).into());

    if base > 2000 || han > 5 || (han >= 4 && fu >= 40) || (han >= 3 && fu >= 70) {
        match han {
            3..=5 => base = 2000,
            6..=7 => base = 3000,
            8..=9 => base = 4000,
            10..=12 => base = 6000,
            _ => base = 8000,
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


fn evaluate_yaku(
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
        total_han: 0,
        total_fu: 0,
        is_yakuman: false,
    };

    // path 1
    if is_hand_closed && kokushi_musou(raw_hand) {
        let mut eval = HandResult {
            yaku_names: vec!["Kokushi Musou".to_string()],
            total_han: 0, total_fu: 0, is_yakuman: true,
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
        best.total_han += count_dora(combined_hand, dead_wall, is_riichi)
    }

    best
}


fn evaluate_standard(
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
        total_han: 0, total_fu: 0, is_yakuman: false,
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

    if junchan(result) {
        eval.yaku_names.push("Junchan".to_string());
        eval.total_han += if is_hand_closed { 3 } else { 2 };
    } else if chanta(result) {
        eval.yaku_names.push("Chanta".to_string());
        eval.total_han += if is_hand_closed { 2 } else { 1 };
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

    if honroutou(combined_hand) {
        eval.yaku_names.push("Honroutou".to_string());
        eval.total_han += 2;
    }

    if sankantsu(open_mentsu) {
        eval.yaku_names.push("Sankantsu".to_string());
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


fn evaluate_chiitoitsu(
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
        total_han: 2,
        total_fu: 25, // always fixed
        is_yakuman: false,
    };

    // yakuman
    if tsuuisou(raw_hand) {
        eval.yaku_names.clear();
        eval.yaku_names.push("Tsuuiisou".to_string());
        eval.is_yakuman = true;
        add_situational_yakuman(&mut eval, kawa, is_oya, is_tsumo, calls_made);
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


fn add_situational_yakuman(eval: &mut HandResult, kawa: &Kawa, is_oya: bool, is_tsumo: bool, calls_made: bool) {
    
    if tenhou(kawa, is_oya, is_tsumo, calls_made) {
        eval.yaku_names.push("Tenhou".to_string());
    }

    if chiihou(kawa, is_oya, is_tsumo, calls_made) {
        eval.yaku_names.push("Chiihou".to_string());
    }

}


fn add_situational(
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


fn is_better(new: &HandResult, old: &HandResult) -> bool {
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


// call on opponent discard
// こんな引数を見せられたら誰でも泣きたくなるんだよなぁ
fn can_declare_ron(
    discard_tile: &Tile,
    hand: &[Tile],
    open_mentsu: &[Mentsu],
    tenpai: &Tenpai,
    is_hand_closed: bool,
    is_oya: bool,
    kawa: &Kawa,
    is_riichi: bool,
    is_double_riichi: bool,
    is_ippatsu: bool,
    bakaze: &Wind,
    jikaze: &Wind,
    wall: &Wall,
    dead_wall: &DeadWall,
    is_chankan: bool,
    calls_made: bool,
) -> Option<HandResult> {
    if !tenpai.0.contains(discard_tile) || is_furiten(kawa, tenpai) {
        return None;
    }

    let mut combined_hand = hand.to_owned();
    combined_hand.push(*discard_tile);
    for mentsu in open_mentsu {
        match mentsu {
            Mentsu::Koutsu(tiles, _) | Mentsu::Shuntsu(tiles, _)
            | Mentsu::Ankan(tiles) | Mentsu::Daiminkan(tiles)
            | Mentsu::Shouminkan(tiles) => combined_hand.extend(tiles),
            _ => {}
        }
    }
    combined_hand.sort();

    let mut raw_hand_plus_win = hand.to_owned();
    raw_hand_plus_win.push(*discard_tile);
    raw_hand_plus_win.sort();
    let mut results = decompose(&raw_hand_plus_win);
    for result in &mut results {
        for mentsu in open_mentsu {
            result.push(mentsu.to_owned());
        }
    }

    // yaku validation
    let yaku_result = evaluate_yaku(
        &results,
        hand,
        &raw_hand_plus_win,   // this shouldn't be raw hand only
        &combined_hand,      // combined hand
        open_mentsu,
        is_hand_closed,
        is_oya,
        is_riichi,
        is_double_riichi,
        is_ippatsu,
        bakaze,
        jikaze,
        kawa,                             // for tenhou/chiihou
        discard_tile,
        false,                 // is_tsumo, ron is never tsumo
        false,              // is_rinshan, ron is never rinshan
        is_chankan,          
        wall,
        dead_wall,
        calls_made);
        
    if yaku_result.yaku_names.is_empty() {
        None
    } else {
        Some(yaku_result)
    } 

}


// TODO: multiple ron
// runs once upon entering CallWindow 
fn ron_check(
    query: Query<(Entity, &Hand, &OpenMentsu, &Tenpai, &Kawa, &Jikaze, Has<ClosedHand>, Has<Oya>, Option<&Riichi>,)>,
    discard_query: Query<(&DiscardedTile, &DiscardedBy, Has<Chankan>), With<CurrentDiscard>>,
    game: Res<GameState>,
    wall: Res<Wall>,
    dead_wall: Res<DeadWall>,
    mut commands: Commands,
) {
    let Ok((discarded_tile, discarded_by, is_chankan)) = discard_query.single() else {
        return;
    };

    for (player, hand, open_mentsu, tenpai, kawa, jikaze,
         is_closed, is_oya, maybe_riichi) in &query
    {
        if player == discarded_by.0 { continue; }

        let is_riichi = maybe_riichi.is_some();
        let is_double = maybe_riichi.is_some_and(|r| r.is_double);
        let is_ippatsu = maybe_riichi.is_some_and(|r| r.is_ippatsu_alive);

        if let Some(result) = can_declare_ron(
            &discarded_tile.0, 
            &hand.0, 
            &open_mentsu.0, 
            tenpai,
            is_closed, 
            is_oya, 
            kawa,
            is_riichi, 
            is_double, 
            is_ippatsu,
            &game.bakaze, 
            &jikaze.0,
            &*wall, 
            &*dead_wall,
            is_chankan, 
            game.calls_made,
        ) {
            commands.entity(player).insert(RonOption {
                discarded_by: discarded_by.0,
                result,
            });
        }
    }
}


// reads RonOption, shows button, sends message on click
// TODO:  the actual ui
fn ron_ui_system(
    query: Query<(Entity, &RonOption)>,
    mut messages: MessageWriter<DeclareRonMessage>,
    // ! + ui input stuff later
) {
    for (entity, ron_option) in &query {
        // TODO: show ron button
        // on click:
        let clicked = false; // !placeholder
        if clicked {
            messages.write(DeclareRonMessage {
                player: entity,
                discarded_by: ron_option.discarded_by,
                result: ron_option.result.clone(),
            });
        }
    }
}


// move the ron check into new functions
fn declare_ron(
    mut messages: MessageReader<DeclareRonMessage>,
    oya_query: Query<Has<Oya>>,
    mut points_query: Query<&mut Points>,
    mut game: ResMut<GameState>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut commands: Commands,
) {
    for message in messages.read() {
        let is_oya = oya_query.get(message.player).unwrap_or(false);

        let score = calculate_score(
            message.result.total_han,
            message.result.total_fu,
            is_oya,
            false,
            message.result.is_yakuman,
            &message.result.yaku_names,
        );

        if let Ok([mut winner_points, mut loser_points]) =
            points_query.get_many_mut([message.player, message.discarded_by])
        {
            winner_points.0 += score.total_won as i32;
            loser_points.0 -= score.total_won as i32;

            winner_points.0 += game.riichi_points as i32;
            game.riichi_points = 0;

            if is_oya {
                commands.insert_resource(RoundResult(RoundEndReason::OyaWin));
            } else {
                commands.insert_resource(RoundResult(RoundEndReason::NonOyaWin));
            }

            next_state.set(TurnState::RoundEnd);
        }
    }
}


// cleanup so player doesn't prepetually qualify for ron
fn cleanup_call_options(
    query: Query<Entity, With<RonOption>>,
    mut commands: Commands,
) {
    for entity in &query {
        commands.entity(entity).remove::<RonOption>();
    }
}


// call on self draw
fn can_declare_tsumo(
    drawn_tile: &Tile,
    hand: &[Tile],
    open_mentsu: &[Mentsu],
    tenpai: &Tenpai,
    is_hand_closed: bool,
    is_oya: bool,
    kawa: &Kawa,
    is_riichi: bool,
    is_double_riichi: bool,
    is_ippatsu: bool,
    bakaze: &Wind,
    jikaze: &Wind,
    wall: &Wall,
    dead_wall: &DeadWall,
    is_rinshan: bool,
    calls_made: bool,
) -> Option<HandResult> {
    if !tenpai.0.contains(drawn_tile)  {
        return None;
    }

    let mut combined_hand = hand.to_owned();
    combined_hand.push(*drawn_tile);
    for mentsu in open_mentsu {
        match mentsu {
            Mentsu::Koutsu(tiles, _) | Mentsu::Shuntsu(tiles, _)
            | Mentsu::Ankan(tiles) | Mentsu::Daiminkan(tiles)
            | Mentsu::Shouminkan(tiles) => combined_hand.extend(tiles),
            _ => {}
        }
    }
    combined_hand.sort();

    let mut raw_hand_plus_win = hand.to_owned();
    raw_hand_plus_win.push(*drawn_tile);
    raw_hand_plus_win.sort();
    let mut results = decompose(&raw_hand_plus_win);
    for result in &mut results {
        for mentsu in open_mentsu {
            result.push(mentsu.to_owned());
        }
    }

    // yaku validation
    let yaku_result = evaluate_yaku(
        &results,
        hand,
        &raw_hand_plus_win,            
        &combined_hand,      
        open_mentsu,
        is_hand_closed,
        is_oya,
        is_riichi,
        is_double_riichi,
        is_ippatsu,
        bakaze,
        jikaze,
        kawa,                             // for tenhou/chiihou
        drawn_tile,
        true,                
        is_rinshan,              
        false,          // is_chankan, tsumo can't chankan
        wall,
        dead_wall,
        calls_made);
    
    if yaku_result.yaku_names.is_empty() {
        None
    } else {
        Some(yaku_result)
    }

}

// refer to ron counterpart
fn tsumo_check(
    current_turn: Res<CurrentTurn>,
    query: Query<(&Hand, &OpenMentsu, &Tenpai, &Kawa, &Jikaze, &DrawnTile, Has<ClosedHand>, Has<Oya>, Option<&Riichi>)>,
    game: Res<GameState>,
    wall: Res<Wall>,
    dead_wall: Res<DeadWall>,
    mut commands: Commands,
) {
    if let Ok((hand, open_mentsu, tenpai, kawa, jikaze, drawn,
              is_closed, is_oya, maybe_riichi)) = query.get(current_turn.0)
    {
        let is_riichi = maybe_riichi.is_some();
        let is_double = maybe_riichi.is_some_and(|r| r.is_double);
        let is_ippatsu = maybe_riichi.is_some_and(|r| r.is_ippatsu_alive);

        if let Some(result) = can_declare_tsumo(
            &drawn.0, 
            &hand.0, 
            &open_mentsu.0, 
            tenpai,
            is_closed, 
            is_oya, 
            kawa,
            is_riichi, 
            is_double, 
            is_ippatsu,
            &game.bakaze, 
            &jikaze.0,
            &*wall, 
            &*dead_wall,
            game.pending_rinshan, 
            game.calls_made,
        ) {
            commands.entity(current_turn.0).insert(TsumoOption { result });
        }
    }
}


fn tsumo_ui_system(
    query: Query<(Entity, &TsumoOption)>,
    mut messages: MessageWriter<DeclareTsumoMessage>,
    // ! + ui input stuff
) {
    for (entity, tsumo_option) in &query {
        // TODO: show tsumo button
        let clicked = false; // !placeholder
        if clicked {
            messages.write(DeclareTsumoMessage {
                player: entity,
                result: tsumo_option.result.clone(),
            });
        }
    }
}


fn declare_tsumo(
    mut messages: MessageReader<DeclareTsumoMessage>,
    oya_query: Query<Has<Oya>>,
    mut points_query: Query<(Entity, &mut Points, Has<Oya>)>,
    mut game: ResMut<GameState>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut commands: Commands,
) {
    for message in messages.read() {
        let is_oya = oya_query.get(message.player).unwrap_or(false);

        let score = calculate_score(
            message.result.total_han,
            message.result.total_fu,
            is_oya,
            true,
            message.result.is_yakuman,
            &message.result.yaku_names,
        );

        for (player, mut player_points, is_dealer) in points_query.iter_mut() {
            if player != message.player {
                if is_dealer {
                    player_points.0 -= score.oya_pays as i32;
                } else {
                    player_points.0 -= score.non_oya_pays as i32;
                }
            } else {
                player_points.0 += score.total_won as i32;
                player_points.0 += game.riichi_points as i32;
                game.riichi_points = 0;
            }
        }

        if is_oya {
            commands.insert_resource(RoundResult(RoundEndReason::OyaWin));
        } else {
            commands.insert_resource(RoundResult(RoundEndReason::NonOyaWin));
        }

        next_state.set(TurnState::RoundEnd);
    }
}


// cleanup so player doesn't prepetually qualify for tsumo
fn cleanup_main_phase_options(
    query: Query<Entity, With<TsumoOption>>,
    mut commands: Commands,
) {
    for entity in &query {
        commands.entity(entity).remove::<TsumoOption>();
    }
}


// raw hand nomi works as well, no need to combine with open mentsu!
fn check_tenpai(raw_hand: &[Tile]) -> Vec<Tile> { 
    let mut waiting_on: Vec<Tile> = vec![];
    for tile in all_tiles() {
        let mut hand_speculated = raw_hand.to_owned();
        hand_speculated.push(tile);
        hand_speculated.sort(); 
         if !decompose(&hand_speculated).is_empty() || chiitoitsu(&hand_speculated) || kokushi_musou(&hand_speculated) {
            waiting_on.push(tile);
        }
    }
    waiting_on
}

fn set_tenpai(
    query: Query<(Entity, &Hand)>,
    mut commands: Commands,
) {
    for (entity, hand) in &query {
        let waiting_on = check_tenpai(&hand.0);
        if !waiting_on.is_empty() {
            commands.entity(entity).insert(Tenpai(waiting_on));
        } else {
            commands.entity(entity).remove::<Tenpai>();
        }
    }
}

fn combine_tiles(hand: &Hand, open_mentsu: &OpenMentsu) -> Vec<Tile> {
    let mut result = hand.0.clone();

    for mentsu in &open_mentsu.0{
        if let 
            Mentsu::Koutsu(tiles, _) 
                | Mentsu::Shuntsu(tiles, _) 
                | Mentsu::Ankan(tiles) 
                | Mentsu::Daiminkan(tiles) 
                | Mentsu::Shouminkan(tiles) = mentsu {
                result.extend(tiles)
            }
    };
    result
}

fn tenpai_payout_system(mut query: Query<(&mut Points, Has<Tenpai>)>) {
    let tenpai_count = query.iter().filter(|(_, is_tenpai)| *is_tenpai).count();

    for (mut player_points, is_tenpai) in query.iter_mut() {
        match tenpai_count {
            1 => if is_tenpai { player_points.0 += 3000 } else { player_points.0 -= 1000 },
            2 => if is_tenpai { player_points.0 += 1500 } else { player_points.0 -= 1500 },
            3 => if is_tenpai { player_points.0 += 1000 } else { player_points.0 -= 3000 },
            _ => {}
        }
    }
}


fn can_declare_kyuushu(hand: &[Tile], calls_made: bool, kawa: &Kawa) -> bool {
    let mut yaochuuhai: Vec<&Tile> = hand.iter().filter(|x| is_yaochuuhai(x)).collect();
    yaochuuhai.sort();
    yaochuuhai.dedup();
    yaochuuhai.len() >= 9 && !calls_made && kawa.0.is_empty()
}

fn declare_kyuushu(
    mut messages: MessageReader<DeclareKyuushuMessage>,
    query: Query<(&Hand, &Kawa, &DrawnTile)>,
    game: Res<GameState>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>
) {
    for message in messages.read() {

        if let Ok((hand, kawa, drawn)) = query.get(message.player) {
            let mut combined = hand.0.to_owned();
            combined.push(drawn.0.to_owned());
            if can_declare_kyuushu(&combined, game.calls_made, kawa) {
                commands.insert_resource(RoundResult(RoundEndReason::TochuuRyuukyoku));
                next_state.set(TurnState::RoundEnd);
            }
        }
    }
}


fn suufon_renda(
    game: Res<GameState>,
    query: Query<&Kawa>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>
) {
    if !game.calls_made {
        let four_kawa: Vec<&Kawa> = query.iter().collect();

        if four_kawa.iter().all(|kawa| kawa.0.len() == 1) {
            let first_tile = four_kawa[0].0[0];
            if matches!(first_tile, Tile::Honor(Honor::East | Honor::South | Honor::West | Honor::North))
                && four_kawa.iter().all(|kawa| kawa.0[0] == first_tile)
            {
                commands.insert_resource(RoundResult(RoundEndReason::TochuuRyuukyoku));
                next_state.set(TurnState::RoundEnd);
            }
        }
    }
}


fn suucha_riichi(
    query: Query<&Riichi>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>
) {
    if query.count() == 4 {
        commands.insert_resource(RoundResult(RoundEndReason::TochuuRyuukyoku));
        next_state.set(TurnState::RoundEnd);
    }
}


fn suukaikan(
    query: Query<&OpenMentsu>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    let mut total_kan = 0;
    let mut players_with_kan = 0;

    for open in query.iter() {
        let kan = open.0.iter()
            .filter(|mentsu| matches!(mentsu, Mentsu::Ankan(_) | Mentsu::Daiminkan(_) | Mentsu::Shouminkan(_)))
            .count();

        if kan > 0 {
            players_with_kan += 1;
        }
        total_kan += kan;
    }

    if total_kan >= 4 && players_with_kan > 1 {
        commands.insert_resource(RoundResult(RoundEndReason::TochuuRyuukyoku));
        next_state.set(TurnState::RoundEnd);
    }
}

// TODO: Sanchahou


fn can_declare_riichi(hand: &[Tile], is_closed: bool, is_riichi: bool, points: i32, wall: &Wall) -> bool {
    !is_riichi
        && is_closed
        && points >= 1000
        && wall.0.len() >= 4
        && !check_tenpai(hand).is_empty()
}


fn declare_riichi(
    mut messages: MessageReader<DeclareRiichiMessage>, // store the entity id
    mut query: Query<(Has<ClosedHand>, Has<Riichi>, &Hand, &mut Points, &Kawa)>, // store the data
    wall: Res<Wall>,
    mut game: ResMut<GameState>,
    mut commands: Commands,
) {
    for message in messages.read() {
        if let Ok((is_closed, is_riichi, hand, mut points, kawa)) = query.get_mut(message.player) 
            && can_declare_riichi(&hand.0, is_closed, is_riichi, points.0, &*wall) {
                let is_double = kawa.0.is_empty() && !game.calls_made;
                commands.entity(message.player).insert(Riichi { 
                    is_double, 
                    is_ippatsu_alive: true, 
                    turns_since: 0 });
                points.0 -= 1000;
                game.riichi_points += 1000;
            }
    }
}


impl Hand {
    fn remove_tile_from_hand(&mut self, target: &Tile) {
        if let Some(idx) = self.0.iter().position(|x| x == target) {
            self.0.remove(idx);
        }
    }
}


fn can_declare_pon(hand: &[Tile], tile: &Tile,) -> bool {
    hand.iter().filter(|x| **x == *tile).count() >= 2
}

fn declare_pon(
    mut messages: MessageReader<DeclarePonMessage>,
    mut query: Query<(&mut Hand, &mut OpenMentsu, Option<&mut Riichi>)>,
    mut tile_query: Single<Entity, With<CurrentDiscard>>,
    mut game: ResMut<GameState>,
    mut current_turn: ResMut<CurrentTurn>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut timer: ResMut<CallWindowTimer>,
    mut commands: Commands,
) {
    for message in messages.read(){
        if let Ok((mut hand, mut open_mentsu, _)) = query.get_mut(message.player) 
            && can_declare_pon(&hand.0 ,&message.tile) { 
                open_mentsu.0.push(Mentsu::Koutsu(vec![message.tile; 3], false));
                for _ in 0..2 {
                    let idx = hand.0.iter().position(|x| *x == message.tile).unwrap();
                    hand.0.remove(idx);
                }
                // this does compile because of non-lexical lifetimes
                // same with chi and kan
                for (_, _, mut maybe_riichi) in query.iter_mut() {
                    if let Some(riichi) = maybe_riichi.as_deref_mut() {
                        riichi.is_ippatsu_alive = false;
                    }
                }
                commands.entity(message.player).remove::<ClosedHand>();
                commands.entity(*tile_query).despawn(); 
                game.calls_made = true;
                current_turn.0 = message.player;
                next_state.set(TurnState::MainPhase);
                timer.0.reset();
        }
    }
}


fn can_declare_chi(hand: &[Tile], tile: &Tile) -> Vec<ChiTilePos> {
    let mut results = vec![];

    // safe 'unwrap' with if let
    if let (Some(prev), Some(next)) = (previous_tile_sequence(tile), next_tile_sequence(tile))
        && hand.contains(&prev) && hand.contains(&next) {
            results.push(ChiTilePos::Middle);
    }

    if let Some(next) = next_tile_sequence(tile)
        && let Some(next_next) = next_tile_sequence(&next)
        && hand.contains(&next) && hand.contains(&next_next) {
            results.push(ChiTilePos::Left);
    }

    if let Some(prev) = previous_tile_sequence(tile)
        && let Some(prev_prev) = previous_tile_sequence(&prev)
        && hand.contains(&prev) && hand.contains(&prev_prev) {
            results.push(ChiTilePos::Right);
    }

    results
}


fn is_kamicha(self_wind: &Wind, discard_wind: &Wind) -> bool {
    matches!((self_wind, discard_wind), 
    (Wind::South, Wind::East) 
        | (Wind::West, Wind::South) 
        | (Wind::North, Wind::West) 
        | (Wind::East, Wind::North))
}

fn declare_chi(
    mut messages: MessageReader<DeclareChiMessage>,
    mut query: Query<(&mut Hand, &mut OpenMentsu, &Jikaze, Option<&mut Riichi>)>,
    mut tile_query: Single<Entity, With<CurrentDiscard>>,
    mut game: ResMut<GameState>,
    mut timer: ResMut<CallWindowTimer>,
    mut current_turn: ResMut<CurrentTurn>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut commands: Commands,
) {
    for message in messages.read() {
        let is_valid = if let (
            Ok((hand, _, self_jikaze, _)),
            Ok((_, _, discard_jikaze, _))
        ) = (
            query.get(message.player),
            query.get(message.discarded_by)
        ) {
            let positions = can_declare_chi(&hand.0, &message.tile);
            !positions.is_empty()
                && positions.contains(&message.pos)
                && is_kamicha(&self_jikaze.0, &discard_jikaze.0)
        } else {
            false
        };

        if is_valid && let Ok((mut hand, mut open_mentsu, _, _))= query.get_mut(message.player) {
            let pos: &ChiTilePos = &message.pos; // let the player choose 
            let tile = &message.tile;

            match pos {
                ChiTilePos::Middle => {
                    let next = next_tile_sequence(tile).unwrap();
                    let prev = previous_tile_sequence(tile).unwrap();
                    // use the variables as a pointer for removal first b4 moving the value 
                    hand.remove_tile_from_hand(&next);
                    hand.remove_tile_from_hand(&prev);
                    open_mentsu.0.push(Mentsu::Shuntsu(vec![prev, *tile, next], false));                         
                },
                ChiTilePos::Left => {
                    let next = next_tile_sequence(tile).unwrap();
                    let next_next = next_tile_sequence(&next).unwrap();
                    hand.remove_tile_from_hand(&next);
                    hand.remove_tile_from_hand(&next_next);
                    open_mentsu.0.push(Mentsu::Shuntsu(vec![*tile, next, next_next], false));
                },
                ChiTilePos::Right => {
                    let prev = previous_tile_sequence(tile).unwrap();
                    let prev_prev = previous_tile_sequence(&prev).unwrap();
                    hand.remove_tile_from_hand(&prev);
                    hand.remove_tile_from_hand(&prev_prev);
                    open_mentsu.0.push(Mentsu::Shuntsu(vec![prev_prev, prev, *tile], false));
                },
            }
            
            commands.entity(message.player).remove::<ClosedHand>();
            commands.entity(*tile_query).despawn(); 
            game.calls_made = true;
            for (_, _, _, mut maybe_riichi) in query.iter_mut() {
                if let Some(riichi) = maybe_riichi.as_deref_mut() {
                    riichi.is_ippatsu_alive = false;
                }
            }
            current_turn.0 = message.player;
            next_state.set(TurnState::MainPhase);
            timer.0.reset();
        }
    }
}

fn can_declare_kan_from_hand(hand: &[Tile], tile: &Tile) -> u8 {
    hand.iter().filter(|x| *x == tile).count() as u8
}

fn can_declare_kan_from_pon(open_mentsu: &[Mentsu], tile: &Tile) -> bool{
    open_mentsu.iter().any(|mentsu| {
        if let Mentsu::Koutsu(tiles, false) = mentsu && tiles[0] == *tile {
            true
        } else {false}
    }) 
}

#[derive(PartialEq)]
enum Kantsu {
    Ankan,
    Daiminkan,
    Shouminkan,
}


fn declare_kan(
    mut messages: MessageReader<DeclareKanMessage>,
    mut query: Query<(&mut Hand, &mut OpenMentsu, Option<&mut Riichi>)>,
    tile_query: Query<Entity, With<CurrentDiscard>>,
    mut game: ResMut<GameState>,
    mut dead_wall: ResMut<DeadWall>,
    mut timer: ResMut<CallWindowTimer>,
    mut current_turn: ResMut<CurrentTurn>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut commands: Commands
) { 
    for message in messages.read() {
        if let Ok((mut hand, mut open_mentsu, _)) = query.get_mut(message.player){
            let tile = &message.tile;
            let count = can_declare_kan_from_hand(&hand.0, tile);
            let mut kan_successful_type: Option<Kantsu> = None;

            if message.is_discard && count == 3 {
                open_mentsu.0.push(Mentsu::Daiminkan(vec![*tile; 4]));
                hand.0.retain(|x| x != tile);
                commands.entity(tile_query.single().unwrap()).despawn(); 
                commands.entity(message.player).remove::<ClosedHand>(); 
                kan_successful_type = Some(Kantsu::Daiminkan);
                game.pending_kan_dora = true;
            } 

            else if !message.is_discard && count == 4 {
                open_mentsu.0.push(Mentsu::Ankan(vec![*tile; 4]));
                // dora flipping timing 
                let new_dora = dead_wall.filler_tiles.remove(0);
                let new_ura =  dead_wall.filler_tiles.remove(0);
                dead_wall.dora_indicators.push(new_dora);
                dead_wall.ura_indicators.push(new_ura);
                hand.0.retain(|x| x != tile);
                kan_successful_type = Some(Kantsu::Ankan);
            }  

            else if !message.is_discard { // this check should be enough hopefully
                for mentsu in &mut open_mentsu.0 {
                    if let Mentsu::Koutsu(tiles, false) = mentsu && tiles[0] == *tile {
                        // deref to mutate
                        *mentsu = Mentsu::Shouminkan(vec![*tile; 4]);
                        hand.0.retain(|x| x != tile);
                        kan_successful_type = Some(Kantsu::Shouminkan);
                        game.pending_kan_dora = true;
                        game.pending_rinshan = true;
                        commands.spawn((
                            CurrentDiscard,
                            DiscardedTile(*tile),
                            DiscardedBy(message.player),
                            Chankan,
                        ));
                        break;
                    } 
                }
            }

            if kan_successful_type == Some(Kantsu::Ankan) || kan_successful_type == Some(Kantsu::Daiminkan)  {
                game.calls_made = true;
                for (_, _, mut maybe_riichi) in query.iter_mut() {
                    if let Some(riichi) = maybe_riichi.as_deref_mut() {
                        riichi.is_ippatsu_alive = false;
                    }
                }
                current_turn.0 = message.player;
                next_state.set(TurnState::RinshanDraw);
                timer.0.reset();
            } else if kan_successful_type == Some(Kantsu::Shouminkan) {
                 for (_, _, mut maybe_riichi) in query.iter_mut() {
                    if let Some(riichi) = maybe_riichi.as_deref_mut() {
                        riichi.is_ippatsu_alive = false;
                    }
                }
                current_turn.0 = message.player;
                next_state.set(TurnState::CallWindow);
                timer.0.reset();
            }
        }
    }
}


fn is_terminal(tile: &Tile) -> bool {
    matches!(tile, Tile::Sou(1 | 9) | Tile::Pin(1 | 9) | Tile::Man(1 | 9))
}


fn is_honor(tile: &Tile) -> bool {
    matches!(tile, Tile::Honor(_))
}


fn is_yaochuuhai(tile: &Tile) -> bool {
    is_terminal(tile) || is_honor(tile)
}

fn is_green(tile: &Tile) -> bool {
    matches!(tile, Tile::Sou(2 | 3 | 4 | 6 | 8) | Tile::Honor(Honor::Green))
}

fn ryuuiisou(hand: &[Tile]) -> bool {
    hand.iter().all(is_green)
}


fn tanyao(hand: &[Tile]) -> bool {
    hand.iter().all(|x| !is_yaochuuhai(x))    
}


fn kokushi_musou(hand: &[Tile]) -> bool {
    if hand.len() == 14 && hand.iter().all(is_yaochuuhai) {
        let mut pair_counter: u8 = 0;
        for i in 0..hand.len() - 1 {
            if hand[i] == hand[i+1] {
                pair_counter += 1;
            }
        }
        if pair_counter == 1 {
            return true;
        } 
        return false;
    }
    false
} 


fn tsuuisou(hand: &[Tile]) -> bool {
    hand.iter().all(is_honor)
}


fn iipeikou(result: &[Mentsu]) -> bool {
    let shuntsu: Vec<&Mentsu> = result.iter().filter(|x| matches!(x, Mentsu::Shuntsu(_, true))).collect();

    for i in 0..shuntsu.len() {
        for j in i+1..shuntsu.len() {
            if shuntsu[i] == shuntsu[j] {
                return true;
            }
        }
    }
    // for any()
    false
}


fn ryanpeikou(result: &[Mentsu]) -> bool {
            let mut shuntsu: Vec<&Mentsu> = result.iter().filter(|x| matches!(x, Mentsu::Shuntsu(_, true))).collect();

        if shuntsu.len() == 4 {
            shuntsu.sort();
            if shuntsu[0] == shuntsu[1] && shuntsu[2] == shuntsu[3] {
                return true;
            }
        }
        false
}


fn wind_to_honor(wind: &Wind) -> Honor {
    match wind {
        Wind::East => Honor::East,
        Wind::South => Honor::South,
        Wind::West => Honor::West,
        Wind::North => Honor::North,
    }
}


fn yakuhai(result: &[Mentsu], jikaze: &Wind, bakaze: &Wind) -> u8 {
    result.iter().map(|mentsu| {
        if let
            Mentsu::Koutsu(tiles, _)
                | Mentsu::Ankan(tiles)
                | Mentsu::Daiminkan(tiles)
                | Mentsu::Shouminkan(tiles) = mentsu
        {
            let tile = &tiles[0];
            let mut count = 0;
            
            if let Tile::Honor(Honor::Red | Honor::Green | Honor::White) = tile {
                count += 1;
            } 

            if let Tile::Honor(h) = tile {
                if *h == wind_to_honor(jikaze) { 
                    count += 1; 
                }
                if *h == wind_to_honor(bakaze) { 
                    count += 1; 
                }
            }
            count
        } else {
            0
        }
    }).sum()
}


fn sanankou(result: &[Mentsu], winning_tile: &Tile, is_tsumo: bool, thirteen_tiles: &[Tile]) -> bool {
    result
        .iter()
        .filter(|mentsu| {
            if let Mentsu::Koutsu(tiles, true) | Mentsu::Ankan(tiles) = mentsu {
                // compares result with thirteen tiles to see if the winning tile forms the final koutsu and doesn't come from ron 
                !(tiles[0] == *winning_tile && !is_tsumo && thirteen_tiles.iter().filter(|x| *x == winning_tile).count() == 2)
            } else {
                false
            }
        }).count() == 3
}


fn suuankou(result: &[Mentsu], winning_tile: &Tile, is_tsumo: bool) -> bool {
    result
        .iter()
        .filter(|mentsu|{
            if let Mentsu::Koutsu(tiles, true) | Mentsu::Ankan(tiles) = mentsu {
                tiles[0] != *winning_tile || is_tsumo
            } else {
                false
            }
        }).count() == 4 
}


fn toitoi(result: &[Mentsu]) -> bool {
    result
        .iter()
        .filter(|mentsu| 
            matches!(mentsu, Mentsu::Koutsu(_, _) | Mentsu::Ankan(_) | Mentsu::Daiminkan(_) | Mentsu::Shouminkan(_)))
        .count() == 4 
}


fn daisangen(result: &[Mentsu]) -> bool {
    has_koutsu_or_kan(result, Tile::Honor(Honor::Red))
        && has_koutsu_or_kan(result, Tile::Honor(Honor::Green)) 
        && has_koutsu_or_kan(result, Tile::Honor(Honor::White))
}


fn shousangen(result: &[Mentsu]) -> bool {
    let dragon_kou_or_kan = has_koutsu_or_kan(result, Tile::Honor(Honor::Red)) as u8
        + has_koutsu_or_kan(result, Tile::Honor(Honor::Green)) as u8
        + has_koutsu_or_kan(result, Tile::Honor(Honor::White)) as u8;

    let dragon_jantou = has_jantou(result, Tile::Honor(Honor::Red)) as u8 
        + has_jantou(result, Tile::Honor(Honor::Green)) as u8 
        + has_jantou(result, Tile::Honor(Honor::White)) as u8;

    dragon_kou_or_kan == 2 && dragon_jantou == 1
}


fn daisuushii(result: &[Mentsu]) -> bool {
    has_koutsu_or_kan(result, Tile::Honor(Honor::East))
        && has_koutsu_or_kan(result, Tile::Honor(Honor::South)) 
        && has_koutsu_or_kan(result, Tile::Honor(Honor::West))
        && has_koutsu_or_kan(result, Tile::Honor(Honor::North))
}

fn shousuushii(result: &[Mentsu]) -> bool {
    let wind_kou_or_kan = has_koutsu_or_kan(result, Tile::Honor(Honor::East)) as u8
        + has_koutsu_or_kan(result, Tile::Honor(Honor::South)) as u8
        + has_koutsu_or_kan(result, Tile::Honor(Honor::West)) as u8
        + has_koutsu_or_kan(result, Tile::Honor(Honor::North)) as u8;
    
    let wind_jantou = has_jantou(result, Tile::Honor(Honor::East)) as u8 
        + has_jantou(result, Tile::Honor(Honor::South)) as u8 
        + has_jantou(result, Tile::Honor(Honor::West)) as u8 
        + has_jantou(result, Tile::Honor(Honor::North)) as u8;
    
        wind_kou_or_kan == 3 && wind_jantou == 1
}


fn chinitsu(hand: &[Tile]) -> bool {
    hand.iter().all(|x| matches!(x, Tile::Man(_)))
        || hand.iter().all(|x| matches!(x, Tile::Pin(_))) 
        || hand.iter().all(|x| matches!(x, Tile::Sou(_))) 
}


fn honitsu(hand: &[Tile]) -> bool {
    hand.iter().all(|x| matches!(x, Tile::Man(_)) || is_honor(x)) 
        || hand.iter().all(|x| matches!(x, Tile::Pin(_)) || is_honor(x)) 
        || hand.iter().all(|x| matches!(x, Tile::Sou(_)) || is_honor(x)) 
}


fn chanta(result: &[Mentsu]) -> bool {
    result.iter().all(|mentsu| {
        match mentsu {
            Mentsu::Shuntsu(tiles, _) => {
                is_terminal(&tiles[0]) || is_terminal(&tiles[2])
            }
            Mentsu::Koutsu(tiles, _)  
                | Mentsu::Jantou(tiles) 
                | Mentsu::Ankan(tiles)  
                | Mentsu::Daiminkan(tiles) 
                | Mentsu::Shouminkan(tiles)  => {
                is_yaochuuhai(&tiles[0])
            }
        }
    })
}


fn junchan(result: &[Mentsu]) -> bool {
    result.iter().all(|mentsu| {
        match mentsu {
            Mentsu::Shuntsu(tiles, _) => {
                is_terminal(&tiles[0]) || is_terminal(&tiles[2])
            }
            Mentsu::Koutsu(tiles, _)  
                | Mentsu::Jantou(tiles) 
                | Mentsu::Ankan(tiles)  
                | Mentsu::Daiminkan(tiles) 
                | Mentsu::Shouminkan(tiles)  => {
                is_terminal(&tiles[0])
            }
        }
    })
}


fn sankantsu(open_mentsu: &[Mentsu]) -> bool {
    open_mentsu.iter().filter(|mentsu|
        matches!(mentsu, Mentsu::Ankan(_) | Mentsu::Daiminkan(_) | Mentsu::Shouminkan(_))).count() == 3 
}


fn suukantsu(open_mentsu: &[Mentsu]) -> bool {
    open_mentsu.iter().filter(|mentsu|
        matches!(mentsu, Mentsu::Ankan(_) | Mentsu::Daiminkan(_) | Mentsu::Shouminkan(_))).count() == 4 
}


fn chiitoitsu(hand: &[Tile]) -> bool {
    if hand.len() != 14 {
        return false;
    }
    let mut i = 0;
    let mut seen = vec![];
    while i < hand.len() - 1 {
        if hand[i] != hand[i + 1] {
            return false;
        }
        if seen.contains(&hand[i]) { 
            return false; 
        } 
        seen.push(hand[i]);
        i += 2;
    }
    true
} 


fn honroutou(hand: &[Tile]) -> bool {
    hand.iter().all(is_yaochuuhai)
}


fn chinroutou(hand: &[Tile]) -> bool {
    hand.iter().all(is_terminal)
}


fn has_shuntsu(result: &[Mentsu], first_tile: Tile) -> bool {
    result.iter().any(|mentsu|{
        if let Mentsu::Shuntsu(tiles, _) = mentsu {
            tiles[0] == first_tile 
        } else {
            false
        }
    })
}


fn ittsuu(result: &[Mentsu]) -> bool {
    let man: bool = has_shuntsu(result, Tile::Man(1))
        && has_shuntsu(result, Tile::Man(4))
        && has_shuntsu(result, Tile::Man(7));
    let pin: bool = has_shuntsu(result, Tile::Pin(1))
        && has_shuntsu(result, Tile::Pin(4)) 
        && has_shuntsu(result, Tile::Pin(7));
    let sou: bool = has_shuntsu(result, Tile::Sou(1)) 
        && has_shuntsu(result, Tile::Sou(4)) 
        && has_shuntsu(result, Tile::Sou(7));

    man || pin || sou
}


fn sanshoku_doujun(result: &[Mentsu]) -> bool {
    for i in 1..=7 {
        let num_match = has_shuntsu(result, Tile::Man(i))
            && has_shuntsu(result, Tile::Pin(i))
            && has_shuntsu(result, Tile::Sou(i));

        if num_match {
            return true;
        }
    } 
    false
}


fn has_koutsu_or_kan(result: &[Mentsu], first_tile: Tile) -> bool{
    result.iter().any(|mentsu|{
        if let Mentsu::Koutsu(tiles, _) 
            | Mentsu::Ankan(tiles) 
            | Mentsu::Daiminkan(tiles)
            | Mentsu::Shouminkan(tiles)  = mentsu {
            tiles[0] == first_tile 
        } else {
            false
        }
    })
}

fn has_jantou(result: &[Mentsu], target_tile: Tile) -> bool {
    result.iter().any(|mentsu| {
        if let Mentsu::Jantou(tiles) = mentsu {
            tiles[0] == target_tile
        } else {
            false
        }
    })
}


fn sanshoku_doukou(result: &[Mentsu]) -> bool {
    for i in 1..=9 {
        let color_match =  has_koutsu_or_kan(result, Tile::Man(i))
            && has_koutsu_or_kan(result, Tile::Pin(i))
            && has_koutsu_or_kan(result, Tile::Sou(i));
        
        if color_match {
            return true;
        }
    }
    false
}


fn chuuren_poutou(combined_hand: &[Tile]) -> bool {
    if (combined_hand.len() == 14) 
    && (combined_hand.iter().all(|x| matches!(x, Tile::Man(_)))
        || combined_hand.iter().all(|x| matches!(x, Tile::Pin(_)))
        || combined_hand.iter().all(|x| matches!(x, Tile::Sou(_)))) {
            for i in 1..=9 {
                if !combined_hand.contains(&Tile::Man(i)) && !combined_hand.contains(&Tile::Pin(i)) && !combined_hand.contains(&Tile::Sou(i)) {
                    return false;
                }
            }
            combined_hand.iter().filter(|x| matches!(x, Tile::Man(1) | Tile::Pin(1) | Tile::Sou(1))).count() >= 3 
                && combined_hand.iter().filter(|x| matches!(x, Tile::Man(9) | Tile::Pin(9) | Tile::Sou(9))).count() >= 3 
    } else {
        false
    }
}


fn pinfu(result: &[Mentsu], winning_tile: &Tile, jikaze: &Wind, bakaze: &Wind) -> bool {
    let mut shuntsu_count = 0;
    let mut has_ryanmen = false;
    let mut has_valid_jantou = false;

    for mentsu in result {
        
        match mentsu {
            Mentsu::Shuntsu(tiles, true) => {
                shuntsu_count += 1;
                // ? this is right there's no way this is wrong please tell me this is enough
                // https://riichi.wiki/Pinfu
                // https://riichi.wiki/Complex_waits
                if is_ryanmen_wait(tiles, winning_tile) { 
                    has_ryanmen = true;
                }
            }
            Mentsu::Jantou(tiles) => {
                has_valid_jantou = match tiles[0] {
                    Tile::Honor(Honor::Red | Honor::Green | Honor::White) => false,
                    Tile::Honor(h) if h == wind_to_honor(jikaze) => false,
                    Tile::Honor(h) if h == wind_to_honor(bakaze) => false,
                    _ => true,
                };
            }
            _ => {}
        }
    }
    shuntsu_count == 4 && has_ryanmen && has_valid_jantou
}


fn is_ryanmen_wait(shuntsu_tiles: &[Tile], winning_tile: &Tile) -> bool {
    if shuntsu_tiles[0] == *winning_tile {
        // left machi (accepts 1/4)
        !matches!(winning_tile, Tile::Man(7) | Tile::Pin(7) | Tile::Sou(7))
    } else if shuntsu_tiles[2] == *winning_tile {
        // right machi (accepts 6/9)
        !matches!(winning_tile, Tile::Man(3) | Tile::Pin(3) | Tile::Sou(3))
    } else {
        false
    }
}

fn is_penchan_wait(result: &[Mentsu], winning_tile: &Tile) -> bool {
    result.iter().any(|mentsu| {
    if let Mentsu::Shuntsu(tiles, _) = mentsu {
        (tiles[0] == *winning_tile && is_terminal(&tiles[2])) || // 7 machi
        (tiles[2] == *winning_tile && is_terminal(&tiles[0]))    // 3 machi
    } else {
        false
    }
    })
}

fn is_kanchan_wait(result: &[Mentsu], winning_tile: &Tile) -> bool {
    result.iter().any(|mentsu| matches!(mentsu, Mentsu::Shuntsu(tiles, _) if tiles[1] == *winning_tile)) // 2 5 8
}

fn is_tanki_wait(result: &[Mentsu], winning_tile: &Tile) -> bool {
    result.iter().any(|mentsu| matches!(mentsu, Mentsu::Jantou(tiles) if tiles[0] == *winning_tile))
}


fn calculate_fu(
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
                    if *h == wind_to_honor(jikaze) { 
                        fu += 2; 
                    }
                    if *h == wind_to_honor(bakaze) { 
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


fn haitei(wall: &Wall, is_tsumo: bool) -> bool {
    wall.0.is_empty() && is_tsumo
}

fn houtei(wall: &Wall, is_tsumo: bool) -> bool {
    wall.0.is_empty() && !is_tsumo
}

fn tenhou(kawa: &Kawa, is_oya: bool, is_tsumo: bool, calls_made: bool) -> bool {
    kawa.0.is_empty() && is_oya && is_tsumo && !calls_made
} 

fn chiihou(kawa: &Kawa, is_oya: bool, is_tsumo: bool, calls_made: bool) -> bool {
    kawa.0.is_empty() && !is_oya && is_tsumo && !calls_made
} 



fn all_tiles() -> Vec<Tile> {
    // will compare vec vs array later
    let mut tiles = vec![];
    for n in 1..=9 {
        tiles.push(Tile::Man(n));
        tiles.push(Tile::Pin(n));
        tiles.push(Tile::Sou(n));
    }
    tiles.push(Tile::Honor(Honor::East));
    tiles.push(Tile::Honor(Honor::South));
    tiles.push(Tile::Honor(Honor::West));
    tiles.push(Tile::Honor(Honor::North));
    tiles.push(Tile::Honor(Honor::White));
    tiles.push(Tile::Honor(Honor::Green));
    tiles.push(Tile::Honor(Honor::Red));
    tiles
}


// one hand can return different mentsu varations
// example: [sou1, sou1, sou1 sou2, sou2, sou2, sou3, sou3, sou3] 
// can return [shuntsu, shuntsu, shuntsu] or [koutsu, koutsu, koutsu]
// so the final result is a vector of those two
fn decompose(tiles: &[Tile]) -> Vec<Vec<Mentsu>> {
    let mut results = vec![];

    for i in 0..tiles.len() - 1 {
        if tiles[i] == tiles[i+1] {
            if i > 0 && tiles[i] == tiles[i-1]{
                continue;
            }
            let pair = Mentsu::Jantou(vec![tiles[i], tiles[i+1]]);
            let mut remaining = tiles.to_owned();
            // removes jantou from mentsu check
            remaining.remove(i + 1);
            remaining.remove(i);

            find_mentsu(&remaining, vec![pair], &mut results);
        }
    }
    results
}


fn next_tile_sequence(tile: &Tile) -> Option<Tile> {
    match tile {
        Tile::Man(n) if *n < 9 => Some(Tile::Man(n + 1)),
        Tile::Pin(n) if *n < 9 => Some(Tile::Pin(n + 1)),
        Tile::Sou(n) if *n < 9 => Some(Tile::Sou(n + 1)),
        _ => None, 
    }
}


fn previous_tile_sequence(tile: &Tile) -> Option<Tile> {
    match tile {
        Tile::Man(n) if *n > 1 => Some(Tile::Man(n - 1)),
        Tile::Pin(n) if *n > 1 => Some(Tile::Pin(n - 1)),
        Tile::Sou(n) if *n > 1 => Some(Tile::Sou(n - 1)),
        _ => None, 
    }
}


fn find_mentsu(remaining: &[Tile], current: Vec<Mentsu>, results: &mut Vec<Vec<Mentsu>>) {
    if remaining.is_empty() {
        results.push(current);
        return;
    }

    // koutsu check
    if remaining.len() >= 3 && remaining[0] == remaining[1] && remaining[0] == remaining[2] {
        let koutsu_group = Mentsu::Koutsu(vec![remaining[0], remaining[1], remaining[2]], true);
        let mut new_remaining = remaining.to_owned();
        for _ in 0..3 {
            new_remaining.remove(0);
        }
        let mut new_current = current.to_owned();
        new_current.push(koutsu_group);
        find_mentsu(&new_remaining, new_current, results);
    }

    // shuntsu check
    if let Some(second) = next_tile_sequence(&remaining[0]) 
        && let Some(third) = next_tile_sequence(&second)
        && let Some(second_seq) = remaining.iter().skip(1).position(|x| *x == second).map(|i| i + 1)
        && let Some(third_seq) = remaining.iter().skip(second_seq + 1).position(|x| *x == third).map(|i| i + second_seq + 1) {
            let shuntsu_group = Mentsu::Shuntsu(vec![remaining[0], remaining[second_seq], remaining[third_seq]], true);
            let mut new_remaining = remaining.to_owned();
            // starts from the highest index
            for idx in [third_seq, second_seq, 0] {
                new_remaining.remove(idx);
                }
            let mut new_current = current.clone();
            new_current.push(shuntsu_group);
            find_mentsu(&new_remaining, new_current, results);
    }
    
}


fn start_game(
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>
) {
    let mut wall = vec![];
    for _ in 0..4 {
        wall.extend(all_tiles());
    }
    wall.shuffle(&mut rand::rng());
    
    let seats = [Wind::East, Wind::South, Wind::West, Wind::North];
    let mut starting_player = Entity::PLACEHOLDER;

    commands.insert_resource(DeadWall {
        dora_indicators: wall.drain(..1).collect(),
        ura_indicators: wall.drain(..1).collect(),
        rinshan_tiles: wall.drain(..4).collect(),
        filler_tiles:wall.drain(..8).collect(),
    });

    for &wind in &seats {
        let starting_hand: Vec<Tile> = wall.drain(wall.len() - 13..).collect();
        let mut player = commands.spawn((
            PlayerTag, 
            Points(25000),
            Jikaze(wind),
            Hand(starting_hand),
            OpenMentsu(vec![]),
            Kawa(vec![]),
            Alive,
            ClosedHand,
        ));

         if wind == Wind::East {
            player.insert(Oya);
            starting_player = player.id();
        }

    }

    commands.insert_resource(
        GameState { 
            rounds: 1,  
            honba: 0,
            bakaze: Wind::East, 
            bullet: 1,
            calls_made: false,
            riichi_points: 0,
            pending_kan_dora: false,
            pending_rinshan: false,
        }
    );
    commands.insert_resource(CurrentTurn(starting_player));
    
    commands.insert_resource(Wall(wall));
    commands.insert_resource(CallWindowTimer(Timer::from_seconds(2.0, TimerMode::Once)));
    println!("ゲーム開始");
    next_state.set(TurnState::Draw);
}


fn draw_tile(
    current_turn: Res<CurrentTurn>,
    mut wall: ResMut<Wall>,
    mut query: Query<(Entity, Has<Furiten>, Has<Riichi>)>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>, // used to change the game phase
) {
    let drawn = wall.0.remove(0); 
    commands.entity(current_turn.0).insert(DrawnTile(drawn));

    if let Ok((player, _, is_riichi)) = query.get(current_turn.0) 
    && !is_riichi {
            commands.entity(player).remove::<Furiten>();
    }
    

    next_state.set(TurnState::MainPhase);

    println!("{} draws {:?}", current_turn.0, drawn);
}


fn rinshan_draw(
    current_turn: Res<CurrentTurn>,
    mut wall: ResMut<Wall>,
    mut dead_wall: ResMut<DeadWall>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    let drawn = dead_wall.rinshan_tiles.remove(0);

    commands.entity(current_turn.0).insert(DrawnTile(drawn));

    dead_wall.filler_tiles.push(wall.0.pop().unwrap());

    next_state.set(TurnState::MainPhase);
}


fn discard_tile(
    mut messages: MessageReader<DiscardTileMessage>,
    mut query: Query<(&mut Hand, &mut DrawnTile, &mut Kawa)>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
    mut game: ResMut<GameState>,
    mut dead_wall: ResMut<DeadWall>
) {
    for message in messages.read() {
        if let Ok((mut hand, drawn, mut kawa)) =  query.get_mut(message.player) {
            
            if !message.is_tsumogiri { // the opposite should do nothing
                hand.0.push(drawn.0);
                if let Some(idx) = hand.0.iter().position(|x| *x == message.tile) {
                    hand.0.remove(idx);
                }
                hand.0.sort();
            }

            kawa.0.push(message.tile);

            commands.entity(message.player).remove::<DrawnTile>();
            commands.spawn((
                CurrentDiscard,
                DiscardedTile(message.tile),
                DiscardedBy(message.player),
            ));

            if game.pending_kan_dora {
                let new_dora = dead_wall.filler_tiles.remove(0);
                let new_ura =  dead_wall.filler_tiles.remove(0);
                dead_wall.dora_indicators.push(new_dora);
                dead_wall.ura_indicators.push(new_ura);
                game.pending_kan_dora = false;
            }

            next_state.set(TurnState::CallWindow);

            println!("{} discards {:?}", message.player, message.tile);
        }
    }
}

fn next_turn_wind(current: &Wind) -> Wind {
    match current {
        Wind::East => Wind::South,
        Wind::South => Wind::West,
        Wind::West => Wind::North,
        Wind::North => Wind::East,
    }
}


fn next_turn(
    mut current_turn: ResMut<CurrentTurn>,
    mut query: Query<(Entity, &Jikaze)>,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    if let Ok((_, current_jikaze)) = query.get_mut(current_turn.0){
        let next_jikaze = next_turn_wind(&current_jikaze.0);
        for (player, jikaze) in query.iter() {
            if jikaze.0 == next_jikaze {
                current_turn.0 = player;
                next_state.set(TurnState::Draw);
                break;
            }
        }
    }
}


// for testing
fn auto_discard_bot(
    current_turn: Res<CurrentTurn>,
    query: Query<&DrawnTile>,
    mut messages: MessageWriter<DiscardTileMessage>,
) {
    if let Ok(drawn) = query.get(current_turn.0) {

        messages.write(DiscardTileMessage {
            player: current_turn.0,
            tile: drawn.0,
            is_tsumogiri: true,
        });
    }
}


fn call_window_timeout(
    time: Res<Time>, // built-in clock
    mut game: ResMut<GameState>,
    mut call_timer: ResMut<CallWindowTimer>,
    mut next_state: ResMut<NextState<TurnState>>,
    tile_query: Single<(Entity, &DiscardedTile), With<CurrentDiscard>>,
    furiten_check: Query<(Entity, &Tenpai)>,
    mut commands: Commands
) {
    // this shouldn't be a timer later, but a declare confirmation (no?)
    call_timer.0.tick(time.delta());

    if call_timer.0.just_finished() {
        let (discard_entity, discarded_tile) = *tile_query;

        for (player, tenpai) in furiten_check {
            if tenpai.0.contains(&discarded_tile.0) {
                commands.entity(player).insert(Furiten);
            }
        }

        commands.entity(discard_entity).despawn();

        if game.pending_rinshan {
            next_state.set(TurnState::RinshanDraw);
            game.pending_rinshan = false;
        } else {
            next_state.set(TurnState::AdvanceTurn);
        }
        call_timer.0.reset();
    }
}


fn start_round(
    query: Query<&mut Hand>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>
) {
    let mut wall = vec![];
    for _ in 0..4 {
        wall.extend(all_tiles());
    }
    wall.shuffle(&mut rand::rng());

    commands.insert_resource(DeadWall {
        dora_indicators: wall.drain(..1).collect(),
        ura_indicators: wall.drain(..1).collect(),
        rinshan_tiles: wall.drain(..4).collect(),
        filler_tiles:wall.drain(..8).collect(),
    });

    for mut hand in query {
        let starting_hand: Vec<Tile> = wall.drain(wall.len() - 13..).collect();
        hand.0 = starting_hand;
    }

    commands.insert_resource(Wall(wall));
    next_state.set(TurnState::Draw);
}

fn next_round_wind(jikaze: &Wind) -> Wind {
    match jikaze {
        Wind::East => Wind::North,
        Wind::South => Wind::East,
        Wind::West =>  Wind::South,
        Wind::North => Wind::West,
    }
}

fn round_cleanup(
    mut query: Query<(Entity, &mut Jikaze, Has<Oya>)>,
    tile_query: Query<Entity, With<DiscardedTile>>,
    player_query: Query<(Entity, &mut Hand, &mut OpenMentsu, &mut Kawa)>,
    result: Res<RoundResult>,
    mut game: ResMut<GameState>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    match result.0 {
        RoundEndReason::OyaWin | RoundEndReason::RyuukyokuOyaTenpai | RoundEndReason::TochuuRyuukyoku => {game.honba += 1},
        RoundEndReason::NonOyaWin | RoundEndReason::RyuukyokuOyaNoten => { 
            if game.bakaze == Wind::East && game.rounds == 4 {
                game.bakaze = Wind::South;
                game.rounds = 1;
            } else {
                game.rounds += 1;
            }

            for (player, mut jikaze, is_oya) in query.iter_mut() {
                jikaze.0 = next_round_wind(&jikaze.0);

                if is_oya {
                    commands.entity(player).remove::<Oya>();
                }
                if jikaze.0 == Wind::East {
                    commands.entity(player).insert(Oya);
                    commands.insert_resource(CurrentTurn(player));
                }
            }
         },
    }

    for tile in tile_query.iter() {
        commands.entity(tile).despawn();
    }

    for (player, mut hand, mut open_mentsu, mut kawa) in player_query {
        hand.0.clear();
        open_mentsu.0.clear();
        kawa.0.clear();

        commands.entity(player).remove::<Tenpai>();
        commands.entity(player).remove::<Riichi>();
        commands.entity(player).remove::<Furiten>();
        commands.entity(player).remove::<DrawnTile>();

        commands.entity(player).insert(ClosedHand);
    }
    game.calls_made = false;
    game.pending_kan_dora = false;
    game.pending_rinshan = false;

    commands.remove_resource::<RoundResult>();
    next_state.set(TurnState::StartNewRound);
}

fn main() {

    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<TurnState>()
        // messages
        .add_message::<DiscardTileMessage>()
        .add_message::<DeclarePonMessage>()
        .add_message::<DeclareChiMessage>()
        .add_message::<DeclareKanMessage>()
        .add_message::<DeclareRiichiMessage>()
        .add_message::<DeclareRonMessage>()
        .add_message::<DeclareTsumoMessage>()
        .add_message::<DeclareKyuushuMessage>()
        // setup (first round)
        .add_systems(OnEnter(TurnState::Setup), (
            start_game,
            set_tenpai,
        ).chain())
        // new round
        .add_systems(OnEnter(TurnState::StartNewRound), (
            start_round,
            set_tenpai,
        ).chain())
        // draw
        .add_systems(OnEnter(TurnState::Draw), draw_tile)
        // main phase
        .add_systems(OnEnter(TurnState::MainPhase), (
            check_ryuukyoku,
            tsumo_check,
        ).chain())
        .add_systems(Update, (
            tsumo_ui_system,
            declare_tsumo,
            auto_discard_bot,
            discard_tile,
        ).run_if(in_state(TurnState::MainPhase))
        .run_if(not(resource_exists::<RoundResult>)))
        .add_systems(OnExit(TurnState::MainPhase), cleanup_main_phase_options)
        // call window
        .add_systems(OnEnter(TurnState::CallWindow), (
            set_tenpai,
            ron_check,
        ).chain())
        .add_systems(Update, (
            ron_ui_system,
            declare_ron,
            call_window_timeout,
        ).run_if(in_state(TurnState::CallWindow)))
        .add_systems(OnExit(TurnState::CallWindow), cleanup_call_options)
        // advance & end
        .add_systems(OnEnter(TurnState::AdvanceTurn), next_turn)
        .add_systems(OnEnter(TurnState::RoundEnd), round_cleanup)
        .run();



    // TODO: logical sorting when player picks up a tile (or not?)
    
}