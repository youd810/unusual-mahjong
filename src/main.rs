// TODO remaining todos
// custom yaku and rules later
// ! ai behavior (priority: call/naki resolving logic)


mod components;
mod resources;
mod core;
mod scoring;
mod messages;
mod board_systems;
mod bot_systems;
mod states;
mod yaku;
mod ui;

use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

use resources::*;
use messages::*;
use board_systems::*;
use bot_systems::*;
use states::*;
use ui::*;


fn main() {

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .init_resource::<CallLock>()
        .init_state::<TurnState>()
        .add_systems(EguiPrimaryContextPass, (info_display_ui_system).run_if(resource_exists::<Wall>))
        .insert_resource(BlackoutCheckTimer(Timer::from_seconds(0.5, TimerMode::Repeating)))
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
            |mut lock: ResMut<CallLock>| lock.0 = false,
            tsumo_check,
            riichi_check,
            kyuushu_check,
            ankan_check,
            shouminkan_check,
        ).chain())
        .add_systems(Update, (
            declare_tsumo,
            declare_riichi,
            declare_kyuushu,
            declare_kan,
            bot_discard_system,
            discard_tile,
        ).run_if(in_state(TurnState::MainPhase))
        .run_if(not(resource_exists::<RoundResult>)))
        // main phase ui
        .add_systems(EguiPrimaryContextPass, (
            main_phase_ui_system,
            human_discard_ui_system,
            debug_ui_system,
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
            |mut lock: ResMut<CallLock>| lock.0 = false, // lock reset
        ).chain())
        .add_systems(Update, (
            bot_call_system,
            declare_ron,
            declare_kan,
            declare_pon,
            declare_chi,
            auto_advance_call_window,
        ).chain()
        .run_if(in_state(TurnState::CallWindow))
        .run_if(not(resource_exists::<RoundResult>)))
        // call window ui
        .add_systems(EguiPrimaryContextPass, (
            call_window_ui_system,
        ).run_if(in_state(TurnState::CallWindow))
        .run_if(not(resource_exists::<RoundResult>)))
        .add_systems(OnExit(TurnState::CallWindow), cleanup_call_options)
        // rinshan draw
        .add_systems(OnEnter(TurnState::RinshanDraw), rinshan_draw)
        // blackout
        .add_systems(Update, blackout_check_system
            .run_if(in_state(TurnState::Draw)
                .or(in_state(TurnState::MainPhase))
                .or(in_state(TurnState::CallWindow))
                .or(in_state(TurnState::AdvanceTurn))
                .or(in_state(TurnState::RinshanDraw))
            )
        )
        // advance turn
        .add_systems(OnEnter(TurnState::AdvanceTurn), (check_ryuukyoku, next_turn).chain())
        // round end
        .add_systems(OnEnter(TurnState::RoundEnd), (
            tenpai_payout_system
                .run_if(|result: Res<RoundResult>| matches!(result.0,
                    RoundEndReason::RyuukyokuOyaTenpai | RoundEndReason::RyuukyokuOyaNoten)),
            build_round_summary,
        ).chain())
        .add_systems(EguiPrimaryContextPass, round_end_ui_system
            .run_if(in_state(TurnState::RoundEnd)))
        .add_systems(Update, round_cleanup
            .run_if(in_state(TurnState::RoundEnd))
            .run_if(not(resource_exists::<RoundSummary>)))
        // shooting phase
        .add_sub_state::<ExecutionSubState>()
        .add_systems(OnEnter(ExecutionSubState::BuildQueue), build_shot_queue)
        .add_systems(OnEnter(ExecutionSubState::Processing), process_shot_queue)
        // shooting ui
        .add_systems(EguiPrimaryContextPass, target_selection_ui_system.run_if(in_state(ExecutionSubState::SelectTargets)))
        .run();
    
}