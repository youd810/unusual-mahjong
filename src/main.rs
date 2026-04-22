// yaku todo
//
// Yakuman (Limit Hands)
// 
// Kokushi Musou (Thirteen Orphans) done
// Suuankou (Four Concealed Triplets) done
// Daisangen (Big Three Dragons)
// Shousuushii (Little Four Winds)
// Daisuushii (Big Four Winds)
// Tsuuiisou (All Honors) done
// Chinroutou (All Terminals)
// Ryuuiisou (All Green)
// Chuuren Poutou (Nine Gates)
// Suukantsu (Four Kans)
// Tenhou (Heavenly Win — dealer wins on first draw)
// Chiihou (Earthly Win — non-dealer wins on first draw)
// 
// 6 Han
// 
// Chinitsu (Full Flush)
// 
// 3 Han
// 
// Honitsu (Half Flush)
// Ryanpeikou (Two Sets of Identical Sequences)
// Junchan (All sets contain terminals)
// 
// 2 Han
// 
// Chanta (All sets contain terminals or honors)
// Sanshoku Doujun (Three Color Straight) 
// Sanshoku Doukou (Three Color Triplets)
// Ittsu (Straight 1-9)
// Toitoi (All Triplets)
// Sanankou (Three Concealed Triplets) done
// Shousangen (Little Three Dragons)
// Honroutou (All Terminals and Honors)
// Chiitoitsu (Seven Pairs) done
// Sankantsu (Three Kans)
// Double Riichi
// 
// 1 Han
// 
// Tanyao (All Simples) needs closed hand check
// Iipeikou (One Set of Identical Sequences) done
// Yakuhai / Fanpai (Value Tiles — seat wind, round wind, dragons) done
// Riichi
// Ippatsu
// Menzen Tsumo (Self-draw win with closed hand)
// Pinfu (No-points hand)
// Haitei (Win on last tile from wall)
// Houtei (Win on last discard)
// Rinshan Kaihou (Win after Kan draw)
// Chankan (Robbing a Kan)
// 
// Special
// 
// Tenpai/Machi done
// Dora counting (not a yaku but affects scoring)
// Fu calculation
// Han → Score conversion table

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Tile {
    Man(u8),
    Pin(u8),
    Sou(u8),
    Honor(Honor),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Honor {
    White,
    Red,
    Green,
    North,
    West,
    East,
    South,
}

#[derive(PartialEq, Eq, Clone)]
enum Mentsu {
    Jantou(Vec<Tile>),
    Koutsu(Vec<Tile>, bool), // true = closed
    Shuntsu(Vec<Tile>, bool),
    Kantsu(Kantsu),
}

#[derive(PartialEq, Eq, Clone)]
enum Kantsu {
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

struct Player {
    points: i32,
    hand: Vec<Tile>,
    open_mentsu: Vec<Mentsu>,
    jikaze: Winds,
    is_tenpai: bool,
    // is_hand_closed: bool,
    is_alive: bool,
    aggression: u8,
    defense: u8,
    cheating_inclination: u8, 
}

struct Game {
    rounds: u8,
    bakaze: Winds,
    bullet: u8,
}

#[derive(PartialEq, Eq)]
enum ChiTilePos { // tile drawn/discarded
    Left,  
    Middle, 
    Right,  
}

impl Player {
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
            self.open_mentsu.push(Mentsu::Koutsu(vec![tile.clone(); 3], false));
            for _ in 0..2 {
                let idx = self.hand.iter().position(|x| x == tile).unwrap();
                self.hand.remove(idx);
            }
        }
    }

    fn can_declare_chi(&mut self, tile: &Tile) -> Vec<ChiTilePos> {
        let mut results = vec![];

        // safe 'unwrap' with if let
        if let (Some(prev), Some(next)) = (previous_tile_sequence(tile), next_tile_sequence(tile)) {
            if self.hand.contains(&prev) && self.hand.contains(&next) {
                results.push(ChiTilePos::Middle);
            }
        }

        if let Some(next) = next_tile_sequence(tile) {
            if let Some(next_next) = next_tile_sequence(&next) {
                if self.hand.contains(&next) && self.hand.contains(&next_next) {
                    results.push(ChiTilePos::Left);
                }
            }
        }

        if let Some(prev) = previous_tile_sequence(tile) {
            if let Some(prev_prev) = previous_tile_sequence(&prev) {
                if self.hand.contains(&prev) && self.hand.contains(&prev_prev) {
                    results.push(ChiTilePos::Right);
                }
            }
        }

        results
    }

    fn declare_chi(&mut self, tile: &Tile, pos: ChiTilePos){
        let positions = self.can_declare_chi(tile);
        if !positions.is_empty(){
            let pos: ChiTilePos = choose_chi_pos_or_something(positions);// let the player choose 
            
            match pos {
                ChiTilePos::Middle => {
                    let next = next_tile_sequence(tile).unwrap();
                    let prev = previous_tile_sequence(tile).unwrap();
                    // use the variables as a pointer for removal first b4 moving the value 
                    self.remove_tile_from_hand(&next);
                    self.remove_tile_from_hand(&prev);
                    self.open_mentsu.push(Mentsu::Shuntsu(vec![prev, tile.clone(), next], false));
                    
                },
                ChiTilePos::Left => {
                    let next = next_tile_sequence(tile).unwrap();
                    let next_next = next_tile_sequence(&next).unwrap();
                    self.remove_tile_from_hand(&next);
                    self.remove_tile_from_hand(&next_next);
                    self.open_mentsu.push(Mentsu::Shuntsu(vec![tile.clone(), next, next_next], false));
                    
                },
                ChiTilePos::Right => {
                    let prev = previous_tile_sequence(tile).unwrap();
                    let prev_prev = previous_tile_sequence(&prev).unwrap();
                    self.remove_tile_from_hand(&prev);
                    self.remove_tile_from_hand(&prev_prev);
                    self.open_mentsu.push(Mentsu::Shuntsu(vec![prev_prev, prev, tile.clone()], false));
                },
            }
        }
    }

    fn declare_kan_from_hand(&mut self, tile: &Tile, is_discard: bool) { 
        let count = self.hand.iter().filter(|x| **x == *tile).count();
        if is_discard && count == 3 {
            self.open_mentsu.push(Mentsu::Kantsu(Kantsu::Daiminkan(vec![tile.clone(); 4])));
            self.hand.retain(|x| x != tile);
        } 
        else if !is_discard && count == 4 {
            self.open_mentsu.push(Mentsu::Kantsu(Kantsu::Ankan(vec![tile.clone(); 4])));
            self.hand.retain(|x| x != tile);
        }  
    }

    fn declare_kan_from_meld(&mut self, tile: &Tile) {
        for mentsu in &mut self.open_mentsu {
            if let Mentsu::Koutsu(tiles, false) = mentsu {
                if tiles[0] == *tile {
                    // deref to mutate
                    *mentsu = Mentsu::Kantsu(Kantsu::Shouminkan(vec![tile.clone(); 4]));
                    self.hand.retain(|x| x != tile);
                    break;
                }
            }
        }
    } 
}

