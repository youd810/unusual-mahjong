// yaku todo
// ?note to self: kan check = open mentsu, unusual yaku check = hand, normal yaku check = decompose result  
// *Yakuman (Limit Hands)
// !Kokushi Musou (Thirteen Orphans) done
// !Suuankou (Four Concealed Triplets) done
// !Daisangen (Big Three Dragons) done
// !Shousuushii (Little Four Winds) done
// !Daisuushii (Big Four Winds) done 
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
// Double Riichi
// 
// *1 Han
// !Tanyao (All Simples) done
// !Iipeikou (One Set of Identical Sequences) done
// !Yakuhai / Fanpai (Value Tiles — seat wind, round wind, dragons) done
// !Riichi done
// Ippatsu
// Menzen Tsumo (Self-draw win with closed hand)
// !Pinfu (No-points hand) done
// !Haitei (Win on last tile from wall) done
// !Houtei (Win on last discard) done
// Rinshan Kaihou (Win after Kan draw)
// Chankan (Robbing a Kan)
// 
// *Special
// !Tenpai/Machi done
// Dora counting (not a yaku but affects scoring)
// Fu calculation
// Han → Score conversion table

// TODO: return options for some of these

use bevy::prelude::*;

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

#[derive(PartialEq, Eq, Clone, PartialOrd, Ord)]
enum Mentsu {
    Jantou(Vec<Tile>),
    Koutsu(Vec<Tile>, bool), // true = closed
    Shuntsu(Vec<Tile>, bool),
    Ankan(Vec<Tile>),
    Daiminkan(Vec<Tile>),
    Shouminkan(Vec<Tile>),
}

enum Winds {
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

use bevy::prelude::*;
use rand::{RngExt, seq::SliceRandom};


#[derive(Resource)]
struct GameState {
    rounds: u8,
    turns: u8,
    bakaze: Winds,
    bullet: u8,
}

#[derive(Resource)]
struct Wall(Vec<Tile>);

// components
#[derive(Component)]
struct PlayerTag;

#[derive(Component)]
struct SeatWind(Winds);

#[derive(Component)]
struct Points(i32);

#[derive(Component)]
struct Hand(Vec<Tile>);

#[derive(Component)]
struct OpenMelds(Vec<Mentsu>);

// markers
#[derive(Component)]
struct Oya;

#[derive(Component)]
struct ClosedHand;

#[derive(Component)]
struct Tenpai;

#[derive(Component)]
struct Riichi {
    turns_since: u8,
}

#[derive(Component)]
struct Alive;





// use or dispose later
struct Player {
    points: i32,
    hand: Vec<Tile>,
    drawn: Option<Tile>,
    
    open_mentsu: Vec<Mentsu>,
    jikaze: Winds,
    is_tenpai: bool,
    is_hand_closed: bool,
    is_riichi: bool,
    turns_since_riichi: u8,
    is_alive: bool,
    aggression: u8,
    defense: u8,
    cheating_inclination: u8, 
}

struct Game {
    rounds: u8,
    turns: u8,
    oya: Player,
    wall: Vec<Tile>,
    bakaze: Winds,
    bullet: u8,
    player_discard: Option<Tile>,
}


impl Player {
    fn tenpai(&mut self, hand: &[Tile]) -> Vec<Tile> { // this should be hand + open mentsu
        let mut waiting_on: Vec<Tile> = vec![];
        for tile in all_tiles() {
            let mut hand_speculated = hand.to_owned();
            hand_speculated.push(tile);
            if !decompose(&hand_speculated).is_empty() {
                self.is_tenpai = true;
                waiting_on.push(tile);
            }
        }
        if waiting_on.is_empty() {
            self.is_tenpai = false;
        }
        waiting_on
    }

    fn tenpai_payout_system(mut query: Query<&mut Points, With<Tenpai>>) {
        for mut player_points in &mut query {
            player_points.0 += 1000;
        }
    }


    fn can_declare_riichi(&mut self, hand: &[Tile]) -> bool {
        !self.tenpai(hand).is_empty() && self.is_hand_closed
    }

    fn declare_riichi(&mut self) {
        self.points -= 1000;
        self.is_riichi = true
    }

    fn remove_tile_from_hand(&mut self, target: &Tile) {
        if let Some(idx) = self.hand.iter().position(|x| x == target) {
            self.hand.remove(idx);
        }
    }
    
    fn can_declare_pon(&mut self, tile: &Tile,) -> bool {
        self.hand.iter().filter(|x| **x == *tile).count() >= 2
    }

