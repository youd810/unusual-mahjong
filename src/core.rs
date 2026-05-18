use crate::components::*;
use crate::yaku::*;
use crate::scoring::*;
use crate::resources::*;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub enum Tile {
    Man(u8),
    Pin(u8),
    Sou(u8),
    Honor(Honor),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub enum Honor {
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
pub enum Mentsu {
    Jantou(Vec<Tile>),
    Koutsu(Vec<Tile>, bool), // true = closed
    Shuntsu(Vec<Tile>, bool),
    Ankan(Vec<Tile>),
    Daiminkan(Vec<Tile>),
    Shouminkan(Vec<Tile>),
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Kantsu {
    Ankan,
    Daiminkan,
    Shouminkan,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Wind {
    East,
    South,
    West,
    North
}

impl Wind {
    // riichi sticks distribution
    pub fn to_num(&self) -> u8 {
        match self {
            Wind::East => 0,
            Wind::South => 1,
            Wind::West => 2,
            Wind::North => 3,
        }
    }

    pub fn distance_to(&self, other: &Wind) -> u8 {
        (other.to_num() + 4 - self.to_num()) % 4
    }

    pub fn is_kamicha_to(&self, discard_wind: &Wind) -> bool {
        matches!((self, discard_wind),
        (Wind::South, Wind::East)
            | (Wind::West, Wind::South)
            | (Wind::North, Wind::West)
            | (Wind::East, Wind::North))
    }

    pub fn next_turn_wind(&self) -> Wind {
        match self {
            Wind::East => Wind::South,
            Wind::South => Wind::West,
            Wind::West => Wind::North,
            Wind::North => Wind::East,
        }
    }

    pub fn next_round_wind(&self) -> Wind {
        match self {
            Wind::East => Wind::North,
            Wind::South => Wind::East,
            Wind::West =>  Wind::South,
            Wind::North => Wind::West,
        }
    }

}


#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ChiTilePos { // tile discarded
    Left,  
    Middle, 
    Right,  
}

pub fn all_tiles() -> Vec<Tile> {
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
pub fn decompose(tiles: &[Tile]) -> Vec<Vec<Mentsu>> {
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


pub fn find_mentsu(remaining: &[Tile], current: Vec<Mentsu>, results: &mut Vec<Vec<Mentsu>>) {
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

pub fn combine_tiles(hand: &Hand, open_mentsu: &OpenMentsu) -> Vec<Tile> {
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


pub fn next_tile_sequence(tile: &Tile) -> Option<Tile> {
    match tile {
        Tile::Man(n) if *n < 9 => Some(Tile::Man(n + 1)),
        Tile::Pin(n) if *n < 9 => Some(Tile::Pin(n + 1)),
        Tile::Sou(n) if *n < 9 => Some(Tile::Sou(n + 1)),
        _ => None, 
    }
}


pub fn previous_tile_sequence(tile: &Tile) -> Option<Tile> {
    match tile {
        Tile::Man(n) if *n > 1 => Some(Tile::Man(n - 1)),
        Tile::Pin(n) if *n > 1 => Some(Tile::Pin(n - 1)),
        Tile::Sou(n) if *n > 1 => Some(Tile::Sou(n - 1)),
        _ => None, 
    }
}


pub fn wind_to_honor(wind: &Wind) -> Honor {
    match wind {
        Wind::East => Honor::East,
        Wind::South => Honor::South,
        Wind::West => Honor::West,
        Wind::North => Honor::North,
    }
}

pub fn is_ryanmen_wait(shuntsu_tiles: &[Tile], winning_tile: &Tile) -> bool {
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

pub fn is_penchan_wait(result: &[Mentsu], winning_tile: &Tile) -> bool {
    result.iter().any(|mentsu| {
    if let Mentsu::Shuntsu(tiles, _) = mentsu {
        (tiles[0] == *winning_tile && is_terminal(&tiles[2])) || // 7 machi
        (tiles[2] == *winning_tile && is_terminal(&tiles[0]))    // 3 machi
    } else {
        false
    }
    })
}

pub fn is_kanchan_wait(result: &[Mentsu], winning_tile: &Tile) -> bool {
    result.iter().any(|mentsu| matches!(mentsu, Mentsu::Shuntsu(tiles, _) if tiles[1] == *winning_tile)) // 2 5 8
}

pub fn is_tanki_wait(result: &[Mentsu], winning_tile: &Tile) -> bool {
    result.iter().any(|mentsu| matches!(mentsu, Mentsu::Jantou(tiles) if tiles[0] == *winning_tile))
}


pub fn is_terminal(tile: &Tile) -> bool {
    matches!(tile, Tile::Sou(1 | 9) | Tile::Pin(1 | 9) | Tile::Man(1 | 9))
}


pub fn is_honor(tile: &Tile) -> bool {
    matches!(tile, Tile::Honor(_))
}


pub fn is_yaochuuhai(tile: &Tile) -> bool {
    is_terminal(tile) || is_honor(tile)
}

pub fn is_green(tile: &Tile) -> bool {
    matches!(tile, Tile::Sou(2 | 3 | 4 | 6 | 8) | Tile::Honor(Honor::Green))
}


pub fn has_shuntsu(result: &[Mentsu], first_tile: Tile) -> bool {
    result.iter().any(|mentsu|{
        if let Mentsu::Shuntsu(tiles, _) = mentsu {
            tiles[0] == first_tile 
        } else {
            false
        }
    })
}


pub fn has_koutsu_or_kan(result: &[Mentsu], first_tile: Tile) -> bool{
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

pub fn has_jantou(result: &[Mentsu], target_tile: Tile) -> bool {
    result.iter().any(|mentsu| {
        if let Mentsu::Jantou(tiles) = mentsu {
            tiles[0] == target_tile
        } else {
            false
        }
    })
}

pub fn can_declare_pon(hand: &[Tile], tile: &Tile,) -> bool {
    hand.iter().filter(|x| **x == *tile).count() >= 2
}

pub fn can_declare_chi(hand: &[Tile], tile: &Tile) -> Vec<ChiTilePos> {
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

pub fn can_declare_kan_from_hand(hand: &[Tile], tile: &Tile) -> u8 {
    hand.iter().filter(|x| *x == tile).count() as u8
}

pub fn can_declare_kan_from_pon(open_mentsu: &[Mentsu], tile: &Tile) -> bool{
    open_mentsu.iter().any(|mentsu| {
        if let Mentsu::Koutsu(tiles, false) = mentsu && tiles[0] == *tile {
            true
        } else {false}
    }) 
}


// raw hand nomi works as well, no need to combine with open mentsu!
pub fn check_tenpai(raw_hand: &[Tile]) -> Vec<Tile> { 
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


pub fn is_furiten(kawa: &Kawa, tenpai: &Tenpai) -> bool {
    tenpai.0.iter().any(|wait| kawa.0.contains(wait))
}


// call on opponent discard
// こんな引数を見せられたら誰でも泣きたくなるんだよなぁ
pub fn can_declare_ron(
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
    has_temp_furiten: bool,
) -> Option<HandResult> {
    if !tenpai.0.contains(discard_tile) || is_furiten(kawa, tenpai) || has_temp_furiten {
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



// call on self draw
pub fn can_declare_tsumo(
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


pub fn can_declare_kyuushu(hand: &[Tile], calls_made: bool, kawa: &Kawa) -> bool {
    let mut yaochuuhai: Vec<&Tile> = hand.iter().filter(|x| is_yaochuuhai(x)).collect();
    yaochuuhai.sort();
    yaochuuhai.dedup();
    yaochuuhai.len() >= 9 && !calls_made && kawa.0.is_empty()
}