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

#[derive(PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub enum Mentsu {
    Jantou([Tile; 2]),
    Koutsu([Tile; 3], bool), // true = closed
    Shuntsu([Tile; 3], bool),
    Ankan([Tile; 4]),
    Daiminkan([Tile; 4]),
    Shouminkan([Tile; 4]),
}

impl Mentsu {
    pub fn tiles(&self) -> &[Tile] {
        match self {
            Mentsu::Jantou(t) => t,
            Mentsu::Koutsu(t, _) | Mentsu::Shuntsu(t, _) => t,
            Mentsu::Ankan(t) | Mentsu::Daiminkan(t) | Mentsu::Shouminkan(t) => t,
        }
    }
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
    pub fn to_num(self) -> u8 {
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
            let pair = Mentsu::Jantou([tiles[i], tiles[i+1]]);
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
        let koutsu_group = Mentsu::Koutsu([remaining[0], remaining[1], remaining[2]], true);
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
            let shuntsu_group = Mentsu::Shuntsu([remaining[0], remaining[second_seq], remaining[third_seq]], true);
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


pub fn tile_to_index(tile: &Tile) -> usize {
    match tile {
        Tile::Man(n) => (n - 1) as usize,
        Tile::Pin(n) => (n - 1) as usize + 9,
        Tile::Sou(n) => (n - 1) as usize + 18,
        Tile::Honor(h) => 27 + match h {
            Honor::East => 0,
            Honor::South => 1,
            Honor::West => 2,
            Honor::North => 3,
            Honor::White => 4,
            Honor::Green => 5,
            Honor::Red => 6,
        },
    }
}


pub fn tiles_to_frequency_array(tiles: &[Tile]) -> [u8; 34] {
    let mut freq_array= [0; 34]; 
    for tile in tiles.iter() {
        freq_array[tile_to_index(tile)] += 1
    }
    freq_array
}

pub fn count_blocks(pos: usize, mentsu: u8, partials: u8, freq_array: &mut [u8; 34], has_pair: u8) -> i32 {
    if pos == 34 {
        let valid_partials = partials.min(4u8.saturating_sub(mentsu));
        return 8 - (2 * mentsu as i32) - (valid_partials as i32) - (has_pair as i32);
    }
    let mut min_shanten = count_blocks(pos + 1, mentsu, partials, freq_array, has_pair);
    
    // koutsu
    if freq_array[pos] >= 3 {
        freq_array[pos] -= 3;
        let koutsu_shanten = count_blocks(pos, mentsu + 1, partials, freq_array, has_pair);
        min_shanten = min_shanten.min(koutsu_shanten);
        freq_array[pos] += 3;
    }

    // shuntsu 
    if pos < 27 && pos % 9 <= 6 && freq_array[pos] >= 1 && freq_array[pos+1] >= 1 && freq_array[pos+2] >= 1 {
        for i in 0..3 {
            freq_array[pos + i] -= 1;
        }
        let shuntsu_shanten = count_blocks(pos, mentsu + 1, partials, freq_array, has_pair);
        min_shanten = min_shanten.min(shuntsu_shanten);
        for i in 0..3 {
            freq_array[pos + i] += 1;
        }
    } 

    // toitsu
    if freq_array[pos] >= 2 {
        freq_array[pos] -= 2;
        let toitsu_shanten = count_blocks(pos, mentsu, partials + 1, freq_array, has_pair);
        min_shanten = min_shanten.min(toitsu_shanten);
        freq_array[pos] += 2;
    }
    // ryanmen/penchan
    if pos < 27 && pos % 9 <= 7 && freq_array[pos] >= 1 && freq_array[pos+1] >= 1 {
        for i in 0..2 {
            freq_array[pos + i] -= 1;
        }
        let taatsu_shanten = count_blocks(pos, mentsu, partials + 1, freq_array, has_pair);
        min_shanten = min_shanten.min(taatsu_shanten);
        for i in 0..2 {
            freq_array[pos + i] += 1;
        }
    }
    // kanchan
    if pos < 27 && pos % 9 <= 6 && freq_array[pos] >= 1 && freq_array[pos+2] >= 1 {
        freq_array[pos] -= 1;
        freq_array[pos+2] -= 1;
        let taatsu_shanten = count_blocks(pos, mentsu, partials + 1, freq_array, has_pair);
        min_shanten = min_shanten.min(taatsu_shanten);
        freq_array[pos] += 1;
        freq_array[pos+2] += 1;
    }

    min_shanten

}


pub fn calculate_standard_shanten(mut freq_array: &mut [u8; 34]) -> i32 {
    let mut best_shanten = 8;
    best_shanten = best_shanten.min(count_blocks(0, 0, 0, &mut freq_array, false as u8));

    for i in 0..34 {
        if freq_array[i] >= 2 {
            freq_array[i] -= 2;
            best_shanten = best_shanten.min(count_blocks(0, 0, 0, &mut freq_array, true as u8));
            freq_array[i] += 2;
        }
    }
    best_shanten
}


fn calculate_chiitoitsu_shanten(freq_array: &mut [u8; 34]) -> i32 {
    let mut pairs = 0;

    for tile in freq_array {
        if *tile >= 2 {
            pairs += 1
        }
    }
    6 - pairs
}

fn calculate_kokushi_shanten(freq_array: &mut [u8; 34]) -> i32 {
    let mut yaochuuhai_count = 0;
    let mut has_pair = false;
    const YAOCHUUHAI_POS: [usize; 13] = [
        0, 8, 9, 17, 18, 26, 27, 28, 29, 30, 31, 32, 33
    ];

    for &i in YAOCHUUHAI_POS.iter() {
        if freq_array[i] >= 1 {
            yaochuuhai_count += 1;
        }
        if freq_array[i] >= 2 {
            has_pair = true;
        }
    }

    13 - yaochuuhai_count - has_pair as i32
}


pub fn calculate_shanten(hand: &[Tile]) -> i32 {
    let mut freq_array = tiles_to_frequency_array(hand);
    calculate_shanten_from_array(&mut freq_array)
}


pub fn calculate_shanten_from_array(freq_array: &mut [u8; 34]) -> i32 {
    let mut shanten = 8;
    shanten = shanten.min(calculate_standard_shanten(freq_array));
    shanten = shanten.min(calculate_chiitoitsu_shanten(freq_array));
    shanten = shanten.min(calculate_kokushi_shanten(freq_array));
    shanten
}


pub fn ukeire_tiles(freq_array: &mut [u8; 34], current_shanten: i32) -> Vec<usize> {
    let mut ukeire = vec![];
    for i in 0..34 {
        if freq_array[i] >= 4 { continue; }
        freq_array[i] += 1;
        if calculate_shanten_from_array(freq_array) < current_shanten {
            ukeire.push(i);
        }
        freq_array[i] -= 1;
    }
    ukeire
}

pub fn index_to_tile(index: usize) -> Tile {
    match index {
        0..=8 => Tile::Man((index + 1) as u8),
        9..=17 => Tile::Pin((index - 8) as u8),
        18..=26 => Tile::Sou((index - 17) as u8),
        27 => Tile::Honor(Honor::East),
        28 => Tile::Honor(Honor::South),
        29 => Tile::Honor(Honor::West),
        30 => Tile::Honor(Honor::North),
        31 => Tile::Honor(Honor::White),
        32 => Tile::Honor(Honor::Green),
        33 => Tile::Honor(Honor::Red),
        _ => panic!("invalid tile index: {}", index),
    }
}

pub fn evaluate_discard(
    hand_plus_drawn: &[Tile], 
    open_mentsu: &Vec<Mentsu>, 
    visible_tiles: &[u8; 34], 
    safe_tiles: &[Vec<Tile>], 
) -> Tile {
    let mut safe_map = [0; 34];
    let mut has_safe_tiles = false;
    for kawa in safe_tiles {
        for tile in kawa {
            safe_map[tile_to_index(tile)] += 1;
            if hand_plus_drawn.contains(tile) {
                has_safe_tiles = true;
            }
        }
    }
    
    let combined_hand= combine_tiles(hand_plus_drawn, open_mentsu);
    let mut freq_array = tiles_to_frequency_array(&combined_hand);
    let mut best_index = 0;
    let mut lowest_shanten = 8;
    let mut max_ukeire_count = 0;
    let should_defend = calculate_shanten_from_array(&mut freq_array.to_owned()) > 1;

    for i in 0..34 {
        if (freq_array[i] == 0 || !hand_plus_drawn.contains(&index_to_tile(i))) || (has_safe_tiles && safe_map[i] == 0  && should_defend) { 
            continue; 
        }

        freq_array[i] -= 1;

        let shanten = calculate_shanten_from_array(&mut freq_array);

        if shanten <= lowest_shanten {
            let ukeire = ukeire_tiles(&mut freq_array, shanten);
            let ukeire_count: u8 = ukeire.iter()
                .map(|&j| 4u8.saturating_sub(freq_array[j]).saturating_sub(visible_tiles[j]))
                .sum();

            if shanten < lowest_shanten 
                || (shanten == lowest_shanten && ukeire_count > max_ukeire_count) {
                best_index = i;
                lowest_shanten = shanten;
                max_ukeire_count = ukeire_count;
            }
        };

        freq_array[i] += 1;
    }
    index_to_tile(best_index)
}


pub fn combine_tiles(hand: &[Tile], open_mentsu: &Vec<Mentsu>) -> Vec<Tile> {
    let mut result = hand.to_owned();

    for mentsu in open_mentsu{
            result.extend(mentsu.tiles());
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
        match mentsu {
            Mentsu::Koutsu(tiles, _) => tiles[0] == first_tile,
            Mentsu::Ankan(tiles) 
            | Mentsu::Daiminkan(tiles)
            | Mentsu::Shouminkan(tiles) => tiles[0] == first_tile,
            _ => false
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
        combined_hand.extend(mentsu.tiles());
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
        combined_hand.extend(mentsu.tiles());
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