use bevy::prelude::*;
use crate::core::*;
use crate::scoring::*;

#[derive(Component)]
pub struct PlayerTag;

#[derive(Component)]
pub struct Jikaze(pub Wind);

#[derive(Component)]
pub struct Points(pub i32);

#[derive(Component)]
pub struct Hand(pub Vec<Tile>);

impl Hand {
    pub fn remove_tile_from_hand(&mut self, target: &Tile) {
        if let Some(idx) = self.0.iter().position(|x| x == target) {
            self.0.remove(idx);
        }
    }
}


#[derive(Component)]
pub struct OpenMentsu(pub Vec<Mentsu>);

// markers
#[derive(Component)]
pub struct Oya;

#[derive(Component)]
pub struct ClosedHand;

#[derive(Component)]
pub struct Tenpai(pub Vec<Tile>);

#[derive(Component)]
pub struct Furiten;

#[derive(Component)]
pub struct Riichi {
    pub turns_since: u8,
}

#[derive(Component)]
pub struct Ippatsu;

#[derive(Component)]
pub struct DoubleRiichi;

#[derive(Component)]
pub struct Alive;

#[derive(Component)]
pub struct Kawa(pub Vec<Tile>);

// ! a component to an entity (each tile is its own entity)
#[derive(Component)] 
pub struct DiscardedTile(pub Tile);

#[derive(Component)]
pub struct DiscardedBy(pub Entity);

#[derive(Component)]
pub struct CurrentDiscard;

#[derive(Component)]
pub struct PonOption(pub Tile);

#[derive(Component)]
pub struct ChiOption {
    pub tile: Tile,
    pub positions: Vec<ChiTilePos>,
}

#[derive(Component)]
pub struct DaiminkanOption(pub Tile);

// main phase
#[derive(Component)]
pub struct AnkanOption(pub Vec<Tile>);

#[derive(Component)]
pub struct ShouminkanOption(pub Vec<Tile>);

#[derive(Component)]
pub struct RiichiOption;

#[derive(Component)]
pub struct KyuushuOption;

// for ron
#[derive(Component)]
pub struct Chankan;

#[derive(Component)]
pub struct RonOption {
    pub discarded_by: Entity,
    pub result: HandResult,
}

#[derive(Component)]
pub struct RonDeclared;

#[derive(Component)]
pub struct TsumoOption {
    pub result: HandResult,
}

#[derive(Component)]
pub struct DrawnTile(pub Tile);