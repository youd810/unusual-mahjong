use bevy::prelude::*;
use crate::core::*;
use crate::scoring::*;

#[derive(Component)]
pub struct PlayerTag;

#[derive(Component)]
pub struct HumanPlayer;

#[derive(Component, Debug, Clone)]
pub struct BotProfile {
    pub aggressiveness: f32, // defensive <-> agressive
    pub cheat_tendency: f32,
    pub speed: f32, // hand value <-> speed
    pub read: f32,
    pub emotional_invulnerability: f32,
    pub composure: f32, // panic <-> calm, starts at 1.0
}

impl BotProfile {
    pub fn great() -> Self {
        BotProfile {
            aggressiveness: 0.9,
            cheat_tendency: 0.3,
            speed: 0.5,
            read: 0.9,
            emotional_invulnerability: 1.0,
            composure: 1.0,
        }
    }

    pub fn good() -> Self {
        BotProfile {
            aggressiveness: 0.8,
            cheat_tendency: 0.5,
            speed: 0.6,
            read: 0.7,
            emotional_invulnerability: 0.7,
            composure: 1.0,
        }
    }

    pub fn average() -> Self {
        BotProfile {
            aggressiveness: 0.5,
            cheat_tendency: 0.2,
            speed: 0.4,
            read: 0.5,
            emotional_invulnerability: 0.5,
            composure: 1.0,
        }
    }

    pub fn bad() -> Self {
        BotProfile {
            aggressiveness: 0.3,
            cheat_tendency: 0.1,
            speed: 0.3,
            read: 0.3,
            emotional_invulnerability: 0.2,
            composure: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suit { 
    Man, 
    Pin, 
    Sou, 
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetYaku {
    Speed,
    Tanyao,
    Honitsu(Suit),
    Chinitsu(Suit),
    Pairs,
    Kokushi,
    SanshokuDoujun(u8),
    Chanta,
    Junchan,
    Pinfu,
    Ittsuu(Suit),
}

#[derive(Component)]
pub struct BotStrategy {
    pub target: TargetYaku,
}

#[derive(Component)]
pub struct BotCheatIntent {
    pub execute_at: f32,
}

#[derive(Component)]
pub struct BotAccusationIntent {
    pub suspect: Entity,
    pub confidence: f32,
    pub accuse_at: f32,
}


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
pub struct RiichiOption(pub Vec<Tile>);

#[derive(Component)]
pub struct RiichiSelecting;

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

#[derive(Component)]
pub struct DrawnFromRinshan;

#[derive(Component)]
pub struct ForbiddenDiscard(pub Vec<Tile>);

#[derive(Component)]
pub struct DiscardWasCalled;


#[derive(Component)]
pub struct PonDeclared;

#[derive(Component)]
pub struct DaiminkanDeclared;

#[derive(Component)]
pub struct ChiDeclared(pub ChiTilePos);
