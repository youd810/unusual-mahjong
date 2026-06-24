use bevy::prelude::*;
use crate::core::*;
use crate::states::*;
use crate::scoring::*;
use rand::{RngExt, seq::SliceRandom};


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
pub struct Wall {
    pub tiles: Vec<Tile>,
    pub head: usize,
    pub tail: usize,
    pub dora_count: usize,
    pub rinshan_draws: usize,
    pub rinshan_max: usize,
    pub dice_roll: usize, // add this
}

impl Wall {
    pub fn new(mut tiles: Vec<Tile>, phase: MatchPhase, dice_roll: usize) -> Self {
        tiles.shuffle(&mut rand::rng());
        let rinshan_max = match phase {
            MatchPhase::Yonma => 4,
            MatchPhase::Sanma | MatchPhase::Nima => 8,
        };

        Self {
            tiles,
            head: 0,
            tail: 136 - 14,
            dora_count: 1,
            rinshan_draws: 0,
            rinshan_max,
            dice_roll,
        }
    }

    pub fn draw(&mut self) -> Option<Tile> {
        if self.head >= self.tail { return None; }
        let tile = self.tiles[self.head];
        self.head += 1;
        Some(tile)
    }

    pub fn rinshan_draw(&mut self) -> Option<Tile> {
        if self.rinshan_draws >= self.rinshan_max { return None; }

        // rinshan tiles sit at the very start of the dead wall section
        let idx = self.tiles.len() - 14 + self.rinshan_draws;
        let tile = self.tiles[idx];

        self.rinshan_draws += 1;
        self.tail -= 1; // supplement the dead wall by pushing the boundary backwards
        Some(tile)
    }

    pub fn get_dora_indicators(&self) -> Vec<Tile> {
        let mut dora = Vec::new();
        let base_idx = self.tiles.len() - 14 + self.rinshan_max;
        for i in 0..self.dora_count {
            dora.push(self.tiles[base_idx + (i * 2)]);
        }
        dora
    }

    pub fn get_ura_indicators(&self) -> Vec<Tile> {
        let mut ura = Vec::new();
        let base_idx = self.tiles.len() - 14 + self.rinshan_max;
        for i in 0..self.dora_count {
            ura.push(self.tiles[base_idx + (i * 2) + 1]);
        }
        ura
    }

    pub fn remaining_draws(&self) -> usize {
        self.tail.saturating_sub(self.head)
    }
}

//#[derive(Resource)]
//pub struct DeadWall {
//    pub dora_indicators: Vec<Tile>,
//    pub ura_indicators: Vec<Tile>,
//    pub rinshan_tiles: Vec<Tile>,
//    pub filler_tiles: Vec<Tile>, // The remaining face-down tiles to maintain the 14 count
//}


#[derive(Debug, Clone)]
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


#[derive(Debug, Clone)]
pub enum TochuuType {
    SuufonRenda,
    SuuchaRiichi,
    Suukaikan,
    Sanchahou,
}



#[derive(Debug, Clone)]
pub enum ReplayEvent {
    // match lifecycle
    MatchStart {
        phase: MatchPhase,
        seats: Vec<(Entity, Wind, i32)>,
    },
    RoundStart {
        round: u8,
        honba: u8,
        bakaze: Wind,
        dora_indicator: Tile,
        hands: Vec<(Entity, Vec<Tile>)>,
    },
    RoundEnd {
        reason: RoundEndReason,
        winners: Vec<(Entity, HandResult, u32)>,
        loser: Option<Entity>,
        is_tsumo: bool,
    },
    MatchTransition {
        new_phase: MatchPhase,
        eliminated: Entity,
        new_standings: Vec<(Entity, Wind, i32)>,
    },
    GameOver {
        standings: Vec<(Entity, i32)>,
    },

    // core turn actions
    Draw {
        player: Entity,
        tile: Tile,
    },
    RinshanDraw {
        player: Entity,
        tile: Tile,
    },
    Discard {
        player: Entity,
        tile: Tile,
        is_tsumogiri: bool,
    },

    // calls on opponent discards
    Ron {
        winner: Entity,
        from: Entity,
        result: HandResult,
        payout: u32,
    },
    Pon {
        player: Entity,
        tile: Tile,
        from: Entity,
    },
    Chi {
        player: Entity,
        tile: Tile,
        position: ChiTilePos,
        from: Entity,
    },
    Daiminkan {
        player: Entity,
        tile: Tile,
        from: Entity,
    },

    // self-turn declarations
    Tsumo {
        player: Entity,
        result: HandResult,
        payout: u32,
    },
    Ankan {
        player: Entity,
        tile: Tile,
    },
    Shouminkan {
        player: Entity,
        tile: Tile,
    },
    RiichiDeclared {
        player: Entity,
        tile: Tile,
        is_double: bool,
    },
    Nukidora {
        player: Entity,
        tile: Tile,
    },
    KyuushuKyuuhai {
        player: Entity,
    },

    NagashiMangan {
        player: Entity,
        payout: u32,
    },

    // dora
    DoraRevealed {
        indicator: Tile,
    },

    // round-ending draws
    Ryuukyoku {
        tenpai_players: Vec<Entity>,
    },
    TochuuRyuukyoku {
        reason: TochuuType,
        causers: Vec<Entity>,
    },

    // blackout and cheating
    BlackoutStart {
        duration_secs: f32,
    },
    BlackoutEnd,
    Cheat {
        cheater: Entity,
        target_kawa: Entity,
        tile_taken: Tile,
        tile_left: Tile,
    },
    Accusation {
        accuser: Entity,
        suspect: Entity,
        was_correct: bool,
    },

    // shooting
    ShotFired {
        shooter: Entity,
        target: Entity,
        lethal: bool,
    },
}

#[derive(Resource, Default)]
pub struct ReplayLog {
    pub events: Vec<ReplayEvent>,
}