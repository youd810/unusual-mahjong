use bevy::prelude::*;
use crate::core::*;
use crate::states::*;
use crate::scoring::*;
use rand::RngExt;


#[derive(Resource)]
pub struct GameState {
    pub match_phase: MatchPhase,
    pub rounds: u8,
    pub honba: u8,
    pub bakaze: Wind,
    pub bullet: u8,
    pub calls_made: bool,  // ! IMPORTANT: removed after the first call
    pub riichi_points: u32,
    pub pending_kan_dora: bool,
    pub pending_rinshan: bool,
}

// if match phase ended naturally
#[derive(Resource)]
pub struct MatchEndPending;

#[derive(Resource, Default)]
pub struct SimulationMode;

#[derive(Resource)]
pub struct CurrentTurn(pub Entity); // id of the current tsumo


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
    pub winners: Vec<(Entity, HandResult, u32)>,  // can be multiple for double ron
    pub loser: Option<Entity>,               // None for tsumo
    pub is_tsumo: bool,
    pub tochuu_causer: Vec<Entity>,
}

#[derive(Resource)]
pub struct RoundSummary {
    pub reason_text: String,
    pub winners: Vec<(Entity, HandResult, u32)>,
    pub loser: Option<Entity>,
    pub is_tsumo: bool,
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

#[derive(Resource)]
pub struct KawaSnapshot {
    pub all_kawa: Vec<(Entity, Vec<Tile>)>
}

#[derive(Resource)]
pub struct PreBlackoutState(pub TurnState);

#[derive(Resource)]
pub struct BlackoutCheckTimer(pub Timer);

#[derive(Resource)]
pub struct BlackoutTimer(pub Timer);

#[derive(Resource)]
pub struct AccusationTimer(pub Timer);

#[derive(Resource, Default)]
pub struct CheatLog(pub Vec<CheatEntry>);

pub struct CheatEntry {
    pub cheater: Entity,
    pub target_kawa: Entity,
    pub tile_taken: Tile,
    pub tile_left: Tile,
}

#[derive(Default, Clone)]
pub enum SelectedSource {
    #[default]
    None,
    Hand(usize, Tile),
    Drawn(Tile),
}

#[derive(Resource, Default)]
pub struct BlackoutTileSelection {
     pub selected: SelectedSource,
}

#[derive(Resource, Default)]
pub struct Omniscience(pub bool);