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
    pub discarded_by: Entity,    
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