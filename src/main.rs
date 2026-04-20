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

fn tanyao(hand: &Vec<Tile>) -> bool {
    hand.iter().all(|x| !is_yaochuhai(x))    
}

// will deal with this later
//fn iipeikou(hand: &Vec<Tile>) -> bool {
//    let grouped = vec![hand[..3].to_vec(), hand[3..6].to_vec(), hand[6..9].to_vec(), hand[9..12].to_vec(), hand[12..].to_vec()];
//    let mut flag = false;
//    for i in 0..grouped.len() -1 {
//        if i == 0 {
//            continue;
//        }
//        if shuntsu(&grouped[i]) && shuntsu(&grouped[i-1]) &&  grouped[i] == grouped[i-1] {
//            flag = true
//        }
//    }
//    flag
//}

fn decompose(tiles: &Vec<Tile>) -> Vec<Vec<Vec<Tile>>> {
    let mut results = vec![];
    let mut sorted = tiles.clone();
    sorted.sort();

    

    for i in 0..sorted.len() - 1 {
        if sorted[i] == sorted[i+1] {
            let pair = vec![sorted[i].clone(), sorted[i+1].clone()];
            let mut remaining = sorted.clone();
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


fn find_mentsu(remaining: &Vec<Tile>, current: Vec<Vec<Tile>>, results: &mut Vec<Vec<Vec<Tile>>>) {
    if remaining.is_empty() {
        results.push(current);
        return;
    }

    // koutsu check
    if remaining.len() >= 3 && remaining[0] == remaining[1] && remaining[0] == remaining[2] {
        let koutsu_group = vec![remaining[0].clone(), remaining[1].clone(), remaining[2].clone()];
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
                    let shuntsu_group = vec![remaining[0].clone(), remaining[second_seq].clone(), remaining[third_seq].clone()];
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
    let mut wall = vec![Tile::Sou(1), Tile::Honor(Honor::Red)];

    println!("{}", is_terminal(&wall[0]));
}