    fn declare_pon(&mut self, tile: &Tile,) {
        if self.can_declare_pon(tile) {
            self.open_mentsu.push(Mentsu::Koutsu(vec![*tile; 3], false));
            for _ in 0..2 {
                let idx = self.hand.iter().position(|x| x == tile).unwrap();
                self.hand.remove(idx);
                self.is_hand_closed = false;
            }
        }
    }

    fn can_declare_chi(&mut self, tile: &Tile) -> Vec<ChiTilePos> {
        let mut results = vec![];

        // safe 'unwrap' with if let
        if let (Some(prev), Some(next)) = (previous_tile_sequence(tile), next_tile_sequence(tile))
            && self.hand.contains(&prev) && self.hand.contains(&next) {
                results.push(ChiTilePos::Middle);
        }

        if let Some(next) = next_tile_sequence(tile)
            && let Some(next_next) = next_tile_sequence(&next)
            && self.hand.contains(&next) && self.hand.contains(&next_next) {
                results.push(ChiTilePos::Left);
        }

        if let Some(prev) = previous_tile_sequence(tile)
            && let Some(prev_prev) = previous_tile_sequence(&prev)
            && self.hand.contains(&prev) && self.hand.contains(&prev_prev) {
                results.push(ChiTilePos::Right);
        }

        results
    }

    fn declare_chi(&mut self, tile: &Tile, pos: ChiTilePos){
        let positions = self.can_declare_chi(tile);
        if !positions.is_empty(){
            let pos: ChiTilePos = ChiTilePos::Middle;// choose_chi_pos_or_something(positions);// let the player choose 
            
            match pos {
                ChiTilePos::Middle => {
                    let next = next_tile_sequence(tile).unwrap();
                    let prev = previous_tile_sequence(tile).unwrap();
                    // use the variables as a pointer for removal first b4 moving the value 
                    self.remove_tile_from_hand(&next);
                    self.remove_tile_from_hand(&prev);
                    self.open_mentsu.push(Mentsu::Shuntsu(vec![prev, *tile, next], false));
                    self.is_hand_closed = false;
                },
                ChiTilePos::Left => {
                    let next = next_tile_sequence(tile).unwrap();
                    let next_next = next_tile_sequence(&next).unwrap();
                    self.remove_tile_from_hand(&next);
                    self.remove_tile_from_hand(&next_next);
                    self.open_mentsu.push(Mentsu::Shuntsu(vec![*tile, next, next_next], false));
                    self.is_hand_closed = false;
                },
                ChiTilePos::Right => {
                    let prev = previous_tile_sequence(tile).unwrap();
                    let prev_prev = previous_tile_sequence(&prev).unwrap();
                    self.remove_tile_from_hand(&prev);
                    self.remove_tile_from_hand(&prev_prev);
                    self.open_mentsu.push(Mentsu::Shuntsu(vec![prev_prev, prev, *tile], false));
                    self.is_hand_closed = false;
                },
            }
        }
    }

    fn declare_kan_from_hand(&mut self, tile: &Tile, is_discard: bool) { 
        let count = self.hand.iter().filter(|x| **x == *tile).count();
        if is_discard && count == 3 {
            self.open_mentsu.push(Mentsu::Daiminkan(vec![*tile; 4]));
            self.hand.retain(|x| x != tile);
            self.is_hand_closed = false;
        } 
        else if !is_discard && count == 4 {
            self.open_mentsu.push(Mentsu::Ankan(vec![*tile; 4]));
            self.hand.retain(|x| x != tile);
        }  
    }

