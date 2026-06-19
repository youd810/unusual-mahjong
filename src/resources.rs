use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use crate::core::*;
use crate::states::*;
use crate::scoring::*;

use rand::RngExt;
use std::collections::HashMap;


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


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoundEndReason {
    OyaWin, // renchan
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


#[derive(Debug, Clone, Serialize, Deserialize)] 
pub enum TochuuType {
    SuufonRenda,
    SuuchaRiichi,
    Suukaikan,
    Sanchahou,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplayEvent<P> {
    MatchStart {
        phase: MatchPhase,
        seats: Vec<(P, Wind, i32)>,
    },
    RoundStart {
        round: u8,
        honba: u8,
        bakaze: Wind,
        dora_indicator: Tile,
        hands: Vec<(P, Vec<Tile>)>,
    },
    RoundEnd {
        reason: RoundEndReason,
        winners: Vec<(P, HandResult, u32)>,
        loser: Option<P>,
        is_tsumo: bool,
    },
    MatchTransition {
        new_phase: MatchPhase,
        eliminated: P,
        new_standings: Vec<(P, Wind, i32)>,
    },
    GameOver {
        standings: Vec<(P, i32)>,
    },
    Draw {
        player: P,
        tile: Tile,
    },
    RinshanDraw {
        player: P,
        tile: Tile,
    },
    Discard {
        player: P,
        tile: Tile,
        is_tsumogiri: bool,
    },
    Ron {
        winner: P,
        from: P,
        result: HandResult,
        payout: u32,
    },
    Pon {
        player: P,
        tile: Tile,
        from: P,
    },
    Chi {
        player: P,
        tile: Tile,
        position: ChiTilePos,
        from: P,
    },
    Daiminkan {
        player: P,
        tile: Tile,
        from: P,
    },
    Tsumo {
        player: P,
        result: HandResult,
        payout: u32,
    },
    Ankan {
        player: P,
        tile: Tile,
    },
    Shouminkan {
        player: P,
        tile: Tile,
    },
    RiichiDeclared {
        player: P,
        tile: Tile,
        is_double: bool,
    },
    Nukidora {
        player: P,
        tile: Tile,
    },
    KyuushuKyuuhai {
        player: P,
    },
    NagashiMangan {
        player: P,
        payout: u32,
    },
    DoraRevealed {
        indicator: Tile,
    },
    Ryuukyoku {
        tenpai_players: Vec<P>,
    },
    TochuuRyuukyoku {
        reason: TochuuType,
        causers: Vec<P>,
    },
    BlackoutStart {
        duration_secs: f32,
    },
    BlackoutEnd,
    Cheat {
        cheater: P,
        target_kawa: P,
        tile_taken: Tile,
        tile_left: Tile,
    },
    Accusation {
        accuser: P,
        suspect: P,
        was_correct: bool,
    },
    ShotFired {
        shooter: P,
        target: P,
        lethal: bool,
    },
}

impl ReplayEvent<Entity> {
    pub fn to_export(&self, map: &HashMap<Entity, u8>) -> ReplayEvent<u8> {
        // fallback to 255 if an entity somehow slipped past mapping (should never happen)
        let get = |e: &Entity| *map.get(e).unwrap_or(&255);

        match self {
            ReplayEvent::MatchStart { phase, seats } => ReplayEvent::MatchStart {
                phase: *phase,
                seats: seats.iter().map(|(e, w, p)| (get(e), *w, *p)).collect(),
            },
            ReplayEvent::RoundStart { round, honba, bakaze, dora_indicator, hands } => ReplayEvent::RoundStart {
                round: *round,
                honba: *honba,
                bakaze: *bakaze,
                dora_indicator: *dora_indicator,
                hands: hands.iter().map(|(e, t)| (get(e), t.clone())).collect(),
            },
            ReplayEvent::RoundEnd { reason, winners, loser, is_tsumo } => ReplayEvent::RoundEnd {
                reason: reason.clone(),
                winners: winners.iter().map(|(e, r, p)| (get(e), r.clone(), *p)).collect(),
                loser: loser.as_ref().map(get),
                is_tsumo: *is_tsumo,
            },
            ReplayEvent::MatchTransition { new_phase, eliminated, new_standings } => ReplayEvent::MatchTransition {
                new_phase: *new_phase,
                eliminated: get(eliminated),
                new_standings: new_standings.iter().map(|(e, w, p)| (get(e), *w, *p)).collect(),
            },
            ReplayEvent::GameOver { standings } => ReplayEvent::GameOver {
                standings: standings.iter().map(|(e, p)| (get(e), *p)).collect(),
            },
            ReplayEvent::Draw { player, tile } => ReplayEvent::Draw { 
                player: get(player), 
                tile: *tile 
            },
            ReplayEvent::RinshanDraw { player, tile } => ReplayEvent::RinshanDraw { 
                player: get(player), 
                tile: *tile 
            },
            ReplayEvent::Discard { player, tile, is_tsumogiri } => ReplayEvent::Discard { 
                player: get(player), 
                tile: *tile, 
                is_tsumogiri: *is_tsumogiri 
            },
            ReplayEvent::Ron { winner, from, result, payout } => ReplayEvent::Ron { 
                winner: get(winner), 
                from: get(from), 
                result: result.clone(), 
                payout: *payout 
            },
            ReplayEvent::Pon { player, tile, from } => ReplayEvent::Pon { 
                player: get(player), 
                tile: *tile, 
                from: get(from) 
            },
            ReplayEvent::Chi { player, tile, position, from } => ReplayEvent::Chi { 
                player: get(player), 
                tile: *tile, 
                position: *position, 
                from: get(from) 
            },
            ReplayEvent::Daiminkan { player, tile, from } => ReplayEvent::Daiminkan { 
                player: get(player), 
                tile: *tile, 
                from: get(from) 
            },
            ReplayEvent::Tsumo { player, result, payout } => ReplayEvent::Tsumo { 
                player: get(player), 
                result: result.clone(), 
                payout: *payout 
            },
            ReplayEvent::Ankan { player, tile } => ReplayEvent::Ankan { 
                player: get(player), 
                tile: *tile 
            },
            ReplayEvent::Shouminkan { player, tile } => ReplayEvent::Shouminkan { 
                player: get(player), 
                tile: *tile 
            },
            ReplayEvent::RiichiDeclared { player, tile, is_double } => ReplayEvent::RiichiDeclared { 
                player: get(player), 
                tile: *tile, 
                is_double: *is_double 
            },
            ReplayEvent::Nukidora { player, tile } => ReplayEvent::Nukidora { 
                player: get(player), 
                tile: *tile 
            },
            ReplayEvent::KyuushuKyuuhai { player } => ReplayEvent::KyuushuKyuuhai { player: get(player) },
            ReplayEvent::NagashiMangan { player, payout } => ReplayEvent::NagashiMangan { 
                player: get(player), 
                payout: *payout 
            },
            ReplayEvent::DoraRevealed { indicator } => ReplayEvent::DoraRevealed { indicator: *indicator },
            ReplayEvent::Ryuukyoku { tenpai_players } => ReplayEvent::Ryuukyoku { 
                tenpai_players: tenpai_players.iter().map(get).collect() 
            },
            ReplayEvent::TochuuRyuukyoku { reason, causers } => ReplayEvent::TochuuRyuukyoku { 
                reason: reason.clone(), 
                causers: causers.iter().map(get).collect() 
            },
            ReplayEvent::BlackoutStart { duration_secs } => ReplayEvent::BlackoutStart { duration_secs: *duration_secs },
            ReplayEvent::BlackoutEnd => ReplayEvent::BlackoutEnd,
            ReplayEvent::Cheat { cheater, target_kawa, tile_taken, tile_left } => ReplayEvent::Cheat {
                cheater: get(cheater),
                target_kawa: get(target_kawa),
                tile_taken: *tile_taken,
                tile_left: *tile_left,
            },
            ReplayEvent::Accusation { accuser, suspect, was_correct } => ReplayEvent::Accusation {
                accuser: get(accuser),
                suspect: get(suspect),
                was_correct: *was_correct,
            },
            ReplayEvent::ShotFired { shooter, target, lethal } => ReplayEvent::ShotFired {
                shooter: get(shooter),
                target: get(target),
                lethal: *lethal,
            },
        }
    }
}

// for game
#[derive(Resource, Default)]
pub struct ReplayLog {
    pub events: Vec<ReplayEvent<Entity>>,
}

// for serialization
#[derive(Serialize, Deserialize)]
pub struct ExportableReplayLog {
    pub events: Vec<ReplayEvent<u8>>,
}