use bevy::prelude::*;
use crate::core::*;
use crate::scoring::*;
use rand::RngExt;


#[derive(Resource)]
pub struct GameState {
    pub rounds: u8,
    pub honba: u8,
    pub bakaze: Wind,
    pub bullet: u8,
    pub calls_made: bool,  // ! IMPORTANT: removed after the first call
    pub riichi_points: u32,
    pub pending_kan_dora: bool,
    pub pending_rinshan: bool,
}

#[derive(Resource)]
pub struct CurrentTurn(pub Entity); // id of the current tsumo

// #[derive(Resource)]
// pub struct CallWindowTimer(pub Timer);


#[derive(Resource)]
pub struct Wall(pub Vec<Tile>);

#[derive(Resource)]
pub struct DeadWall {
    pub dora_indicators: Vec<Tile>,
    pub ura_indicators: Vec<Tile>,
    pub rinshan_tiles: Vec<Tile>,
    pub filler_tiles: Vec<Tile>, // The remaining face-down tiles to maintain the 14 count
}


pub enum RoundEndReason {
    OyaWin,             // renchan
    NonOyaWin,
    RyuukyokuOyaTenpai, // renchan
    RyuukyokuOyaNoten,
    TochuuRyuukyoku,
}

#[derive(Resource)]
pub struct RoundResult(pub RoundEndReason);

// TODO: add tochuu causer to tochuu systems
#[derive(Resource)]
pub struct RoundOutcome {
    pub winners: Vec<(Entity, HandResult)>,  // can be multiple for double ron
    pub loser: Option<Entity>,               // None for tsumo
    pub is_tsumo: bool,
    pub tochuu_causer: Vec<Entity>,
}


pub struct Execute {
    pub shooter: Entity,
    pub target: Entity,
}

#[derive(Resource)]
pub struct ExecuteQueue(pub Vec<Execute>);

#[derive(Resource)]
pub struct PendingTargetSelection {
    pub shooter: Entity,
    pub remaining_picks: u8,
}


// ! a call/naki lock. very important!
#[derive(Resource, Default)]
pub struct CallLock(pub bool);


#[derive(Resource)]
pub struct Revolver {
    pub bullet: u8,   
    pub chamber: u8, 
}

impl Revolver {
    pub fn new() -> Self {
        Revolver {
            bullet: rand::rng().random_range(1..=6),
            chamber: 1,
        }
    }

    pub fn pull(&mut self) -> bool {
        let fired = self.chamber == self.bullet;
        if fired {
            *self = Revolver::new();
        } else {
            self.chamber += 1;
        }
        fired
    }
}