    fn declare_kan_from_meld(&mut self, tile: &Tile) {
        for mentsu in &mut self.open_mentsu {
            if let Mentsu::Koutsu(tiles, false) = mentsu && tiles[0] == *tile {
                // deref to mutate
                *mentsu = Mentsu::Shouminkan(vec![*tile; 4]);
                self.hand.retain(|x| x != tile);
                self.is_hand_closed = false;
                break;
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

fn combine_tiles(player: &Player) -> Vec<Tile> {
    let mut result = player.hand.clone();

    for mentsu in &player.open_mentsu {
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



fn tanyao(hand: &[Tile]) -> bool {
    // add is_closed cond
    hand.iter().all(|x| !is_yaochuuhai(x))    
}


fn kokushi_musou(hand: &[Tile]) -> bool {
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


fn iipeikou(results: &[Vec<Mentsu>]) -> bool {
    results.iter().any(|result| {
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
    })
}


fn ryanpeikou(results: &[Vec<Mentsu>]) -> bool {
    results.iter().any(|result| {
        let mut shuntsu: Vec<&Mentsu> = result.iter().filter(|x| matches!(x, Mentsu::Shuntsu(_, true))).collect();

        if shuntsu.len() == 4 {
            shuntsu.sort();
            if shuntsu[0] == shuntsu[1] && shuntsu[2] == shuntsu[3] {
                return true;
            }
        }
        false
    })
}


fn wind_to_honor(wind: &Winds) -> Honor {
    match wind {
        Winds::East => Honor::East,
        Winds::South => Honor::South,
        Winds::West => Honor::West,
        Winds::North => Honor::North,
    }
}


fn yakuhai(player: &Player, results: &[Vec<Mentsu>], bakaze: &Winds) -> u8 {
    results.iter().map(|result| {
        result.iter().filter_map(|mentsu| {
            if let 
                Mentsu::Koutsu(tiles, _)  
                    | Mentsu::Ankan(tiles)  
                    | Mentsu::Daiminkan(tiles)
                    | Mentsu::Shouminkan(tiles) = mentsu {
                match &tiles[0] {
                    Tile::Honor(Honor::Red) => Some(1),
                    Tile::Honor(Honor::Green) => Some(1),
                    Tile::Honor(Honor::White) => Some(1),
                    Tile::Honor(h) if *h == wind_to_honor(&player.jikaze) => Some(1),
                    Tile::Honor(h) if *h == wind_to_honor(bakaze) => Some(1),
                    _ => None,
                }
            } else {
                None
            }
        }).sum::<u8>()
    }).max().unwrap_or(0)
}


fn sanankou(results: &[Vec<Mentsu>]) -> bool {
    results.iter().any(|result| {
        result
            .iter()
            .filter(|mentsu| 
                matches!(mentsu, Mentsu::Koutsu(_, true) | Mentsu::Ankan(_)))
            .count() == 3
    })
}


fn suuankou(results: &[Vec<Mentsu>]) -> bool {
    results.iter().any(|result| {
        result
            .iter()
            .filter(|mentsu| 
                matches!(mentsu, Mentsu::Koutsu(_, true) | Mentsu::Ankan(_)))
            .count() == 4 
    })
}


fn toitoi(results: &[Vec<Mentsu>]) -> bool {
    results.iter().any(|result| {
        result
            .iter()
            .filter(|mentsu| 
                matches!(mentsu, Mentsu::Koutsu(_, _) | Mentsu::Ankan(_) | Mentsu::Daiminkan(_) | Mentsu::Shouminkan(_)))
            .count() == 4 
    })
}


fn daisangen(results: &[Vec<Mentsu>]) -> bool {
    results.iter().any(|result| {
        has_koutsu_or_kan(result, Tile::Honor(Honor::Red))
            && has_koutsu_or_kan(result, Tile::Honor(Honor::Green)) 
            && has_koutsu_or_kan(result, Tile::Honor(Honor::White))
    })
}


fn shousangen(results: &[Vec<Mentsu>]) -> bool {
    results.iter().any(|result| {
        let dragon_kou_or_kan = has_koutsu_or_kan(result, Tile::Honor(Honor::Red)) as u8
            + has_koutsu_or_kan(result, Tile::Honor(Honor::Green)) as u8
            + has_koutsu_or_kan(result, Tile::Honor(Honor::White)) as u8;
        
        let dragon_jantou = has_jantou(result, Tile::Honor(Honor::Red)) as u8 
            + has_jantou(result, Tile::Honor(Honor::Green)) as u8 
            + has_jantou(result, Tile::Honor(Honor::White)) as u8;
        
        dragon_kou_or_kan == 2 && dragon_jantou == 1
    })
}


fn daisuushii(results: &[Vec<Mentsu>]) -> bool {
    results.iter().any(|result| {
        has_koutsu_or_kan(result, Tile::Honor(Honor::East))
            && has_koutsu_or_kan(result, Tile::Honor(Honor::South)) 
            && has_koutsu_or_kan(result, Tile::Honor(Honor::West))
            && has_koutsu_or_kan(result, Tile::Honor(Honor::North))
    })
}

fn shousuushii(results: &[Vec<Mentsu>]) -> bool {
 results.iter().any(|result| {
        let wind_kou_or_kan = has_koutsu_or_kan(result, Tile::Honor(Honor::East)) as u8
            + has_koutsu_or_kan(result, Tile::Honor(Honor::South)) as u8
            + has_koutsu_or_kan(result, Tile::Honor(Honor::West)) as u8
            + has_koutsu_or_kan(result, Tile::Honor(Honor::North)) as u8;
        
        let wind_jantou = has_jantou(result, Tile::Honor(Honor::East)) as u8 
            + has_jantou(result, Tile::Honor(Honor::South)) as u8 
            + has_jantou(result, Tile::Honor(Honor::West)) as u8 
            + has_jantou(result, Tile::Honor(Honor::North)) as u8;
        
        wind_kou_or_kan == 3 && wind_jantou == 1
    })
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


fn chanta(results: &[Vec<Mentsu>]) -> bool {
    results.iter().any(|result| {
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
                    is_terminal(&tiles[0]) || is_honor(&tiles[0])
                }
            }
        })
    })
}


fn junchan(results: &[Vec<Mentsu>]) -> bool {
     results.iter().any(|result| {
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
    })
}


fn sankantsu(player: &Player) -> bool {
    player.open_mentsu.iter().filter(|mentsu|
        matches!(mentsu, Mentsu::Ankan(_) | Mentsu::Daiminkan(_) | Mentsu::Shouminkan(_))).count() == 3 
}


fn suukantsu(player: &Player) -> bool {
    player.open_mentsu.iter().filter(|mentsu|
        matches!(mentsu, Mentsu::Ankan(_) | Mentsu::Daiminkan(_) | Mentsu::Shouminkan(_))).count() == 4 
}


fn chiitoitsu(hand: &[Tile]) -> bool {
    if hand.len() != 14 {return false;}
    let mut i = 0;
    while i < hand.len() - 1 {
        if hand[i] != hand[i + 1] {
            return false;
        }
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


fn ittsuu(results: &[Vec<Mentsu>]) -> bool {
    results.iter().any(|result| {
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
    })
}


fn sanshoku_doujun(results: &[Vec<Mentsu>]) -> bool {
    results.iter().any(|result| {
        for i in 1..=7 {
            let num_match = has_shuntsu(result, Tile::Man(i))
                && has_shuntsu(result, Tile::Pin(i))
                && has_shuntsu(result, Tile::Sou(i));

            if num_match {
                return true;
            }
        } 
        false
    })
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


fn sanshoku_doukou(results: &[Vec<Mentsu>]) -> bool {
    results.iter().any(|result| {
        for i in 1..=9 {
            let color_match =  has_koutsu_or_kan(result, Tile::Man(i))
                && has_koutsu_or_kan(result, Tile::Pin(i))
                && has_koutsu_or_kan(result, Tile::Sou(i));
            
            if color_match {
                return true;
            }
        }
        false
    })
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


fn pinfu(player: &Player, game: &Game, results: &[Vec<Mentsu>], winning_tile: &Tile) -> bool {
    if !player.is_hand_closed {
        return false;
    }
    results.iter().any(|result| {
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
                        Tile::Honor(h) if h == wind_to_honor(&player.jikaze) => false,
                        Tile::Honor(h) if h == wind_to_honor(&game.bakaze) => false,
                        _ => true,
                    };
                }
                _ => {}
            }
        }
        shuntsu_count == 4 && has_ryanmen && has_valid_jantou
    })
}


fn haitei(wall: &Wall, is_tsumo: bool) -> bool {
    wall.0.len() == 14 && is_tsumo
}

fn houtei(wall: &Wall, is_tsumo: bool) -> bool {
    wall.0.len() == 14 && !is_tsumo
}

fn tenhou(game: &GameState, is_oya: bool, is_tsumo: bool) -> bool {
    game.turns == 0 && is_oya && is_tsumo
} 

fn chihou(game: &GameState, is_oya: bool, is_tsumo: bool) -> bool {
    game.turns == 0 && !is_oya && is_tsumo
} 


fn check_win_system(
    query: Query<(&Hand, Has<Oya>, Has<Riichi>)>,
    game_state: Res<GameState>,
) {
    for (hand, is_oya) in &query {
        if tenhou(&game_state, is_oya, true) {
        }
    }
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


fn start_game(mut commands: Commands) {
    let mut wall = vec![];
    for _ in 0..4 {
        wall.extend(all_tiles());
    }
    wall.shuffle(&mut rand::rng());
    
    commands.spawn((
        PlayerTag, 
        Points(25000),
        SeatWind(Winds::East),
        Hand(vec![]),
        OpenMelds(vec![]),
        Alive,
        ClosedHand,
        Oya,
    ));
    commands.insert_resource(
        GameState { 
            rounds: 0, 
            turns: 0, 
            bakaze: Winds::East, 
            bullet: 1,
        }
    );
    commands.insert_resource(
        Wall(wall)
    )
}


fn main() {

    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, start_game)
        .run();




    
    //let mut player1 = Player {
    //    points: 123,
    //    jikaze: Winds::East,
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
    //    bakaze: Winds::East,
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
