#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
enum Tile {
    Man(u8),
    Pin(u8),
    Sou(u8),
    Honor(Honor),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
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
    Koutsu(Vec<Tile>),
    Shuntsu(Vec<Tile>),
}

struct Player {
    points: i32,
    is_tenpai: bool,
    is_hand_closed: bool,
    is_alive: bool,
    aggression: u8,
    defense: u8,
    cheating_inclination: u8, 
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

    // jantou check here

}



fn main() {
    let mut wall = vec![Tile::Sou(1), Tile::Honor(Honor::Red)];

    // logical sorting when player picks up a card
    let mut hand = vec![Tile::Sou(1), Tile::Honor(Honor::Red)];
    hand.sort();
    if let Some(results) = check_win(&hand) {
        if iipeikou(&results) {}  
        if tanyao(&hand) {} 
        if tsuuisou(&hand){}
        if daisangen(&hand){}
        if toitoi(&results) {}
        if honitsu(&hand) {}
        if  {} // other yaku
    
    } else if { // special yaku like kokushi musou or chiitoitsu

    }


    println!("{}", is_terminal(&wall[0]));
}
