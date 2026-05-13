use crate::core::*;
use crate::resources::*;
use crate::components::*;


pub fn ryuuiisou(hand: &[Tile]) -> bool {
    hand.iter().all(is_green)
}


pub fn tanyao(hand: &[Tile]) -> bool {
    hand.iter().all(|x| !is_yaochuuhai(x))    
}


pub fn kokushi_musou(hand: &[Tile]) -> bool {
    if hand.len() == 14 && hand.iter().all(is_yaochuuhai) {
        let mut pair_counter: u8 = 0;
        for i in 0..hand.len() - 1 {
            if hand[i] == hand[i+1] {
                pair_counter += 1;
            }
        }

        pair_counter == 1 

    } else {
        false
    }
} 


pub fn tsuuisou(hand: &[Tile]) -> bool {
    hand.iter().all(is_honor)
}


pub fn iipeikou(result: &[Mentsu]) -> bool {
    let shuntsu: Vec<&Mentsu> = result.iter().filter(|x| matches!(x, Mentsu::Shuntsu(_, true))).collect();

    for i in 0..shuntsu.len() {
        for j in i+1..shuntsu.len() {
            if shuntsu[i] == shuntsu[j] {
                return true;
            }
        }
    }

    false
}


pub fn ryanpeikou(result: &[Mentsu]) -> bool {
    let mut shuntsu: Vec<&Mentsu> = result.iter().filter(|x| matches!(x, Mentsu::Shuntsu(_, true))).collect();

    if shuntsu.len() == 4 {
        shuntsu.sort();

        shuntsu[0] == shuntsu[1] && shuntsu[2] == shuntsu[3] 
    
    } else {
        false
    }
    
}


pub fn yakuhai(result: &[Mentsu], jikaze: &Wind, bakaze: &Wind) -> u8 {
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


pub fn sanankou(result: &[Mentsu], winning_tile: &Tile, is_tsumo: bool, thirteen_tiles: &[Tile]) -> bool {
    result
        .iter()
        .filter(|mentsu| {
            if let Mentsu::Koutsu(tiles, true) | Mentsu::Ankan(tiles) = mentsu {
                // compares result with thirteen tiles to see if the winning tile forms the final koutsu and doesn't come from ron 
                // this check should suffice, or shouldn't it?
                !(tiles[0] == *winning_tile && !is_tsumo && thirteen_tiles.iter().filter(|x| *x == winning_tile).count() == 2)
            } else {
                false
            }
        }).count() == 3
}


pub fn suuankou(result: &[Mentsu], winning_tile: &Tile, is_tsumo: bool) -> bool {
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


pub fn toitoi(result: &[Mentsu]) -> bool {
    result
        .iter()
        .filter(|mentsu| 
            matches!(mentsu, Mentsu::Koutsu(_, _) | Mentsu::Ankan(_) | Mentsu::Daiminkan(_) | Mentsu::Shouminkan(_)))
        .count() == 4 
}


pub fn daisangen(result: &[Mentsu]) -> bool {
    has_koutsu_or_kan(result, Tile::Honor(Honor::Red))
        && has_koutsu_or_kan(result, Tile::Honor(Honor::Green)) 
        && has_koutsu_or_kan(result, Tile::Honor(Honor::White))
}


pub fn shousangen(result: &[Mentsu]) -> bool {
    let dragon_kou_or_kan = has_koutsu_or_kan(result, Tile::Honor(Honor::Red)) as u8
        + has_koutsu_or_kan(result, Tile::Honor(Honor::Green)) as u8
        + has_koutsu_or_kan(result, Tile::Honor(Honor::White)) as u8;

    let dragon_jantou = has_jantou(result, Tile::Honor(Honor::Red)) as u8 
        + has_jantou(result, Tile::Honor(Honor::Green)) as u8 
        + has_jantou(result, Tile::Honor(Honor::White)) as u8;

    dragon_kou_or_kan == 2 && dragon_jantou == 1
}


pub fn daisuushii(result: &[Mentsu]) -> bool {
    has_koutsu_or_kan(result, Tile::Honor(Honor::East))
        && has_koutsu_or_kan(result, Tile::Honor(Honor::South)) 
        && has_koutsu_or_kan(result, Tile::Honor(Honor::West))
        && has_koutsu_or_kan(result, Tile::Honor(Honor::North))
}

pub fn shousuushii(result: &[Mentsu]) -> bool {
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


pub fn chinitsu(hand: &[Tile]) -> bool {
    hand.iter().all(|x| matches!(x, Tile::Man(_)))
        || hand.iter().all(|x| matches!(x, Tile::Pin(_))) 
        || hand.iter().all(|x| matches!(x, Tile::Sou(_))) 
}


pub fn honitsu(hand: &[Tile]) -> bool {
    hand.iter().all(|x| matches!(x, Tile::Man(_)) || is_honor(x)) 
        || hand.iter().all(|x| matches!(x, Tile::Pin(_)) || is_honor(x)) 
        || hand.iter().all(|x| matches!(x, Tile::Sou(_)) || is_honor(x)) 
}


pub fn chanta(result: &[Mentsu]) -> bool {
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


pub fn junchan(result: &[Mentsu]) -> bool {
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


pub fn sankantsu(open_mentsu: &[Mentsu]) -> bool {
    open_mentsu.iter().filter(|mentsu|
        matches!(mentsu, Mentsu::Ankan(_) | Mentsu::Daiminkan(_) | Mentsu::Shouminkan(_))).count() == 3 
}


pub fn suukantsu(open_mentsu: &[Mentsu]) -> bool {
    open_mentsu.iter().filter(|mentsu|
        matches!(mentsu, Mentsu::Ankan(_) | Mentsu::Daiminkan(_) | Mentsu::Shouminkan(_))).count() == 4 
}


pub fn chiitoitsu(hand: &[Tile]) -> bool {
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

pub fn honroutou(hand: &[Tile]) -> bool {
    hand.iter().all(is_yaochuuhai)
}


pub fn chinroutou(hand: &[Tile]) -> bool {
    hand.iter().all(is_terminal)
}

pub fn ittsuu(result: &[Mentsu]) -> bool {
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


pub fn sanshoku_doujun(result: &[Mentsu]) -> bool {
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

pub fn sanshoku_doukou(result: &[Mentsu]) -> bool {
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


pub fn chuuren_poutou(combined_hand: &[Tile]) -> bool {
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


pub fn pinfu(result: &[Mentsu], winning_tile: &Tile, jikaze: &Wind, bakaze: &Wind) -> bool {
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


pub fn haitei(wall: &Wall, is_tsumo: bool) -> bool {
    wall.0.is_empty() && is_tsumo
}

pub fn houtei(wall: &Wall, is_tsumo: bool) -> bool {
    wall.0.is_empty() && !is_tsumo
}

pub fn tenhou(kawa: &Kawa, is_oya: bool, is_tsumo: bool, calls_made: bool) -> bool {
    kawa.0.is_empty() && is_oya && is_tsumo && !calls_made
} 

pub fn chiihou(kawa: &Kawa, is_oya: bool, is_tsumo: bool, calls_made: bool) -> bool {
    kawa.0.is_empty() && !is_oya && is_tsumo && !calls_made
} 