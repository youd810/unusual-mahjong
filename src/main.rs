// yaku todo
// ?note to self: kan check = open mentsu, unusual yaku check = hand, normal yaku check = decompose result  
// *Yakuman (Limit Hands)
// !Kokushi Musou (Thirteen Orphans) done
// !Suuankou (Four Concealed Triplets) done
// !Daisangen (Big Three Dragons) done
// !Shousuushii (Little Four Wind) done
// !Daisuushii (Big Four Wind) done 
// !Tsuuiisou (All Honors) done
// !Chinroutou (All Terminals) done
// !Ryuuiisou (All Green) done
// !Chuuren Poutou (Nine Gates) done
// !Suukantsu (Four Kans) done
// !Tenhou (Heavenly Win — dealer wins on first draw) done?
// !Chiihou (Earthly Win — non-dealer wins on first draw) done?
// 
// *6 Han
// !Chinitsu (Full Flush) done
// 
// *3 Han
// !Honitsu (Half Flush) done
// !Ryanpeikou (Two Sets of Identical Sequences) done
// !Junchan (All sets contain terminals) done
// 
// *2 Han
// !Chanta (All sets contain terminals or honors) done
// !Sanshoku Doujun (Three Color Straight) done 
// !Sanshoku Doukou (Three Color Triplets) done
// !Ittsu (Straight 1-9) done
// !Toitoi (All Triplets) done
// !Sanankou (Three Concealed Triplets) done
// !Shousangen (Little Three Dragons) done
// !Honroutou (All Terminals and Honors) done
// !Chiitoitsu (Seven Pairs) done
// !Sankantsu (Three Kans) done
// !Double Riichi done
// 
// *1 Han
// !Tanyao (All Simples) done
// !Iipeikou (One Set of Identical Sequences) done
// !Yakuhai / Fanpai (Value Tiles — seat Wind, round Wind, dragons) done
// !Riichi done
// !Ippatsu done
// !Menzen Tsumo (Self-draw win with closed hand) done
// !Pinfu (No-points hand) done
// !Haitei (Win on last tile from wall) done
// !Houtei (Win on last discard) done
// !Rinshan Kaihou (Win after Kan draw) done
// !Chankan (Robbing a Kan) done
// 
// *Special
// !Tenpai/Machi done
// Dora counting (not a yaku but affects scoring)
// Fu calculation
// Han → Score conversion table



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



#[derive(Resource)]
struct GameState {
    rounds: u8,
    turns: u8,
    bakaze: Wind,
    bullet: u8,
    calls_made: bool,  // ! IMPORTANT: removed after the first call
}

#[derive(Resource)]
struct Wall(Vec<Tile>);

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

#[derive(Message)]
struct DeclareRonMessage {
    player: Entity,
    discard_tile: Tile,
    discarded_by: Entity,
    is_chankan: bool,
}

#[derive(Message)]
struct DeclareTsumoMessage {
    player: Entity,
    drawn_tile: Tile,
    is_rinshan: bool,
}



struct HandResult {
    yaku_names: Vec<String>,
    total_han: u8,
    total_fu: u8,
    is_yakuman: bool,
}

// use or dispose later
struct Player {
    points: i32,
    hand: Vec<Tile>,
    drawn: Option<Tile>,
    
    open_mentsu: Vec<Mentsu>,
    jikaze: Wind,
    is_tenpai: bool,
    is_hand_closed: bool,
    is_riichi: bool,
    turns_since_riichi: u8,
    is_alive: bool,
    aggression: u8,
    defense: u8,
    cheating_inclination: u8, 
}

// same with this
struct Game {
    rounds: u8,
    turns: u8,
    oya: Player,
    wall: Vec<Tile>,
    bakaze: Wind,
    bullet: u8,
    player_discard: Option<Tile>,
}

fn check_ryuukoku(wall: &Wall) -> bool {
    wall.0.len() <= 14 // dead wall
}

fn is_furiten(discard_pile: &Kawa, tenpai: &Tenpai) -> bool {
    tenpai.0.iter().any(|wait| discard_pile.0.contains(wait))
}


