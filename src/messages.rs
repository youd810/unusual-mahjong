use bevy::prelude::*;
use crate::core::*;
use crate::scoring::*;

#[derive(Message)]
pub struct DeclarePonMessage {
    pub player: Entity,       // gets the specific player (important)
    pub tile: Tile,           
}

#[derive(Message)]
pub struct DeclareChiMessage {
    pub player: Entity,       
    pub tile: Tile,           
    pub pos: ChiTilePos,  
}

#[derive(Message)]
pub struct DeclareKanMessage {
    pub player: Entity,       
    pub tile: Tile,           
    pub is_discard: bool,
}

#[derive(Message)]
pub struct DeclareRiichiMessage {
    pub player: Entity,
    pub tile: Tile,
}


#[derive(Message)]
pub struct DeclareTsumoMessage {
    pub player: Entity,
    pub result: HandResult,
}

#[derive(Message)]
pub struct DeclareKyuushuMessage {
    pub player: Entity,
}


#[derive(Message)]
pub struct DiscardTileMessage {
    pub player: Entity,
    pub tile: Tile,
    pub is_tsumogiri: bool, 
}


#[derive(Message)]
pub struct AccuseCheatMessage {
    pub accuser: Entity,
    pub suspect: Entity,
}

#[derive(Clone)]
pub struct LoserTiltInfo {
    pub player: Entity,
    pub was_tenpai: bool,
    pub was_riichi: bool,
    pub best_han: Option<u8>,
}

#[derive(Message)]
pub struct RonDealtMessage {
    pub winner: Entity,
    pub winning_han: u8,
    pub is_yakuman: bool,
    pub loser: LoserTiltInfo,
}

#[derive(Message)]
pub struct TsumoDealtMessage {
    pub winner: Entity,
    pub winning_han: u8,
    pub is_yakuman: bool,
    pub losers: Vec<LoserTiltInfo>,
}

#[derive(Message)]
pub struct PlayerEliminatedMessage {
    pub victim: Entity,
    pub shooter: Entity,
}

#[derive(Message)]
pub struct SurvivedShotMessage {
    pub survivor: Entity,
    pub shooter: Entity,
}
