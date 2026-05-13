// TODO remaining todos
// Dora counting (not a yaku but affects scoring) (Done?)
// Fu calculation (done?)
// Han → Score conversion table (done?)
// custom yaku and rules later
// ? 途中流局 (done?)
// ! ai behavior (priority: call/naki resolving logic)


// TODO: return options for some of these (no)

mod components;
mod resources;
mod core;
mod scoring;
mod messages;
mod systems;
mod states;
mod yaku;

use bevy::prelude::*;

use resources::*;

use messages::*;
use systems::*;
use states::*;




fn main() {

    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<TurnState>()
        // messages
        .add_message::<DiscardTileMessage>()
        .add_message::<DeclarePonMessage>()
        .add_message::<DeclareChiMessage>()
        .add_message::<DeclareKanMessage>()
        .add_message::<DeclareRiichiMessage>()
        .add_message::<DeclareTsumoMessage>()
        .add_message::<DeclareKyuushuMessage>()
        // setup (first round)
        .add_systems(OnEnter(TurnState::Setup), (
            start_game,
            set_tenpai,
        ).chain())
        // new round
        .add_systems(OnEnter(TurnState::StartNewRound), (
            start_round,
            set_tenpai,
        ).chain())
        // draw
        .add_systems(OnEnter(TurnState::Draw), draw_tile)
        // main phase
        .add_systems(OnEnter(TurnState::MainPhase), (
            tsumo_check,
            riichi_check,
            kyuushu_check,
            ankan_check,
            shouminkan_check,
        ).chain())
        .add_systems(Update, (
            tsumo_ui_system,
            kan_ui_system,
            riichi_ui_system,
            kyuushu_ui_system,
            declare_tsumo,
            declare_riichi,
            declare_kyuushu,
            declare_kan,
            auto_discard_bot,
            discard_tile,
        ).run_if(in_state(TurnState::MainPhase))
        .run_if(not(resource_exists::<RoundResult>)))
        .add_systems(OnExit(TurnState::MainPhase),cleanup_main_phase_options)
        // call window
        .add_systems(OnEnter(TurnState::CallWindow), (
            suufon_renda,
            suucha_riichi,
            suukaikan,
            set_tenpai,
            ron_check,
            pon_check,
            chi_check,
            daiminkan_check,
            |mut commands: Commands| commands.insert_resource(CallLock(false)), // lock reset
        ).chain())
        .add_systems(Update, (
            ron_ui_system,
            pon_ui_system,
            chi_ui_system,
            daiminkan_ui_system,
            declare_ron,
            declare_kan,
            declare_pon,
            declare_chi,
            call_window_timeout,
        ).chain()
        .run_if(in_state(TurnState::CallWindow))
        .run_if(not(resource_exists::<RoundResult>)))
        .add_systems(OnExit(TurnState::CallWindow), (check_ryuukyoku, cleanup_call_options).chain())
        // rinshan draw
        .add_systems(OnEnter(TurnState::RinshanDraw), rinshan_draw)
        // advance & end
        .add_systems(OnEnter(TurnState::AdvanceTurn), next_turn)
        .add_systems(OnEnter(TurnState::RoundEnd), (
            tenpai_payout_system
                .run_if(|result: Res<RoundResult>| matches!(result.0,
                    RoundEndReason::RyuukyokuOyaTenpai | RoundEndReason::RyuukyokuOyaNoten)),
            round_cleanup,
        ).chain())
        // shooting phase
        .add_sub_state::<ExecutionSubState>()
        .add_systems(OnEnter(ExecutionSubState::BuildQueue), build_shot_queue)
        .add_systems(Update, select_targets.run_if(in_state(ExecutionSubState::SelectTargets)))
        .add_systems(OnEnter(ExecutionSubState::Processing), process_shot_queue)
        .run();



    // TODO: logical sorting when player picks up a tile (or not?)
    
}