fn evaluate_yaku(
    results: &[Vec<Mentsu>],
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
    turns: u8,
    winning_tile: &Tile,
    is_tsumo: bool,
    is_rinshan: bool,
    is_chankan: bool,
    wall: &Wall,
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
        add_situational_yakuman(&mut eval, turns, is_oya, is_tsumo, calls_made);
        return eval;
    }

    // 2
    for result in results {
        let eval = evaluate_standard(
            result, combined_hand, open_mentsu,
            is_hand_closed, is_oya, is_riichi, is_double_riichi,
            is_ippatsu, bakaze, jikaze, turns, winning_tile,
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
            is_tsumo, is_chankan, is_oya, turns, wall, calls_made
        );
        if is_better(&eval, &best) { 
            best = eval; 
        }
    }

    best
}


fn evaluate_standard(
    result: &[Mentsu],
    combined_hand: &[Tile],
    open_mentsu: &[Mentsu],
    is_hand_closed: bool,
    is_oya: bool,
    is_riichi: bool,
    is_double_riichi: bool,
    is_ippatsu: bool,
    bakaze: &Wind,
    jikaze: &Wind,
    turns: u8,
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

    if suuankou(result) {
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

    add_situational_yakuman(&mut eval, turns, is_oya, is_tsumo, calls_made);

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

    if sanankou(result) {
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
    turns: u8,
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
        add_situational_yakuman(&mut eval, turns, is_oya, is_tsumo, calls_made);
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


fn add_situational_yakuman(eval: &mut HandResult, turns: u8, is_oya: bool, is_tsumo: bool, calls_made: bool) {
    
    if tenhou(turns, is_oya, is_tsumo, calls_made) {
        eval.yaku_names.push("Tenhou".to_string());
    }

    if chiihou(turns, is_oya, is_tsumo, calls_made) {
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
    if new.is_yakuman && !old.is_yakuman { return true; }
    if !new.is_yakuman && old.is_yakuman { return false; }
    if new.is_yakuman && old.is_yakuman {
        return new.yaku_names.len() > old.yaku_names.len();
    }
    if new.total_han != old.total_han {
        return new.total_han > old.total_han;
    }
    new.total_fu > old.total_fu
}


// call on opponent discard
fn can_declare_ron(
    discard_tile: &Tile,
    hand: &[Tile],
    open_mentsu: &[Mentsu],
    tenpai: &Tenpai,
    discard_pile: &Kawa,
    is_hand_closed: bool,
    is_oya: bool,
    turns: u8,
    is_riichi: bool,
    is_double_riichi: bool,
    is_ippatsu: bool,
    bakaze: &Wind,
    jikaze: &Wind,
    wall: &Wall,
    is_chankan: bool,
    calls_made: bool,
) -> bool {
    if !tenpai.0.contains(discard_tile) || is_furiten(discard_pile, tenpai) {
        return false;
    }

    // !sort and combine open_mentsu here
    let mut combined_hand = hand.to_owned();
    combined_hand.push(*discard_tile);
    let results = decompose(&combined_hand);

    // yaku validation
    !evaluate_yaku(
        &results,
        hand,            // raw hand (before adding discard)
        &combined_hand,      // combined hand
        open_mentsu,
        is_hand_closed,
        is_oya,
        is_riichi,
        is_double_riichi,
        is_ippatsu,
        bakaze,
        jikaze,
        turns,                             // for tenhou/chiihou
        discard_tile,
        false,                 // is_tsumo, ron is never tsumo
        false,              // is_rinshan, ron is never rinshan
        is_chankan,          
        wall,
        calls_made).yaku_names.is_empty() 

}


fn declare_ron(
    mut messages: MessageReader<DeclareRonMessage>,
    query: Query<(&Hand, &OpenMentsu, &Tenpai, &Kawa, &Jikaze, Has<ClosedHand>, Has<Oya>, Option<&Riichi>)>,
    game: Res<GameState>,
    wall: Res<Wall>
) {
    for message in messages.read() {
        if let Ok((hand, open, tenpai, kawa, jikaze,
            is_closed, is_oya, maybe_riichi)) = query.get(message.player)
        {
            let is_riichi = maybe_riichi.is_some();
            let is_double = maybe_riichi.map_or(false, |r| r.is_double);
            let is_ippatsu = maybe_riichi.map_or(false, |r| r.is_ippatsu_alive);

            if can_declare_ron(
                &message.discard_tile,
                &hand.0,
                &open.0,
                tenpai,
                kawa,
                is_closed,
                is_oya,
                game.turns,
                is_riichi,
                is_double,
                is_ippatsu, 
                &game.bakaze,
                &jikaze.0,
                &*wall, // what the fuck is this?
                message.is_chankan,
                game.calls_made
            ) {
                // !score and  transfer points
                // !should take the yaku list vector also?
            }
        }
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
    turns: u8,
    is_riichi: bool,
    is_double_riichi: bool,
    is_ippatsu: bool,
    bakaze: &Wind,
    jikaze: &Wind,
    wall: &Wall,
    is_rinshan: bool,
    calls_made: bool,
) -> bool {
    if !tenpai.0.contains(drawn_tile)  {
        return false;
    }

    // !also sort and combine here
    let mut combined_hand = hand.to_owned();
    combined_hand.push(*drawn_tile);
    let results = decompose(&combined_hand);

    // yaku validation
    !evaluate_yaku(
        &results,
        hand,            // raw hand (before adding drawn)
        &combined_hand,      // combined hand
        open_mentsu,
        is_hand_closed,
        is_oya,
        is_riichi,
        is_double_riichi,
        is_ippatsu,
        bakaze,
        jikaze,
        turns,                             // for tenhou/chiihou
        drawn_tile,
        true,                
        is_rinshan,              
        false,          // is_chankan, tsumo can't chankan
        wall,
        calls_made).yaku_names.is_empty() 

}


fn declare_tsumo(
    mut messages: MessageReader<DeclareTsumoMessage>,
    query: Query<(&Hand, &OpenMentsu, &Tenpai, &Jikaze, Has<ClosedHand>, Has<Oya>, Option<&Riichi>)>,
    game: Res<GameState>,
    wall: Res<Wall>
) {
    for message in messages.read() {
        if let Ok((hand, open, tenpai, jikaze,
            is_closed, is_oya, maybe_riichi)) = query.get(message.player)
        {
            let is_riichi = maybe_riichi.is_some();
            let is_double = maybe_riichi.map_or(false, |r| r.is_double);
            let is_ippatsu = maybe_riichi.map_or(false, |r| r.is_ippatsu_alive);

            if can_declare_tsumo(
                &message.drawn_tile, 
                &hand.0, 
                &open.0, 
                tenpai,
                is_closed, 
                is_oya, 
                game.turns, 
                is_riichi,
                is_double, 
                is_ippatsu,
                &game.bakaze,
                &jikaze.0,
                &*wall,
                message.is_rinshan,
                game.calls_made,
            ) {
                // !score and  transfer points
            }
        }
    }
}





// raw hand nomi works as well, no need to combine with open mentsu!
fn check_tenpai(raw_hand: &[Tile]) -> Vec<Tile> { 
    let mut waiting_on: Vec<Tile> = vec![];
    for tile in all_tiles() {
        let mut hand_speculated = raw_hand.to_owned();
        hand_speculated.push(tile);
         if !decompose(&hand_speculated).is_empty() {
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

fn tenpai_payout_system(mut query: Query<&mut Points, With<Tenpai>>) {
    for mut player_points in &mut query {
        player_points.0 += 1000;
    }
}


fn can_declare_riichi(hand: &[Tile], is_closed: bool, is_riichi: bool, points: i32, wall: &Wall) -> bool {
    !is_riichi
        && is_closed
        && points >= 1000
        && wall.0.len() >= 4
        && !check_tenpai(hand).is_empty()
}


fn declare_riichi(
    mut messages: MessageReader<DeclareRiichiMessage>, // store the entity id
    mut query: Query<(Has<ClosedHand>, Has<Riichi>, &Hand, &mut Points)>, // store the data
    wall: Res<Wall>,
    game: Res<GameState>,
    mut commands: Commands,
) {
    for message in messages.read() {
        if let Ok((is_closed, is_riichi, hand, mut points)) = query.get_mut(message.player) { 
            if can_declare_riichi(&hand.0, is_closed, is_riichi, points.0, &*wall) {
                let is_double = game.turns == 1 && !game.calls_made;
                commands.entity(message.player).insert(Riichi { 
                    is_double, 
                    is_ippatsu_alive: true, 
                    turns_since: 0 });
                points.0 -= 1000;
            }
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
    mut query: Query<(&mut Hand, &mut OpenMentsu)>,
    mut commands: Commands,
) {
    for message in messages.read(){
        if let Ok((mut hand, mut open_mentsu)) = query.get_mut(message.player){
            if can_declare_pon(&hand.0 ,&message.tile) {
                open_mentsu.0.push(Mentsu::Koutsu(vec![message.tile; 3], false));
                for _ in 0..2 {
                    let idx = hand.0.iter().position(|x| *x == message.tile).unwrap();
                    hand.0.remove(idx);
                }
                commands.entity(message.player).remove::<ClosedHand>();
            }
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

fn declare_chi(
    mut messages: MessageReader<DeclareChiMessage>,
    mut query: Query<(&mut Hand, &mut OpenMentsu)>,
    mut commands: Commands
) {
    for message in messages.read() {
        if let Ok((mut hand, mut open_mentsu)) = query.get_mut(message.player){
            let positions = can_declare_chi(&hand.0, &message.tile);
            if !positions.is_empty() && positions.contains(&message.pos) {
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
                        commands.entity(message.player).remove::<ClosedHand>();
                    },
                    ChiTilePos::Left => {
                        let next = next_tile_sequence(tile).unwrap();
                        let next_next = next_tile_sequence(&next).unwrap();
                        hand.remove_tile_from_hand(&next);
                        hand.remove_tile_from_hand(&next_next);
                        open_mentsu.0.push(Mentsu::Shuntsu(vec![*tile, next, next_next], false));
                        commands.entity(message.player).remove::<ClosedHand>();
                    },
                    ChiTilePos::Right => {
                        let prev = previous_tile_sequence(tile).unwrap();
                        let prev_prev = previous_tile_sequence(&prev).unwrap();
                        hand.remove_tile_from_hand(&prev);
                        hand.remove_tile_from_hand(&prev_prev);
                        open_mentsu.0.push(Mentsu::Shuntsu(vec![prev_prev, prev, *tile], false));
                        commands.entity(message.player).remove::<ClosedHand>();
                    },
                }

            }
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


fn declare_kan(
    mut messages: MessageReader<DeclareKanMessage>,
    mut query: Query<(&mut Hand, &mut OpenMentsu)>,
    mut commands: Commands
) { 
    for message in messages.read() {
        if let Ok((mut hand, mut open_mentsu)) = query.get_mut(message.player){
            let tile = &message.tile;
            let count = can_declare_kan_from_hand(&hand.0, tile);
            if message.is_discard && count == 3 {
                open_mentsu.0.push(Mentsu::Daiminkan(vec![*tile; 4]));
                hand.0.retain(|x| x != tile);
                commands.entity(message.player).remove::<ClosedHand>();
            } 
            else if !message.is_discard && count == 4 {
                open_mentsu.0.push(Mentsu::Ankan(vec![*tile; 4]));
                hand.0.retain(|x| x != tile);
            }  
            else if !message.is_discard { // this check should be enough hopefully
                for mentsu in &mut open_mentsu.0 {
                    if let Mentsu::Koutsu(tiles, false) = mentsu && tiles[0] == *tile {
                        // deref to mutate
                        *mentsu = Mentsu::Shouminkan(vec![*tile; 4]);
                        hand.0.retain(|x| x != tile);
                        break;
                    } 
                }
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


fn check_win(hand: &[Tile], player: &Player) -> Option<Vec<Vec<Mentsu>>> {
    let mut results = decompose(hand);
    if results.is_empty() {
        None
    } else {
        if !player.open_mentsu.is_empty() {
            for result in &mut results {
                result.extend(player.open_mentsu.clone());
                // result.sort();
            }
        }
        Some(results)
    }
}





fn tanyao(hand: &[Tile]) -> bool {
    hand.iter().all(|x| !is_yaochuuhai(x))    
}


fn kokushi_musou(hand: &[Tile]) -> bool { //! this should be combined hand
    if hand.iter().all(is_yaochuuhai) {
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
            let mut count = 0 as u8;
            match tile {
                Tile::Honor(Honor::Red | Honor::Green | Honor::White) => count += 1,
                _ => {}
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


fn sanankou(result: &[Mentsu]) -> bool {
    result
        .iter()
        .filter(|mentsu| 
            matches!(mentsu, Mentsu::Koutsu(_, true) | Mentsu::Ankan(_)))
        .count() == 3
}


fn suuankou(result: &[Mentsu]) -> bool {
    result
        .iter()
        .filter(|mentsu| 
            matches!(mentsu, Mentsu::Koutsu(_, true) | Mentsu::Ankan(_)))
        .count() == 4 
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
    if hand.len() != 14 {return false;}
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


fn chuuren_poutou(hand: &[Tile]) -> bool {
    if hand.iter().all(|x| matches!(x, Tile::Man(_)))
        || hand.iter().all(|x| matches!(x, Tile::Pin(_)))
        || hand.iter().all(|x| matches!(x, Tile::Sou(_))) {
            for i in 1..=9 {
                if !hand.contains(&Tile::Man(i)) && !hand.contains(&Tile::Pin(i)) && !hand.contains(&Tile::Sou(i)) {
                    return false;
                }
            }
            hand.iter().filter(|x| matches!(x, Tile::Man(1) | Tile::Pin(1) | Tile::Sou(1))).count() >= 3 
                && hand.iter().filter(|x| matches!(x, Tile::Man(9) | Tile::Pin(9) | Tile::Sou(9))).count() >= 3 
    } else {
        false
    }
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


fn pinfu(result: &[Mentsu], winning_tile: &Tile, jikaze: &Wind, bakaze: &Wind) -> bool {
    let mut shuntsu_count = 0;
    let mut has_ryanmen = false;
    let mut has_valid_jantou = false;

    for mentsu in result {
        
        match mentsu {
            Mentsu::Shuntsu(tiles, true) => {
                shuntsu_count += 1;
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


fn haitei(wall: &Wall, is_tsumo: bool) -> bool {
    wall.0.len() == 14 && is_tsumo
}

fn houtei(wall: &Wall, is_tsumo: bool) -> bool {
    wall.0.len() == 14 && !is_tsumo
}

fn tenhou(turns: u8, is_oya: bool, is_tsumo: bool, calls_made: bool) -> bool {
    turns == 1 && is_oya && is_tsumo && !calls_made
} 

fn chiihou(turns: u8, is_oya: bool, is_tsumo: bool, calls_made: bool) -> bool {
    turns == 1 && !is_oya && is_tsumo && !calls_made
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
// ! what about hadaka tanki?
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


fn start_game(mut commands: Commands) {
    let mut wall = vec![];
    for _ in 0..4 {
        wall.extend(all_tiles());
    }
    wall.shuffle(&mut rand::rng());
    
    commands.spawn((
        PlayerTag, 
        Points(25000),
        Jikaze(Wind::East),
        Hand(vec![]),
        OpenMentsu(vec![]),
        Alive,
        ClosedHand,
        Oya,
    ));
    commands.spawn((
        DiscardedTile(),
        DiscardedBy(),
        DiscardTurn(0),
    ));
    commands.insert_resource(
        GameState { 
            rounds: 0, 
            turns: 1, 
            bakaze: Wind::East, 
            bullet: 1,
            calls_made: false,
        }
    );
    commands.insert_resource(
        Wall(wall)
    );

}


fn main() {

    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, start_game)
        .run();


    // TODO: Add a check for removing FirstTurn
    // TODO: logical sorting when player picks up a tile
    
    //let mut player1 = Player {
    //    points: 123,
    //    jikaze: Wind::East,
    //    hand: vec![],
    //    drawn: Tile::Man(3),
    //    opponent_discard: Tile::Man(4),
    //    open_mentsu: vec![],
    //    is_hand_closed: true,
    //    is_tenpai: false,
    //    is_alive: true,
    //    aggression: 10,
    //    defense: 10,
    //    cheating_inclination: 10, 
    //};
//
    //let game = Game {
    //    rounds: 3,
    //    turns: 1,
    //    wall: all_tiles(),
    //    bakaze: Wind::East,
    //    bullet: 123,
    //};
//
    //// let mut wall = vec![Tile::Sou(1), Tile::Honor(Honor::Red)];
//
    //// logical sorting when player picks up a card
    //player1.hand.extend(vec![Tile::Sou(1), Tile::Honor(Honor::Red)]);
    //player1.hand.sort();
    //if let Some(results) = check_win(&player1.hand, &player1) {
//
    //    kokushi_musou(&player1.hand);
    //    let hand = combine_tiles(&player1);
    //    kokushi_musou(&player1.hand);
    //    // yakuman
    //    tsuuisou(&hand);
    //    
    //    daisangen(&results);
    //    suukantsu(&player1);
//
    //    // regular yaku with unusual pattern
    //    chiitoitsu(&hand);
//
    //    // regular yaku
    //    iipeikou(&results);  
    //    tanyao(&hand); // closed
    //    sankantsu(&player1);
    //    
    //    yakuhai(&player1, &results, &game.bakaze); // open
    //    
    //    toitoi(&results);
    //    honitsu(&hand);
    //    if chinitsu(&hand) {}
    //     // other yaku
    //
//
    //}
//
    //for tile in game.wall {
    //    for _ in 0..4 {
    //        println!("{:?}", tile);
    //    } 
    //}
    
}