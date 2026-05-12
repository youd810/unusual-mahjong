use bevy::prelude::*;


#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum TurnState {
    #[default]
    Setup,
    StartNewRound,      
    Draw,           
    MainPhase,      
    CallWindow,     
    AdvanceTurn,   
    RinshanDraw,
    RoundEnd,
    Execution,
}


#[derive(SubStates, Default, Debug, Clone, Eq, PartialEq, Hash)]
#[source(TurnState = TurnState::Execution)]
pub enum ExecutionSubState {
    #[default]
    BuildQueue,
    SelectTargets,
    Processing,
}