fn is_terminal(tile: &Tile) -> bool {
    match tile {
        Tile::Sou(1 | 9) | Tile::Pin(1 | 9) | Tile::Man(1 | 9) => true,
        _ => false,
    }
}

fn is_honor(tile: &Tile) -> bool {
    matches!(tile, Tile::Honor(_))
}

fn is_yaochuhai(tile: &Tile) -> bool {
    is_terminal(tile) || is_honor(tile)
}

fn check_win(hand: &Vec<Tile>) -> Option<Vec<Vec<Mentsu>>> {
    let results = decompose(hand);
    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}

fn tanyao(hand: &Vec<Tile>) -> bool {
    // add is_closed cond
    hand.iter().all(|x| !is_yaochuhai(x))    
}

fn kokushi_musou(hand: &Vec<Tile>) -> bool {
    if hand.iter().all(|x| is_yaochuhai(x)) {
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
        

fn tsuuisou(hand: &Vec<Tile>) -> bool {
    hand.iter().all(|x| is_honor(x))
}

fn iipeikou(results: &Vec<Vec<Mentsu>>) -> bool {
    results.iter().any(|result| {
        let shuntsu: Vec<&Mentsu> = result.iter().filter(|x| matches!(x, Mentsu::Shuntsu(_))).collect();
 
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

fn wind_to_honor(wind: &Winds) -> Honor {
    match wind {
        Winds::East => Honor::East,
        Winds::South => Honor::South,
        Winds::West => Honor::West,
        Winds::North => Honor::North,
    }
}

fn yakuhai(player: &Player, results: &Vec<Vec<Mentsu>>, bakaze: &Winds) -> u8 {
    results.iter().map(|result| {
        result.iter().filter_map(|mentsu| {
            if let Mentsu::Koutsu(tiles, _) | Mentsu::Kantsu(tiles, _) = mentsu {
                match &tiles[0] {
                    Tile::Honor(Honor::Red) => Some(1),
                    Tile::Honor(Honor::Green) => Some(1),
                    Tile::Honor(Honor::White) => Some(1),
                    Tile::Honor(h) if *h == wind_to_honor(&player.jikaze) => Some(1),
                    Tile::Honor(h) if *h == wind_to_honor(&bakaze) => Some(1),
                    _ => None,
                }
            } else {
                None
            }
        }).sum::<u8>()
    }).max().unwrap_or(0)
}

fn sanankou(results: &Vec<Vec<Mentsu>>) -> bool {
    results.iter().any(|result| {
        result.iter().filter(|mentsu| matches!(mentsu, Mentsu::Koutsu(_, true))).count() == 3
    })
}

fn suuankou(results: &Vec<Vec<Mentsu>>) -> bool {
    results.iter().any(|result| {
        result.iter().filter(|mentsu| matches!(mentsu, Mentsu::Koutsu(_, true))).count() == 4 
    })
}

fn chiitoitsu(hand: &Vec<Tile>) -> bool {
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

fn tenpai(&mut self, hand: &Vec<Tile>) -> Vec<Tile> {
    let mut waiting_on: Vec<Tile> = vec![];
    for tile in all_tiles() {
        let mut hand_speculated = hand.clone();
        hand_speculated.push(tile);
        if !decompose(&hand_speculated).is_empty() {
            if self.is_tenpai == false {
                self.is_tenpai = true;
            }
            waiting_on.push(tile);
        }
    }
    waiting_on
}

// one hand can return different mentsu varations
// example: [sou1, sou1, sou1 sou2, sou2, sou2, sou3, sou3, sou3] 
// can return [shuntsu, shuntsu, shuntsu] or [koutsu, koutsu, koutsu]
// so the final result is a vector of those two
fn decompose(tiles: &Vec<Tile>) -> Vec<Vec<Mentsu>> {
    let mut results = vec![];

    for i in 0..tiles.len() - 1 {
        if tiles[i] == tiles[i+1] {
            if i > 0 && tiles[i] == tiles[i-1]{
                continue;
            }
            let pair = Mentsu::Jantou(vec![tiles[i].clone(), tiles[i+1].clone()]);
            let mut remaining = tiles.clone();
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


fn find_mentsu(remaining: &Vec<Tile>, current: Vec<Mentsu>, results: &mut Vec<Vec<Mentsu>>) {
    if remaining.is_empty() {
        results.push(current);
        return;
    }

    // koutsu check
    if remaining.len() >= 3 && remaining[0] == remaining[1] && remaining[0] == remaining[2] {
        let koutsu_group = Mentsu::Koutsu(vec![remaining[0].clone(), remaining[1].clone(), remaining[2].clone()]);
        let mut new_remaining = remaining.clone();
        for _ in 0..3 {
            new_remaining.remove(0);
        }
        let mut new_current = current.clone();
        new_current.push(koutsu_group);
        find_mentsu(&new_remaining, new_current, results);
    }

    // shuntsu check
    if let Some(second) = next_tile_sequence(&remaining[0]) {
        if let Some(third) = next_tile_sequence(&second) {
            if let Some(second_seq) = remaining.iter().skip(1).position(|x| *x == second).map(|i| i + 1) {
                if let Some(third_seq) = remaining.iter().skip(second_seq + 1).position(|x| *x == third).map(|i| i + second_seq + 1) {
                    let shuntsu_group = Mentsu::Shuntsu(vec![remaining[0].clone(), remaining[second_seq].clone(), remaining[third_seq].clone()]);
                    let mut new_remaining = remaining.clone();
                    // starts from the highest index
                    for idx in vec![third_seq, second_seq, 0] {
                        new_remaining.remove(idx);
                     }
                    let mut new_current = current.clone();
                    new_current.push(shuntsu_group);
                    find_mentsu(&new_remaining, new_current, results);
                }
            }
        }
    }
}



fn main() {
    let player1 = Player {
        points: 123,
        jikaze: Winds::East,
        hand: vec![],
        open_mentsu: vec![],
        is_tenpai: false,
        is_alive: true,
        aggression: 10,
        defense: 10,
        cheating_inclination: 10, 
    };

    let game = Game {
        rounds: 3,
        bakaze: Winds::East,
        bullet: 123,
    };

    let mut wall = vec![Tile::Sou(1), Tile::Honor(Honor::Red)];

    // logical sorting when player picks up a card
    let mut hand = vec![Tile::Sou(1), Tile::Honor(Honor::Red)];
    hand.sort();
    if let Some(results) = check_win(&hand) {
        // yakuman
        if tsuuisou(&hand){}
        if daisangen(&hand){}

        // regular yaku with unusual pattern
        if chiitoitsu(&hand) {}

        // regular yaku
        if iipeikou(&results) {}  
        if tanyao(&hand) {} // closed
        
        if yakuhai(&player1, &results, &game.bakaze) > 0 {} // open
        
        if toitoi(&results) {}
        if honitsu(&hand) {}
        if  {} // other yaku
    
    } else if { // special yaku like kokushi musou or chiitoitsu

    }


    println!("{}", is_terminal(&wall[0]));
}
