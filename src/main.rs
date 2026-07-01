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
mod visuals;

use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

use resources::*;
use messages::*;
use board_systems::*;
use bot_systems::*;
use states::*;
use ui::*;
use visuals::*;


fn main() {

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(visuals::VisualsPlugin)
        .init_resource::<AnimationBusy>()
        .init_resource::<CallLock>()
        .init_state::<TurnState>()
        .add_systems(EguiPrimaryContextPass, (info_display_ui_system).run_if(resource_exists::<Wall>))
        .insert_resource(BlackoutCheckTimer(Timer::from_seconds(0.5, TimerMode::Repeating)))
        .init_resource::<BlackoutTileSelection>()
        .init_resource::<CheatLog>()
        .init_resource::<Omniscience>()
        // replay related
        .add_systems(Update, toggle_vsync)
        .add_systems(OnEnter(TurnState::GameOver), log_game_over)
        // messages
        .add_message::<DiscardTileMessage>()
        .add_message::<DeclarePonMessage>()
        .add_message::<DeclareChiMessage>()
        .add_message::<DeclareKanMessage>()
        .add_message::<DeclareNukidoraMessage>()
        .add_message::<DeclareRiichiMessage>()
        .add_message::<DeclareTsumoMessage>()
        .add_message::<DeclareKyuushuMessage>()
        .add_message::<AccuseCheatMessage>()
        .add_message::<RonDealtMessage>()
        .add_message::<TsumoDealtMessage>()
        .add_message::<PlayerEliminatedMessage>()
        .add_message::<SurvivedShotMessage>()
        // camera
        //.add_systems(Startup, spawn_camera)
        // setup (first round)
        .add_systems(OnEnter(TurnState::Setup), (
            game_cleanup,
            start_game,
            set_tenpai,
        ).chain())
        // new round
        .add_systems(OnEnter(TurnState::StartNewRound), (
            clear_board_visuals_system,
            start_round,
            set_tenpai,
        ).chain())
        // draw
        .add_systems(OnEnter(TurnState::Draw), draw_tile)
        // main phase
        .add_systems(OnEnter(TurnState::MainPhase), (
            |mut lock: ResMut<CallLock>| lock.0 = false,
            clear_temp_furiten,
            tsumo_check,
            riichi_check,
            kyuushu_check,
            ankan_check,
            shouminkan_check,
            nukidora_check,
            setup_bot_think_timer,
        ).chain())
        .add_systems(Update, (
            handle_3d_tile_clicks,
            bot_think_timer_system,
            declare_tsumo,
            declare_riichi,
            declare_kyuushu,
            declare_drawn_kan,
            declare_nukidora,
            bot_main_phase_system,
            bot_discard_system,
            discard_tile,
        ).chain()
        .run_if(in_state(TurnState::MainPhase))
        .run_if(not(resource_exists::<RoundResult>)))
        // main phase ui
        .add_systems(EguiPrimaryContextPass, (
            main_phase_ui_system,
            //human_discard_ui_system,
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
            daiminkan_check, // this should be a priority over pon
            pon_check,
            chi_check,
            |mut lock: ResMut<CallLock>| lock.0 = false, // lock reset
        ).chain())
        .add_systems(Update, (
            bot_call_system,
            declare_ron,
            declare_discarded_kan,
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
        .add_systems(OnEnter(TurnState::Blackout), bot_cheat_decision_system)
        .add_systems(Update, (
            blackout_timer_system,
            bot_cheat_execution_system,
        ).run_if(in_state(TurnState::Blackout)))
        .add_systems(OnExit(TurnState::Blackout), cleanup_blackout) 
        // blackout ui 
        .add_systems(EguiPrimaryContextPass, blackout_ui_system
            .run_if(in_state(TurnState::Blackout)))
        // accusation window
        .add_systems(OnEnter(TurnState::AccusationWindow), bot_accusation_decision_system)
        .add_systems(Update, (
            bot_accusation_execution_system,
            resolve_accusation,
            accusation_window_system,
        ).chain()
        .run_if(in_state(TurnState::AccusationWindow)))
        .add_systems(OnExit(TurnState::AccusationWindow), cleanup_accusation)
        // accusation ui
        .add_systems(EguiPrimaryContextPass, accusation_ui_system
            .run_if(in_state(TurnState::AccusationWindow)))
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
        .add_systems(Update, process_shot_queue.run_if(in_state(ExecutionSubState::Processing)))
        // shooting ui
        .add_systems(EguiPrimaryContextPass, target_selection_ui_system.run_if(in_state(ExecutionSubState::SelectTargets)))
        // tilt
        .add_systems(Update, bot_tilt_system
            .after(process_shot_queue)
            .after(declare_ron)
            .after(declare_tsumo)
        )
        // match phase transition
        .add_systems(OnEnter(TurnState::MatchTransition), match_transition)
        // winner reveal
        .add_systems(EguiPrimaryContextPass,human_dead_menu_ui_system
            .run_if(in_state(TurnState::HumanDeadMenu)))
        // game over ui
        .add_systems(EguiPrimaryContextPass, game_over_ui_system
            .run_if(in_state(TurnState::GameOver)))
        .run();
    
}