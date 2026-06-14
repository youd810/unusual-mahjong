use bevy::prelude::Entity;

use crate::components::*;
use crate::yaku::*;
use crate::scoring::*;
use crate::resources::*;
use crate::messages::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchPhase {
    Yonma, // 4 players
    Sanma, // 3
    Nima,  // 2
}

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
pub enum Kantsu { // drawn kan only
    Ankan,
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
    pub fn wind_to_honor(&self) -> Honor {
        match self {
            Wind::East => Honor::East,
            Wind::South => Honor::South,
            Wind::West => Honor::West,
            Wind::North => Honor::North,
        }
    }

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


pub fn get_discard_penalty(tile: &Tile, target: &TargetYaku, freq: u8, is_dora: bool) -> i32 {
    let mut penalty = if is_dora { 800 } else { 0 }; // TODO: needs testing

    let is_honor = is_honor(tile);
    let is_terminal = is_terminal(tile);
    let is_yaochuuhai = is_yaochuuhai(tile);

    penalty += match target {
        TargetYaku::Speed => 0,
        TargetYaku::Tanyao => {
            if !is_yaochuuhai { 10000 } else { 0 }
        }
        TargetYaku::Honitsu(target_suit) => {
            let matches_suit = matches!((tile, target_suit), (Tile::Man(_), Suit::Man) | (Tile::Pin(_), Suit::Pin) | (Tile::Sou(_), Suit::Sou));
            if matches_suit || is_honor { 10000 } else { 0 }
        }
        TargetYaku::Chinitsu(target_suit) => {
            let matches_suit = matches!((tile, target_suit), (Tile::Man(_), Suit::Man) | (Tile::Pin(_), Suit::Pin) | (Tile::Sou(_), Suit::Sou));
            if matches_suit { 10000 } else { 0 }
        }
        TargetYaku::Ittsuu(target_suit) => {
            let matches_suit = matches!((tile, target_suit), (Tile::Man(_), Suit::Man) | (Tile::Pin(_), Suit::Pin) | (Tile::Sou(_), Suit::Sou));
            if matches_suit { 10000 } else { 0 }
        }
        TargetYaku::SanshokuDoujun(n) => {
            let dominated = match tile {
                Tile::Man(v) | Tile::Pin(v) | Tile::Sou(v) => *v >= *n && *v <= n + 2,
                _ => false,
            };
            if dominated { 10000 } else { 0 }
        }
        TargetYaku::Chanta => {
            let is_chanta_tile = matches!(tile,
                Tile::Man(1|2|3|7|8|9) | Tile::Pin(1|2|3|7|8|9) | Tile::Sou(1|2|3|7|8|9) | Tile::Honor(_)
            );
            if is_chanta_tile { 10000 } else { 0 }
        }
        TargetYaku::Junchan => {
            let is_junchan_tile = matches!(tile,
                Tile::Man(1|2|3|7|8|9) | Tile::Pin(1|2|3|7|8|9) | Tile::Sou(1|2|3|7|8|9)
            );
            if is_junchan_tile { 10000 } else { 0 }
        }
        TargetYaku::Pinfu => {
            if is_honor { 10000 } else { 0 }
        }
        TargetYaku::Toitoi | TargetYaku::Sanankou | TargetYaku::Suuankou => {
            if freq >= 2 { 10000 } else { 0 }
        }
        TargetYaku::Chiitoitsu => {
            if freq == 2 { 10000 } else { 0 }
        }
        TargetYaku::Kokushi => {
            if is_yaochuuhai && freq == 1 { 10000 } else { 0 }
        }
        TargetYaku::ChuurenPoutou(target_suit) => {
            let matches_suit = matches!((tile, target_suit), (Tile::Man(_), Suit::Man) | (Tile::Pin(_), Suit::Pin) | (Tile::Sou(_), Suit::Sou));
            if matches_suit {
                if is_terminal { 20000 } else { 10000 }
            } else { 0 }
        }
        TargetYaku::Daisangen => {
            if matches!(tile, Tile::Honor(Honor::White | Honor::Green | Honor::Red)) { 10000 } else { 0 }
        }
        TargetYaku::Tsuuiisou => {
            if is_honor { 10000 } else { 0 }
        }
        TargetYaku::Ryuuiisou => {
            if is_green(tile) { 10000 } else { 0 }
        }
    };

    penalty
}



pub fn evaluate_discard(
    hand_plus_drawn: &[Tile],
    open_mentsu: &[Mentsu],
    visible_tiles: &[u8; 34],
    safe_tiles: &[Tile],
    should_defend: bool,
    forbidden: Option<&[Tile]>,
    target_yaku: TargetYaku,
    dora_indicators: &[Tile],
    own_kawa: &[Tile], 
) -> Tile {
    let mut safe_map =[0; 34];
    let mut has_safe_tiles = false;

    for tile in safe_tiles {
        safe_map[tile_to_index(tile)] += 1;
        if hand_plus_drawn.contains(tile) {
            has_safe_tiles = true;
        }
    }

    let combined_hand = combine_tiles(hand_plus_drawn, open_mentsu);
    let mut freq_array = tiles_to_frequency_array(&combined_hand);

    let mut best_index = 0;
    let mut best_score = i32::MAX;
    let mut found_valid = false;

    for i in 0..34 {
        let current_tile = index_to_tile(i);

        if freq_array[i] == 0 || !hand_plus_drawn.contains(&current_tile) {
            continue;
        }

        if let Some(f) = forbidden && f.contains(&current_tile) { continue; }

        if has_safe_tiles && safe_map[i] == 0 && should_defend {
            continue;
        }

        found_valid = true;

        let original_freq = freq_array[i];
        freq_array[i] -= 1;
        let shanten = calculate_shanten_from_array(&mut freq_array);
        let ukeire = ukeire_tiles(&mut freq_array, shanten);
        let ukeire_count: i32 = ukeire.iter()
            .map(|&j| 4i32.saturating_sub(freq_array[j] as i32).saturating_sub(visible_tiles[j] as i32).max(0))
            .sum();

        // furiten check
        let mut furiten_penalty = 0;
        if shanten == 0 {
            let is_furiten = ukeire.iter().any(|&wait_idx| {
                own_kawa.contains(&index_to_tile(wait_idx))
            });
            if is_furiten {
                furiten_penalty = 15000;
            }
        }

        let mut is_dora = false;
        for ind in dora_indicators {
            if current_tile == get_dora_from_indicator(ind) {
                is_dora = true;
                break;
            }
        }

        // defense overrides target logic entirely
        let penalty = if should_defend {
            0
        } else {
            get_discard_penalty(&current_tile, &target_yaku, original_freq, is_dora)
        };

        // shanten is scaled by 1000 so it dominates ukeire (which can go to tens or hundreds)
        // lower is better
        let score = (shanten * 1000) - ukeire_count + penalty + furiten_penalty;

        if score < best_score {
            best_score = score;
            best_index = i;
        }

        freq_array[i] += 1;
    }

    if !found_valid {
        for tile in hand_plus_drawn {
            if let Some(f) = forbidden && f.contains(tile) { continue; }
            return *tile;
        }
    }

    index_to_tile(best_index)
}


// TODO: needs refinement
pub fn estimate_yaku_han(combined_hand: &[Tile], jikaze: &Wind, bakaze: &Wind) -> u8 {
    let mut han = 0;
    let freq = tiles_to_frequency_array(combined_hand);

    // suit
    if chinitsu(combined_hand) { han += 5; }
    else if honitsu(combined_hand) { han += 2; }

    // num
    if tanyao(combined_hand) { han += 1; }
    if honroutou(combined_hand) { han += 2; }

    // yakuhai
    let yakuhai_indexes =[
        tile_to_index(&Tile::Honor(Honor::Red)),
        tile_to_index(&Tile::Honor(Honor::Green)),
        tile_to_index(&Tile::Honor(Honor::White)),
        tile_to_index(&Tile::Honor(jikaze.wind_to_honor())),
        tile_to_index(&Tile::Honor(bakaze.wind_to_honor())),
    ];
    for &idx in &yakuhai_indexes {
        if freq[idx] >= 2 { han += 1; }
    }

    // toitoi
    let pairs_or_triplets = freq.iter().filter(|&&f| f >= 2).count();
    if pairs_or_triplets >= 4 { han += 2; }

    // chanta
    let yaochuuhai_count = combined_hand.iter().filter(|t| is_yaochuuhai(t)).count();
    if yaochuuhai_count >= 8 && !tanyao(combined_hand) {
        han += 1;
    }

    // itsuu
    for suit_offset in[0, 9, 18] {
        let count = (0..9).filter(|&i| freq[suit_offset + i] > 0).count();
        if count >= 7 {
            han += 1;
            break;
        }
    }

    // sanshoku doujun
    for i in 0..7 {
        let man_has = freq[i] > 0 || freq[i+1] > 0 || freq[i+2] > 0;
        let pin_has = freq[i+9] > 0 || freq[i+10] > 0 || freq[i+11] > 0;
        let sou_has = freq[i+18] > 0 || freq[i+19] > 0 || freq[i+20] > 0;

        if man_has && pin_has && sou_has {
            // check if they have at least 6 tiles contributing to this sequence neighborhood
            let total_tiles = (0..3).map(|offset| freq[i+offset] + freq[i+9+offset] + freq[i+18+offset]).sum::<u8>();
            if total_tiles >= 6 {
                han += 1;
                break;
            }
        }
    }

    han
}


pub fn combine_tiles(hand: &[Tile], open_mentsu: &[Mentsu]) -> Vec<Tile> {
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
    nuked_tiles: &[Tile],
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

    let mut yaku_result = evaluate_yaku(
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
        kawa, // for tenhou/chiihou
        discard_tile,
        false, // ron != tsumo
        false, // ron can't be rinshan
        is_chankan,
        wall,
        dead_wall,
        calls_made,
        nuked_tiles
    );

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
    nuked_tiles: &[Tile],
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

    let mut yaku_result = evaluate_yaku(
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
        kawa,
        drawn_tile,
        true, // tsumo is tsumo
        is_rinshan,
        false, // tsumo can't chankan 
        wall,
        dead_wall,
        calls_made,
        nuked_tiles
    );

    if yaku_result.yaku_names.is_empty() {
        None
    } else {
        Some(yaku_result)
    }
}


pub fn best_potential_result(
    hand: &[Tile],
    open_mentsu: &[Mentsu],
    nuked_tiles: &[Tile],
    tenpai: Option<&Tenpai>,
    is_closed: bool,
    is_oya: bool,
    kawa: &Kawa,
    is_riichi: bool,
    is_double_riichi: bool,
    is_ippatsu: bool,
    bakaze: &Wind,
    jikaze: &Wind,
    wall: &Wall,
    dead_wall: &DeadWall,
    calls_made: bool,
    visible_tiles: &[u8; 34],
) -> Option<HandResult> {
    let mut best: Option<HandResult> = None;

    let mut full_visible = *visible_tiles;
    for tile in hand {
        full_visible[tile_to_index(tile)] += 1;
    }
    for mentsu in open_mentsu {
        for tile in mentsu.tiles() {
            full_visible[tile_to_index(tile)] += 1;
        }
    }

    if let Some(tenpai) = tenpai {
        for tile in &tenpai.0 {
            if full_visible[tile_to_index(tile)] >= 4 { continue; }
            if let Some(result) = can_declare_tsumo(
                tile, hand, open_mentsu, nuked_tiles, tenpai,
                is_closed, is_oya, kawa,
                is_riichi, is_double_riichi, is_ippatsu,
                bakaze, jikaze, wall, dead_wall,
                false, calls_made,
            )
            && best.as_ref().is_none_or(|b| is_better(&result, b)) {
                best = Some(result);
            }
        }
    } else {
        let open_vec = open_mentsu.to_vec();
        let combined = combine_tiles(hand, &open_vec);
        let mut freq = tiles_to_frequency_array(&combined);
        let shanten = calculate_shanten_from_array(&mut freq);

        if shanten != 1 { return None; }

        let advancing = ukeire_tiles(&mut freq, 1);

        for tile_idx in advancing {
            if full_visible[tile_idx] >= 4 { continue; }

            let incoming = index_to_tile(tile_idx);
            let mut expanded = hand.to_vec();
            expanded.push(incoming);
            expanded.sort();

            let mut seen_discards = vec![];
            for d in 0..expanded.len() {
                let discard = expanded[d];
                if seen_discards.contains(&discard) { continue; }
                seen_discards.push(discard);

                let mut after_discard = expanded.clone();
                after_discard.remove(d);

                let waits = check_tenpai(&after_discard);
                if waits.is_empty() { continue; }

                let temp_tenpai = Tenpai(waits);

                for wait in &temp_tenpai.0 {
                    if full_visible[tile_to_index(wait)] >= 4 { continue; }
                    if let Some(result) = can_declare_tsumo(
                        wait, &after_discard, open_mentsu, nuked_tiles, &temp_tenpai,
                        is_closed, is_oya, kawa,
                        is_riichi, is_double_riichi, is_ippatsu,
                        bakaze, jikaze, wall, dead_wall,
                        false, calls_made,
                    )
                    && best.as_ref().is_none_or(|b| is_better(&result, b)) {
                        best = Some(result);
                    }
                }
            }
        }
    }

    best
}


pub fn build_loser_tilt_info(
    player: Entity,
    hand: &Hand,
    open_mentsu: &OpenMentsu,
    nuked_tiles: &NukedTiles,
    tenpai: Option<&Tenpai>,
    kawa: &Kawa,
    jikaze: &Jikaze,
    is_closed: bool,
    is_oya: bool,
    is_riichi: bool,
    is_double_riichi: bool,
    is_ippatsu: bool,
    bakaze: &Wind,
    wall: &Wall,
    dead_wall: &DeadWall,
    calls_made: bool,
    visible_tiles: &[u8; 34],
) -> LoserTiltInfo {
    let best = best_potential_result(
        &hand.0, &open_mentsu.0, &nuked_tiles.0, tenpai,
        is_closed, is_oya, kawa,
        is_riichi, is_double_riichi, is_ippatsu,
        bakaze, &jikaze.0, wall, dead_wall,
        calls_made, visible_tiles,
    );

    LoserTiltInfo {
        player,
        was_tenpai: tenpai.is_some(),
        was_riichi: is_riichi,
        best_han: best.map(|r| r.total_han),
    }
}


pub fn can_declare_kyuushu(hand: &[Tile], calls_made: bool, kawa: &Kawa) -> bool {
    let mut yaochuuhai: Vec<&Tile> = hand.iter().filter(|x| is_yaochuuhai(x)).collect();
    yaochuuhai.sort();
    yaochuuhai.dedup();
    yaochuuhai.len() >= 9 && !calls_made && kawa.0.is_empty()
}