// TODO: create a function + ui to instantly form a yaku for testing

use crate::core::*;
use crate::components::*;
use crate::resources::*;
use crate::messages::*;
use crate::states::*;
use crate::scoring::*;
use crate::yaku::nagashi_mangan;
use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow};
use rand::{RngExt, seq::SliceRandom};

use std::collections::HashMap;
use std::fs;

pub fn blackout_check_system(
    time: Res<Time>,
    mut timer: ResMut<BlackoutCheckTimer>,
    state: Res<State<TurnState>>,
    query: Query<(Entity, &Kawa)>,
    sim: Option<Res<SimulationMode>>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands,
) {
    if sim.is_some() {
        return;
    }

    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    if rand::random::<f32>() > 0.001 {
        return;
    }

    let all_kawa = query.iter()
        .map(|(e, kawa)| (e, kawa.0.clone()))
        .collect();

    let duration = rand::rng().random_range(1.0..=5.0);

    commands.insert_resource(KawaSnapshot { all_kawa });
    commands.insert_resource(PreBlackoutState(state.get().clone()));
    commands.insert_resource(CheatLog::default());
    commands.insert_resource(BlackoutTimer(Timer::from_seconds(
        duration,
        TimerMode::Once,
    )));

    if let Some(ref mut log) = replay_log {
        log.events.push(ReplayEvent::BlackoutStart {
            duration_secs: duration,
        });
    }

    next_state.set(TurnState::Blackout);
}


pub fn blackout_timer_system(
    time: Res<Time>,
    mut timer: ResMut<BlackoutTimer>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut commands: Commands,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        commands.insert_resource(AccusationTimer(
            Timer::from_seconds(5.0, TimerMode::Once)
        ));
        next_state.set(TurnState::AccusationWindow);
    }
}


pub fn cleanup_blackout(
    mut commands: Commands,
    mut selection: ResMut<BlackoutTileSelection>,
    mut replay_log: Option<ResMut<ReplayLog>>,
) {
    commands.remove_resource::<BlackoutTimer>();
    selection.selected = SelectedSource::None;

    if let Some(ref mut log) = replay_log {
        log.events.push(ReplayEvent::BlackoutEnd);
    }
}


pub fn accusation_window_system(
    time: Res<Time>,
    mut timer: ResMut<AccusationTimer>,
    pre_blackout: Res<PreBlackoutState>,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        let return_state = pre_blackout.0.clone();
        next_state.set(return_state);
    }
}


pub fn resolve_accusation(
    mut messages: MessageReader<AccuseCheatMessage>,
    cheat_log: Res<CheatLog>,
    human_query: Query<Has<HumanPlayer>>,
    pre_blackout: Res<PreBlackoutState>,
    game: Res<GameState>,
    mut revolver: ResMut<Revolver>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut eliminated_writer: MessageWriter<PlayerEliminatedMessage>,
    mut survived_writer: MessageWriter<SurvivedShotMessage>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands,
) {
    let Some(message) = messages.read().next() else { return };

    let suspect_cheated = cheat_log.0.iter()
        .any(|entry| entry.cheater == message.suspect);

    if let Some(ref mut log) = replay_log {
        log.events.push(ReplayEvent::Accusation {
            accuser: message.accuser,
            suspect: message.suspect,
            was_correct: suspect_cheated,
        });
    }

    let (shooter, target) = if suspect_cheated {
        println!("{} correctly accused {}!", message.accuser, message.suspect);
        (message.accuser, message.suspect)
    } else {
        println!("{} falsely accused {}!", message.accuser, message.suspect);
        (message.suspect, message.accuser)
    };

    let is_human = human_query.get(target).unwrap_or(false);

    if revolver.pull() {
        println!("BANG! {} is eliminated!", target);

        eliminated_writer.write(PlayerEliminatedMessage {
            victim: target,
            shooter,
        });

        commands.entity(target).remove::<Alive>();
        commands.entity(target).remove::<Hand>();
        commands.entity(target).remove::<OpenMentsu>();
        commands.entity(target).remove::<Kawa>();
        commands.entity(target).remove::<ClosedHand>();
        commands.entity(target).remove::<Tenpai>();
        commands.entity(target).remove::<Riichi>();
        commands.entity(target).remove::<Ippatsu>();
        commands.entity(target).remove::<DoubleRiichi>();
        commands.entity(target).remove::<Furiten>();
        commands.entity(target).remove::<DrawnTile>();

        if is_human {
            println!("ゲーム終了\nYou died.");
        }

        match game.match_phase {
            MatchPhase::Nima => next_state.set(TurnState::GameOver),
            _ => next_state.set(TurnState::MatchTransition),
        }
        return;

    } else {
        println!("*click* {} survives.", target);

        survived_writer.write(SurvivedShotMessage {
            survivor: target,
            shooter,
        });
    }

    let return_state = pre_blackout.0.clone();
    next_state.set(return_state);
}


pub fn cleanup_accusation(
    mut commands: Commands,
    query: Query<Entity, With<BotAccusationIntent>>,
) {
    commands.remove_resource::<AccusationTimer>();
    commands.remove_resource::<KawaSnapshot>();
    commands.remove_resource::<PreBlackoutState>();
    commands.remove_resource::<CheatLog>();

    for entity in query {
        commands.entity(entity).remove::<BotAccusationIntent>();
    }
}


pub fn build_shot_queue(
    outcome: Res<RoundOutcome>,
    oya_query: Query<Entity, With<Oya>>,
    mut commands: Commands,
    mut next_step: ResMut<NextState<ExecutionSubState>>,
) {

    let mut queue: Vec<Execute> = vec![];
    let mut needs_selection = false;
    let mut selection_shooter = Entity::PLACEHOLDER;
    let mut selection_picks: u8 = 0;
    let oya = oya_query.single().unwrap();

    // win-based shots
    for (winner, result, _) in &outcome.winners {
        let shots = result.shot_count_from_result();

        if shots > 0 {
            if outcome.is_tsumo {
                needs_selection = true;
                selection_shooter = *winner;
                selection_picks = shots;
            } else if let Some(loser) = outcome.loser {
                for _ in 0..shots {
                    queue.push(Execute { shooter: *winner, target: loser });
                }
            }
        }
    }

    // low han self-shot
    for (winner, result, _) in &outcome.winners {
        if result.is_low_han() {
            queue.push(Execute { shooter: oya, target: *winner });
        }
    }

    // tochuu ryuukyoku
    if !outcome.tochuu_causer.is_empty() {
        let shooter = outcome.loser.unwrap_or(oya);
        for target in &outcome.tochuu_causer {
            queue.push(Execute { shooter, target: *target });
        }
    }

    commands.insert_resource(ExecuteQueue(queue));

    if needs_selection {
        commands.insert_resource(PendingTargetSelection {
            shooter: selection_shooter,
            remaining_picks: selection_picks,
        });
        next_step.set(ExecutionSubState::SelectTargets);
    } else {
        next_step.set(ExecutionSubState::Processing);
    }
}


pub fn process_shot_queue(
    mut queue_opt: Option<ResMut<ExecuteQueue>>,
    mut revolver: ResMut<Revolver>,
    game: Res<GameState>,
    human_query: Query<Has<HumanPlayer>>,
    mut eliminated_writer: MessageWriter<PlayerEliminatedMessage>,
    mut survived_writer: MessageWriter<SurvivedShotMessage>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    let Some(mut queue) = queue_opt else { return };

    for shot in queue.0.drain(..) {
        let is_human = human_query.get(shot.target).unwrap_or(false);
        println!("{} shoots at {} (chamber {}/{})",
            shot.shooter, shot.target, revolver.chamber, revolver.bullet);

        let is_lethal = revolver.pull();

        if let Some(ref mut log) = replay_log {
            log.events.push(ReplayEvent::ShotFired {
                shooter: shot.shooter,
                target: shot.target,
                lethal: is_lethal,
            });
        }

        if is_lethal {
            println!("BANG! {} is eliminated!{}", shot.target,
                if is_human { " (HUMAN)" } else { "" });

            eliminated_writer.write(PlayerEliminatedMessage {
                victim: shot.target,
                shooter: shot.shooter,
            });

            commands.entity(shot.target).remove::<Alive>();
            commands.entity(shot.target).remove::<Hand>();
            commands.entity(shot.target).remove::<OpenMentsu>();
            commands.entity(shot.target).remove::<Kawa>();
            commands.entity(shot.target).remove::<ClosedHand>();
            commands.entity(shot.target).remove::<Tenpai>();
            commands.entity(shot.target).remove::<Riichi>();
            commands.entity(shot.target).remove::<Ippatsu>();
            commands.entity(shot.target).remove::<DoubleRiichi>();
            commands.entity(shot.target).remove::<Furiten>();
            commands.entity(shot.target).remove::<DrawnTile>();
            commands.entity(shot.target).remove::<RonOption>();
            commands.entity(shot.target).remove::<RonDeclared>();
            commands.entity(shot.target).remove::<TsumoOption>();
            commands.entity(shot.target).remove::<PonOption>();
            commands.entity(shot.target).remove::<ChiOption>();
            commands.entity(shot.target).remove::<DaiminkanOption>();
            commands.entity(shot.target).remove::<AnkanOption>();
            commands.entity(shot.target).remove::<ShouminkanOption>();
            commands.entity(shot.target).remove::<RiichiOption>();
            commands.entity(shot.target).remove::<KyuushuOption>();

            if is_human {
                println!("ゲーム終了\nYou died.");
            }

            match game.match_phase {
                MatchPhase::Nima => next_state.set(TurnState::GameOver),
                _ => next_state.set(TurnState::MatchTransition),
            }
            return;

        } else {
            println!("*click* \n{} survives. (chamber now {}/{})",
                shot.target, revolver.chamber, revolver.bullet);

            survived_writer.write(SurvivedShotMessage {
                survivor: shot.target,
                shooter: shot.shooter,
            });
        }
    }

    commands.remove_resource::<ExecuteQueue>();
    commands.remove_resource::<PendingTargetSelection>();
    commands.remove_resource::<RoundOutcome>();
    next_state.set(TurnState::StartNewRound);
}


pub fn bot_tilt_system(
    mut ron_messages: MessageReader<RonDealtMessage>,
    mut tsumo_messages: MessageReader<TsumoDealtMessage>,
    mut eliminated_messages: MessageReader<PlayerEliminatedMessage>,
    mut survived_messages: MessageReader<SurvivedShotMessage>,
    mut bot_query: Query<(Entity, &mut BotProfile), With<Alive>>,
) {
    // ron
    for ron in ron_messages.read() {
        if let Ok((player, mut profile)) = bot_query.get_mut(ron.loser.player) {
            let mut tilt = 0.15;

            if ron.is_yakuman { tilt += 0.25; }
            if ron.loser.was_riichi { tilt += 0.10; }
            if ron.loser.was_tenpai { tilt += 0.08; }

            if let Some(best_han) = ron.loser.best_han && best_han > ron.winning_han { 
                tilt += 0.10; 
            }

            let damage = tilt * (1.0 - profile.emotional_invulnerability);
            profile.composure = (profile.composure - damage).max(0.1); 

            println!("Tilt: Bot {:?} took {:.2} composure damage from Ron. (now {:.2})",
                player, damage, profile.composure);
        }
    }

    // tsumo
    for tsumo in tsumo_messages.read() {
        for loser in &tsumo.losers {
            if let Ok((player, mut profile)) = bot_query.get_mut(loser.player) {
                let mut tilt = 0.08;

                if tsumo.is_yakuman { tilt += 0.25; }
                if loser.was_riichi { tilt += 0.10; }
                if loser.was_tenpai { tilt += 0.08; }

                if let Some(best_han) = loser.best_han && best_han > tsumo.winning_han { 
                    tilt += 0.10; 
                }

                let damage = tilt * (1.0 - profile.emotional_invulnerability);
                profile.composure = (profile.composure - damage).max(0.2);

                println!("Tilt: Bot {:?} took {:.2} composure damage from Tsumo. (now {:.2})",
                    player, damage, profile.composure);
            }
        }
    }

    // witnessing elimination
    for elim in eliminated_messages.read() {
        for (bot_entity, mut profile) in bot_query.iter_mut() {
            if bot_entity != elim.shooter && bot_entity != elim.victim {
                let tilt = 0.25;
                let damage = tilt * (1.0 - profile.emotional_invulnerability);
                profile.composure = (profile.composure - damage).max(0.2);

                println!("Tilt: Bot {:?} took {:.2} composure damage from witnessing elimination. (now {:.2})",
                    bot_entity, damage, profile.composure);
            }
        }
    }

    // surviving a shot
    for surv in survived_messages.read() {
        if let Ok((player, mut profile)) = bot_query.get_mut(surv.survivor) {
            let tilt = 0.35;
            let damage = tilt * (1.0 - profile.emotional_invulnerability);
            profile.composure = (profile.composure - damage).max(0.2);

            println!("Tilt: Bot {:?} took {:.2} composure damage from surviving a shot. (now {:.2})",
                player, damage, profile.composure);
        }
    }
}


pub fn check_ryuukyoku(
    query: Query<(Entity, &Kawa, Has<Oya>), (With<Alive>, Without<DiscardWasCalled>)>,
    mut points_query: Query<(Entity, &mut Points, Has<Oya>)>,
    oya_tenpai_query: Single<Has<Tenpai>, With<Oya>>,
    wall: Res<Wall>,
    game: Res<GameState>,
    round_result: Option<Res<RoundResult>>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut commands: Commands,
) {
    if round_result.is_some() { 
        return; 
    }

    if wall.remaining_draws() == 0 {
        println!("Ryuukyoku! Wall exhausted. Oya tenpai: {}", *oya_tenpai_query);
        let mut nagashi_winners: Vec<(Entity, HandResult, u32)> = vec![];
        let mut any_oya_won = false;
            

        for (nagashi_candidate, kawa, is_candidate_oya) in query.iter() {
            if nagashi_mangan(kawa) {
                println!("{} Has achieved Nagashi Mangan!", nagashi_candidate);

                let result = HandResult {
                    yaku_names: vec!["Nagashi Mangan".to_string()],
                    dora_count: 0,
                    ura_dora_count: 0,
                    total_han: 5,
                    total_fu: 20,
                    is_yakuman: false,
                };

                // treat as tsumo
                let score = calculate_score(result.total_han, 
                    result.total_fu, is_candidate_oya, true, false, &result.yaku_names, game.match_phase);

                for (player, mut points, is_oya) in points_query.iter_mut() {
                    if player == nagashi_candidate {
                        points.0 += score.total_won as i32;
                    } else {
                        if is_oya {
                            points.0 -= score.oya_pays as i32;
                        } else {
                            points.0 -= score.non_oya_pays as i32;
                        }
                    }
                }
                nagashi_winners.push((nagashi_candidate, result, score.total_won));
                if is_candidate_oya {
                    any_oya_won = true;
                }
            }
        }

        if !nagashi_winners.is_empty() {
            commands.insert_resource(RoundOutcome {
                winners: nagashi_winners,
                loser: None,
                is_tsumo: true,
                tochuu_causer: vec![],
            });

            if any_oya_won {
                commands.insert_resource(RoundResult(RoundEndReason::OyaWin));
            } else {
                commands.insert_resource(RoundResult(RoundEndReason::NonOyaWin));
            }

            next_state.set(TurnState::RoundEnd);
            return; // Abort standard ryuukyoku!
        }

        if *oya_tenpai_query {
            commands.insert_resource(RoundResult(RoundEndReason::RyuukyokuOyaTenpai));
        } else {
            commands.insert_resource(RoundResult(RoundEndReason::RyuukyokuOyaNoten));
        }
        next_state.set(TurnState::RoundEnd);

    }
}


// runs once upon entering CallWindow 
pub fn ron_check(
    query: Query<(
        Entity, &Hand,
        &OpenMentsu, &NukedTiles,
        &Tenpai, &Kawa, &Jikaze,
        Has<ClosedHand>, Has<Oya>, Has<Riichi>, Has<Ippatsu>, Has<DoubleRiichi>, Has<Furiten>
    )>,
    discard_query: Query<(&DiscardedTile, &DiscardedBy, Has<Chankan>), With<CurrentDiscard>>,
    game: Res<GameState>,
    wall: Res<Wall>,
    mut commands: Commands,
) {
    let Ok((discarded_tile, discarded_by, is_chankan)) = discard_query.single() else {
        return;
    };

    for (player, hand, open_mentsu, nuked_tiles, tenpai, kawa, jikaze,
         is_closed, is_oya, is_riichi, is_ippatsu, is_double, has_temp_furiten) in &query
    {
        if player == discarded_by.0 { continue; }

        if let Some(result) = can_declare_ron(
            &discarded_tile.0, &hand.0, &open_mentsu.0, &nuked_tiles.0, tenpai,
            is_closed, is_oya, kawa, is_riichi, is_double, is_ippatsu,
            &game.bakaze, &jikaze.0, &*wall,
            is_chankan, game.calls_made, has_temp_furiten
        ) {
            commands.entity(player).insert(RonOption {
                discarded_by: discarded_by.0,
                result,
            });
            println!("{} has Ron option on {:?}", player, discarded_tile.0);
        }
    }
}


pub fn declare_ron(
    declared: Query<(Entity, &RonOption, &Jikaze), With<RonDeclared>>,
    undecided: Query<Entity, (With<RonOption>, Without<RonDeclared>)>,
    alive_check: Query<(), With<Alive>>,
    oya_query: Query<Has<Oya>>,
    jikaze_query: Query<&Jikaze>,
    mut points_query: Query<&mut Points>,
    loser_query: Query<(
        &Hand, &OpenMentsu, &NukedTiles, Option<&Tenpai>, &Kawa, &Jikaze,
        Has<ClosedHand>, Has<Oya>, Has<Riichi>, Has<Ippatsu>, Has<DoubleRiichi>,
    )>,
    visible_query: Query<(Entity, &Kawa, &OpenMentsu)>,
    mut game: ResMut<GameState>,
    wall: Res<Wall>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut lock: ResMut<CallLock>,
    mut ron_writer: MessageWriter<RonDealtMessage>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands,
) {
    // TODO: consider the possibility of ai not declaring a ron
    // TODO: OR add a human confirmation because humans are slow and would be too late to call ron on multiple ron against ais

    if !undecided.is_empty() { return; }

    // automatically transitions to the next phase
    let mut winners: Vec<_> = declared.iter().collect();
    if winners.is_empty() {
        return;
    }

    lock.0 = true;

    let player_count = alive_check.iter().count();

    // sanchahou
    if winners.len() == (player_count - 1) && player_count > 2 {
        commands.insert_resource(RoundResult(RoundEndReason::TochuuRyuukyoku));
        commands.insert_resource(RoundOutcome {
            winners: vec![],
            loser: Some(winners[0].1.discarded_by),
            is_tsumo: false,
            tochuu_causer: winners.iter().map(|(e, _, _)| *e).collect(),
        });
        next_state.set(TurnState::RoundEnd);
        return;
    }

    let loser = winners[0].1.discarded_by;

    // build visible tiles from loser's perspective
    let mut visible_tiles = [0u8; 34];
    for (entity, kawa, open) in visible_query.iter() {
        for tile in &kawa.0 {
            visible_tiles[tile_to_index(tile)] += 1;
        }
        if entity != loser {
            for mentsu in &open.0 {
                for tile in mentsu.tiles() {
                    visible_tiles[tile_to_index(tile)] += 1;
                }
            }
        }
    }
    for tile in &wall.get_dora_indicators() {
        visible_tiles[tile_to_index(tile)] += 1;
    }

    let mut any_oya_won = false;
    let mut ron_winners = vec![];
    let mut best_winning_han = 0u8;
    let mut best_is_yakuman = false;
    let mut best_winner = Entity::PLACEHOLDER;

    for (winner, ron_option, _) in &winners {
        let is_oya = oya_query.get(*winner).unwrap_or(false);
        if is_oya {
            any_oya_won = true;
        }

        let score = calculate_score(
            ron_option.result.total_han,
            ron_option.result.total_fu,
            is_oya,
            false,
            ron_option.result.is_yakuman,
            &ron_option.result.yaku_names,
            game.match_phase,
        );

        if let Ok([mut winner_pts, mut loser_pts]) =
            points_query.get_many_mut([*winner, ron_option.discarded_by])
        {
            let final_payout = score.total_won as i32 + game.honba as i32 * ((player_count as i32 - 1) * 100);
            winner_pts.0 += final_payout;
            loser_pts.0 -= final_payout;
        }

        if ron_option.result.total_han > best_winning_han || ron_option.result.is_yakuman {
            best_winning_han = ron_option.result.total_han;
            best_is_yakuman = ron_option.result.is_yakuman;
            best_winner = *winner;
        }

        ron_winners.push((*winner, ron_option.result.to_owned(), score.total_won));

        if let Some(ref mut log) = replay_log {
            log.events.push(ReplayEvent::Ron {
                winner: *winner,
                from: ron_option.discarded_by,
                result: ron_option.result.to_owned(),
                payout: score.total_won,
            });
        }

        println!("{} declares Ron on {}! {:?} - {}han {}fu - {} points",
            winner, ron_option.discarded_by, ron_option.result.yaku_names,
            ron_option.result.total_han, ron_option.result.total_fu,
            score.total_won
        );
    }

    // sends tilt message
    if let Ok((hand, open, nuked_tiles, tenpai, kawa, jikaze,
              is_closed, is_oya, is_riichi, is_ippatsu, is_double)) = loser_query.get(loser)
    {
        let loser_tilt = build_loser_tilt_info(
            loser, hand, open, nuked_tiles, tenpai, kawa, jikaze, // PASSED nuked_tiles
            is_closed, is_oya, is_riichi, is_double, is_ippatsu,
            &game.bakaze, &*wall, game.calls_made, &visible_tiles,
        );

        ron_writer.write(RonDealtMessage {
            winner: best_winner,
            winning_han: best_winning_han,
            is_yakuman: best_is_yakuman,
            loser: loser_tilt,
        });
    }

    commands.insert_resource(RoundOutcome {
        winners: ron_winners,
        loser: Some(loser),
        is_tsumo: false,
        tochuu_causer: vec![],
    });

    // riichi sticks go to closest winner
    let discarder = winners[0].1.discarded_by;
    if let Ok(discarder_jikaze) = jikaze_query.get(discarder) {
        winners.sort_by_key(|(_, _, jikaze)| jikaze.0.distance_to(&discarder_jikaze.0));
    }

    if let Some((closest, _, _)) = winners.first()
    && let Ok(mut pts) = points_query.get_mut(*closest) {
        pts.0 += game.riichi_points as i32;
        game.riichi_points = 0;
    }

    if any_oya_won {
        commands.insert_resource(RoundResult(RoundEndReason::OyaWin));
    } else {
        commands.insert_resource(RoundResult(RoundEndReason::NonOyaWin));
    }

    next_state.set(TurnState::RoundEnd);
}



pub fn clear_temp_furiten(
    current_turn: Res<CurrentTurn>,
    query: Query<Has<Riichi>>,
    mut commands: Commands,
) {
    if let Ok(is_riichi) = query.get(current_turn.0) {
        if !is_riichi {
            commands.entity(current_turn.0).remove::<Furiten>();
        }
    }
}


// cleanup so player doesn't prepetually qualify for ron
pub fn cleanup_call_options(
    all_call: Query<Entity, Or<(
        With<RonOption>, With<RonDeclared>,
        With<PonOption>, With<PonDeclared>,
        With<ChiOption>, With<ChiDeclared>,
        With<DaiminkanOption>, With<DaiminkanDeclared>,
    )>>,
    discard_query: Single<(Entity, &DiscardedTile, &DiscardedBy), With<CurrentDiscard>>,
    was_called_check: Query<(), With<TileWasCalled>>, // Changed this line
    furiten_check: Query<(Entity, &Tenpai)>,
    round_result: Option<Res<RoundResult>>,
    pre_blackout: Option<Res<PreBlackoutState>>,
    mut kawa_query: Query<(&Kawa, Option<&mut CalledKawaIndices>)>,
    mut commands: Commands,
) {
    if pre_blackout.is_none() && round_result.is_none() {
        let (discard_entity, discarded_tile, discarded_by) = *discard_query;

        // apply temp furiten for anyone in tenpai waiting for this tile
        for (player, tenpai) in &furiten_check {
            if tenpai.0.contains(&discarded_tile.0) {
                commands.entity(player).insert(Furiten);
            }
        }

        // check the tile entity,
        if was_called_check.contains(discard_entity) {
            commands.entity(discard_entity).despawn();

            if let Ok((kawa, mut called_opt)) = kawa_query.get_mut(discarded_by.0) {
                let idx = kawa.0.len().saturating_sub(1);
                if let Some(mut called) = called_opt {
                    if !called.0.contains(&idx) { called.0.push(idx); }
                } else {
                    commands.entity(discarded_by.0).insert(CalledKawaIndices(vec![idx]));
                }
            }
        } else {
            commands.entity(discard_entity).remove::<CurrentDiscard>();
        }
    }

    for entity in &all_call {
        commands.entity(entity)
            .remove::<RonOption>()
            .remove::<RonDeclared>()
            .remove::<PonOption>()
            .remove::<PonDeclared>()
            .remove::<ChiOption>()
            .remove::<ChiDeclared>()
            .remove::<DaiminkanOption>()
            .remove::<DaiminkanDeclared>();
    }
}



// refer to ron counterpart
pub fn tsumo_check(
    current_turn: Res<CurrentTurn>,
    query: Query<(
        &Hand, &OpenMentsu, 
        &NukedTiles,
        &Tenpai, &Kawa,
        &Jikaze, &DrawnTile,
        Has<ClosedHand>, Has<Oya>, Has<Riichi>, Has<Ippatsu>, Has<DoubleRiichi>, Has<DrawnFromRinshan>)>,
    game: Res<GameState>,
    wall: Res<Wall>,
    mut commands: Commands,
) {
    if let Ok((hand, open_mentsu, nuked_tiles, tenpai, kawa, jikaze, drawn,
              is_closed, is_oya, is_riichi, is_ippatsu, is_double, is_rinshan)) = query.get(current_turn.0)

        && let Some(result) = can_declare_tsumo(
            &drawn.0, &hand.0, &open_mentsu.0, &nuked_tiles.0,
            tenpai, is_closed, is_oya, kawa,
            is_riichi, is_double, is_ippatsu,
            &game.bakaze, &jikaze.0, &*wall,
            is_rinshan, game.calls_made,
        ) {
            commands.entity(current_turn.0).insert(TsumoOption { result });
        }
}


pub fn declare_tsumo(
    mut messages: MessageReader<DeclareTsumoMessage>,
    oya_query: Query<Has<Oya>>,
    mut points_query: Query<(Entity, &mut Points, Has<Oya>), With<Alive>>,
    alive_check: Query<(), With<Alive>>,
    loser_info_query: Query<(
        Entity, &Hand, &OpenMentsu, &NukedTiles, Option<&Tenpai>, &Kawa, &Jikaze,
        Has<ClosedHand>, Has<Oya>, Has<Riichi>, Has<Ippatsu>, Has<DoubleRiichi>,
    ), With<Alive>>,
    visible_query: Query<(Entity, &Kawa, &OpenMentsu)>,
    wall: Res<Wall>,
    mut game: ResMut<GameState>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut tsumo_writer: MessageWriter<TsumoDealtMessage>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands,
) {
    // yeah these should be a simple if let check instead of a for loop
    // because this game is turn based and there can only be 1 message per turn (player turn not jun)
    if let Some(message) = messages.read().next() {
        let is_oya = oya_query.get(message.player).unwrap_or(false);

        let player_count = alive_check.iter().count();

        let score = calculate_score(
            message.result.total_han,
            message.result.total_fu,
            is_oya,
            true,
            message.result.is_yakuman,
            &message.result.yaku_names,
            game.match_phase,
        );

        // TODO: leave dead players out of the equation
        for (player, mut player_points, is_dealer) in points_query.iter_mut() {
            if player != message.player {
                if is_dealer {
                    player_points.0 -= score.oya_pays as i32;
                    player_points.0 -= game.honba as i32 * 100;
                } else {
                    player_points.0 -= score.non_oya_pays as i32;
                    player_points.0 -= game.honba as i32 * 100;
                }
            } else {
                player_points.0 += score.total_won as i32;
                player_points.0 += game.riichi_points as i32;
                player_points.0 += game.honba as i32 * ((player_count as i32 - 1) * 100);
                game.riichi_points = 0;
            }
        }

        // build tilt info for each loser
        let mut losers = vec![];
        for (entity, hand, open, nuked_tiles, tenpai, kawa, jikaze,
             is_closed, is_oya_l, is_riichi, is_ippatsu, is_double) in loser_info_query.iter()
        {
            if entity == message.player { continue; }

            let mut visible_tiles = [0u8; 34];
            for (vis_entity, vis_kawa, vis_open) in visible_query.iter() {
                for tile in &vis_kawa.0 {
                    visible_tiles[tile_to_index(tile)] += 1;
                }
                if vis_entity != entity {
                    for mentsu in &vis_open.0 {
                        for tile in mentsu.tiles() {
                            visible_tiles[tile_to_index(tile)] += 1;
                        }
                    }
                }
            }
            for tile in &wall.get_dora_indicators() {
                visible_tiles[tile_to_index(tile)] += 1;
            }

            losers.push(build_loser_tilt_info(
                entity, hand, open, nuked_tiles, tenpai, kawa, jikaze, // PASSED nuked_tiles
                is_closed, is_oya_l, is_riichi, is_double, is_ippatsu,
                &game.bakaze, &*wall, game.calls_made, &visible_tiles,
            ));
        }

        tsumo_writer.write(TsumoDealtMessage {
            winner: message.player,
            winning_han: message.result.total_han,
            is_yakuman: message.result.is_yakuman,
            losers,
        });

        println!("{} declares Tsumo! {:?} - {}han {}fu - {} points",
            message.player, message.result.yaku_names,
            message.result.total_han, message.result.total_fu,
            score.total_won
        );

        if let Some(ref mut log) = replay_log {
            log.events.push(ReplayEvent::Tsumo {
                player: message.player,
                result: message.result.to_owned(),
                payout: score.total_won,
            });
        }

        commands.insert_resource(RoundOutcome {
            winners: vec![(message.player, message.result.to_owned(), score.total_won)],
            loser: None,
            is_tsumo: true,
            tochuu_causer: vec![],
        });

        if is_oya {
            commands.insert_resource(RoundResult(RoundEndReason::OyaWin));
        } else {
            commands.insert_resource(RoundResult(RoundEndReason::NonOyaWin));
        }

        next_state.set(TurnState::RoundEnd);
    }
}


// cleanup so player doesn't prepetually qualify for tsumo
pub fn cleanup_main_phase_options(
    query: Query<
        Entity, Or<(
            With<TsumoOption>, With<RiichiOption>, 
            With<AnkanOption>, With<ShouminkanOption>, 
            With<KyuushuOption>, With<NukidoraOption>, 
            With<RiichiSelecting>
        )>
    >,
    forbidden_query: Query<Entity, With<ForbiddenDiscard>>,
    pre_blackout: Option<Res<PreBlackoutState>>,
    mut commands: Commands,
) {
    for entity in &query {
        commands.entity(entity)
            .remove::<TsumoOption>()
            .remove::<RiichiOption>()
            .remove::<AnkanOption>()
            .remove::<ShouminkanOption>()
            .remove::<KyuushuOption>()
            .remove::<NukidoraOption>()
            .remove::<RiichiSelecting>()
            .remove::<DrawnFromRinshan>();
    }

    if pre_blackout.is_none() {
        for entity in &forbidden_query {
            commands.entity(entity).remove::<ForbiddenDiscard>();
        }
    }
}


pub fn set_tenpai(
    query: Query<(Entity, &Hand)>,
    mut commands: Commands,
) {
    for (entity, hand) in &query {
        let waiting_on = check_tenpai(&hand.0);
        if !waiting_on.is_empty() {
            println!("{} is tenpai, waiting on: {:?}", entity, waiting_on);
            commands.entity(entity).insert(Tenpai(waiting_on));
        } else {
            commands.entity(entity).remove::<Tenpai>();
        }
    }
}


pub fn tenpai_payout_system(mut query: Query<(&mut Points, Has<Tenpai>), With<Alive>>) {
    let tenpai_count = query.iter().filter(|(_, is_tenpai)| *is_tenpai).count();

    for (mut player_points, is_tenpai) in query.iter_mut() {
        match tenpai_count {
            1 => if is_tenpai { player_points.0 += 3000 } else { player_points.0 -= 1000 },
            2 => if is_tenpai { player_points.0 += 1500 } else { player_points.0 -= 1500 },
            3 => if is_tenpai { player_points.0 += 1000 } else { player_points.0 -= 3000 },
            _ => {}
        }
    }
    println!("Tenpai payout: {} players tenpai", tenpai_count);
}


pub fn kyuushu_check(
    current_turn: Res<CurrentTurn>,
    query: Query<(&Hand, &Kawa, &DrawnTile)>,
    game: Res<GameState>,
    mut commands: Commands,
) {
    if let Ok((hand, kawa, drawn)) = query.get(current_turn.0) {
        let mut combined = hand.0.clone();
        combined.push(drawn.0);
        if can_declare_kyuushu(&combined, game.calls_made, kawa) {
            commands.entity(current_turn.0).insert(KyuushuOption);
        }
    }
}


pub fn declare_kyuushu(
    mut messages: MessageReader<DeclareKyuushuMessage>,
    query: Query<(&Hand, &Kawa, &DrawnTile)>,
    game: Res<GameState>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>
) {
    for message in messages.read() {

        if let Ok((hand, kawa, drawn)) = query.get(message.player) {
            let mut combined = hand.0.to_owned();
            combined.push(drawn.0.to_owned());
            if can_declare_kyuushu(&combined, game.calls_made, kawa) {
                println!("{} declares Kyuushu Kyuuhai!", message.player);

                if let Some(ref mut log) = replay_log {
                    log.events.push(ReplayEvent::KyuushuKyuuhai {
                        player: message.player,
                    });
                }

                commands.insert_resource(RoundResult(RoundEndReason::TochuuRyuukyoku));
                commands.insert_resource(RoundOutcome {
                    winners: vec![],
                    loser: None,
                    is_tsumo: false,
                    tochuu_causer: vec![message.player],
                });
                next_state.set(TurnState::RoundEnd);
            }
        }
    }
}


// ! why does this take 3 players as the causer?
pub fn suufon_renda(
    game: Res<GameState>,
    query: Query<&Kawa>,
    causer: Single<Entity, With<CurrentDiscard>>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>
) {
    if !game.calls_made {
        let four_kawa: Vec<&Kawa> = query.iter().collect();

        if four_kawa.iter().all(|kawa| kawa.0.len() == 1) {
            let first_tile = four_kawa[0].0[0];
            if matches!(first_tile, Tile::Honor(Honor::East | Honor::South | Honor::West | Honor::North))
                && four_kawa.iter().all(|kawa| kawa.0[0] == first_tile)
            {
                println!("Suufon Renda! All four players discarded {:?}", first_tile);

                if let Some(ref mut log) = replay_log {
                    log.events.push(ReplayEvent::TochuuRyuukyoku {
                        reason: TochuuType::SuufonRenda,
                        causers: vec![*causer],
                    });
                }

                commands.insert_resource(RoundResult(RoundEndReason::TochuuRyuukyoku));
                commands.insert_resource(RoundOutcome {
                    winners: vec![],
                    loser: None,
                    is_tsumo: false,
                    tochuu_causer: vec![*causer],
                });
                next_state.set(TurnState::RoundEnd);
            }
        }
    }
}


pub fn suucha_riichi(
    query: Query<&Riichi>,
    causer: Single<Entity, With<CurrentDiscard>>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>
) {
    if query.count() == 4 {
        println!("Suucha Riichi! All four players declared Riichi");

        if let Some(ref mut log) = replay_log {
            log.events.push(ReplayEvent::TochuuRyuukyoku {
                reason: TochuuType::SuuchaRiichi,
                causers: vec![*causer],
            });
        }

        commands.insert_resource(RoundResult(RoundEndReason::TochuuRyuukyoku));
        commands.insert_resource(RoundOutcome {
                    winners: vec![],
                    loser: None,
                    is_tsumo: false,
                    tochuu_causer: vec![*causer],
                });
        next_state.set(TurnState::RoundEnd);
    }
}


pub fn suukaikan(
    query: Query<&OpenMentsu>,
    causer: Single<Entity, With<CurrentDiscard>>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    let (players_with_kan, total_kan) = player_and_total_kan_count(&query);

    if total_kan >= 4 && players_with_kan > 1 {
        println!("Suukaikan! {} kan across {} players", total_kan, players_with_kan);

        if let Some(ref mut log) = replay_log {
            log.events.push(ReplayEvent::TochuuRyuukyoku {
                reason: TochuuType::Suukaikan,
                causers: vec![*causer],
            });
        }

        commands.insert_resource(RoundResult(RoundEndReason::TochuuRyuukyoku));
        commands.insert_resource(RoundOutcome {
                    winners: vec![],
                    loser: None,
                    is_tsumo: false,
                    tochuu_causer: vec![*causer],
                });
        next_state.set(TurnState::RoundEnd);
    }
}


pub fn riichi_check(
    current_turn: Res<CurrentTurn>,
    query: Query<(&Hand, &DrawnTile, &Points, Has<ClosedHand>, Has<Riichi>)>,
    wall: Res<Wall>,
    mut commands: Commands,
) {
    if let Ok((hand, drawn, points, is_closed, is_riichi)) = query.get(current_turn.0) {
        if is_riichi || !is_closed || points.0 < 1000 || wall.remaining_draws() < 4 {
            return;
        }

        let mut full_hand = hand.0.clone();
        full_hand.push(drawn.0);

        let mut valid_discards = vec![];
        let mut seen = vec![];

        for (i, tile) in full_hand.iter().enumerate() {
            if seen.contains(tile) { continue; }
            seen.push(*tile);

            let mut remaining = full_hand.clone();
            remaining.remove(i);

            if !check_tenpai(&remaining).is_empty() {
                valid_discards.push(*tile);
            }
        }

        if !valid_discards.is_empty() {
            commands.entity(current_turn.0).insert(RiichiOption(valid_discards));
        }
    }
}


pub fn declare_riichi(
    mut messages: MessageReader<DeclareRiichiMessage>,
    mut query: Query<(&mut Hand, &mut Points, &mut Kawa, Option<&DrawnTile>, &RiichiOption)>,
    mut game: ResMut<GameState>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands,
) {
    for message in messages.read() {
        if let Ok((mut hand, mut points, mut kawa, maybe_drawn, riichi_option)) = query.get_mut(message.player) {
            if !riichi_option.0.contains(&message.tile) {
                println!("{} tried to riichi-discard {:?} but it's not valid", message.player, message.tile);
                continue;
            }

            let is_double = kawa.0.is_empty() && !game.calls_made;

            // Handle the discard
            if let Some(drawn) = maybe_drawn {
                if message.tile != drawn.0 {
                    hand.0.push(drawn.0);
                    if let Some(idx) = hand.0.iter().position(|x| *x == message.tile) {
                        hand.0.remove(idx);
                    }
                }
                commands.entity(message.player).remove::<DrawnTile>();
            } else {
                if let Some(idx) = hand.0.iter().position(|x| *x == message.tile) {
                    hand.0.remove(idx);
                }
            }
            hand.0.sort();
            kawa.0.push(message.tile);

            commands.entity(message.player).insert((
                Riichi { turns_since: 0 },
                Ippatsu,
            ));
            if is_double {
                commands.entity(message.player).insert(DoubleRiichi);
            }

            commands.spawn((
                CurrentDiscard,
                DiscardedTile(message.tile),
                DiscardedBy(message.player),
            ));

            points.0 -= 1000;
            game.riichi_points += 1000;

            println!("{} declares {}Riichi, discards {:?}",
                message.player, if is_double { "Double " } else { "" }, message.tile);

            if let Some(ref mut log) = replay_log {
                log.events.push(ReplayEvent::RiichiDeclared {
                    player: message.player,
                    tile: message.tile,
                    is_double,
                });
            }

            next_state.set(TurnState::CallWindow);
        }
    }
}


pub fn pon_check(
    query: Query<(Entity, &Hand), Without<Riichi>>,
    discard: Query<(&DiscardedTile, &DiscardedBy), (With<CurrentDiscard>, Without<Chankan>)>,
    mut commands: Commands,
) {
    let Ok((tile, discarded_by)) = discard.single() else { return };

    for (player, hand) in &query {
        if player == discarded_by.0 { continue; }
        if can_declare_pon(&hand.0, &tile.0) {
            println!("{} has Pon option on {:?}", player, tile.0);
            commands.entity(player).insert(PonOption(tile.0));
        }
    }
}

// ! IMPORTANT: DON'T POP THE DISCARDED TILE FROM KAWA BECAUSE IT'LL BE USED FOR FURITEN CHCECK!
// ! DESPAWN WILL HANDLE THE VISUAL OF MOVING THE DISCARDED TILE INTO OPEN MENTSU
// ! IT'D LOOK LIKE THERE ARE MORE THAN 4 TILES FOR EACH TYPE, BUT JUST IGNORE THIS FOR NOW
// ! THIS APPLIES TO CHI AND KAN AS WELL
pub fn declare_pon(
    declared: Query<(Entity, &PonOption), With<PonDeclared>>,
    undecided: Query<(), (With<PonOption>, Without<PonDeclared>)>,
    higher_priority: Query<(), With<RonOption>>,
    mut query: Query<(&mut Hand, &mut OpenMentsu)>,
    ippatsu_query: Query<Entity, With<Ippatsu>>,
    discard_query: Single<(Entity, &DiscardedBy), With<CurrentDiscard>>,
    jikaze_query: Query<&Jikaze>,
    mut game: ResMut<GameState>,
    mut current_turn: ResMut<CurrentTurn>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut lock: ResMut<CallLock>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands,
) {
    if !undecided.is_empty() || !higher_priority.is_empty() { return; }
    if lock.0 { return; }

    for (player, pon_option) in declared.iter() {
        if let Ok((mut hand, mut open_mentsu)) = query.get_mut(player) {
            lock.0 = true;
            let tile = pon_option.0;
            let (discard_entity, discarded_by) = *discard_query;

            let mut rot_idx = 0;
            if let (Ok(player_wind), Ok(discard_wind)) = (jikaze_query.get(player), jikaze_query.get(discarded_by.0)) {
                let distance = (discard_wind.0.to_num() + 4 - player_wind.0.to_num()) % 4;
                rot_idx = match distance { 3 => 0, 2 => 1, 1 => 2, _ => 0 };
            }

            open_mentsu.0.push(Mentsu::Koutsu([tile; 3], MentsuState::Open(rot_idx)));
            println!("{} declares Pon on {:?}", player, tile);

            for _ in 0..2 {
                let idx = hand.0.iter().position(|x| *x == tile).unwrap();
                hand.0.remove(idx);
            }

            // this does compile because of non-lexical lifetimes
            // same with chi and kan
            for ippatsu_player in ippatsu_query.iter() {
                commands.entity(ippatsu_player).remove::<Ippatsu>();
            }

            commands.entity(player).remove::<ClosedHand>();
            commands.entity(discarded_by.0).insert(DiscardWasCalled);
            commands.entity(discarded_by.0).insert(DiscardWasCalled); // for nagashi mangan
            commands.entity(discard_entity).insert(TileWasCalled);    // for the kawa rendering
            commands.entity(player).insert(ForbiddenDiscard(vec![tile]));

            if let Some(ref mut log) = replay_log {
                log.events.push(ReplayEvent::Pon {
                    player,
                    tile,
                    from: discarded_by.0,
                });
            }

            game.calls_made = true;
            current_turn.0 = player;
            next_state.set(TurnState::MainPhase);
            // timer.0.reset();
            break;
        }
    }
}


pub fn chi_check(
    query: Query<(Entity, &Hand, &Jikaze), Without<Riichi>>,
    discard: Query<(&DiscardedTile, &DiscardedBy), (With<CurrentDiscard>, Without<Chankan>)>,
    jikaze_query: Query<&Jikaze>,
    mut commands: Commands,
) {
    let Ok((tile, discarded_by)) = discard.single() else { return };
    let Ok(discarder_jikaze) = jikaze_query.get(discarded_by.0) else { return };

    for (player, hand, jikaze) in &query {
        if player == discarded_by.0 { continue; }
        if !jikaze.0.is_kamicha_to(&discarder_jikaze.0) { continue; }

        let positions = can_declare_chi(&hand.0, &tile.0);
        if !positions.is_empty() {
            println!("{} has Chi option on {:?} (positions: {:?})", player, tile.0, positions);
            commands.entity(player).insert(ChiOption { tile: tile.0, positions });
        }
    }
}


pub fn declare_chi(
    declared: Query<(Entity, &ChiOption, &ChiDeclared)>,
    undecided: Query<(), (With<ChiOption>, Without<ChiDeclared>)>,
    higher_priority: Query<(), Or<(With<RonOption>, With<PonOption>, With<DaiminkanOption>)>>,
    mut query: Query<(&mut Hand, &mut OpenMentsu, &Jikaze)>,
    ippatsu_query: Query<Entity, With<Ippatsu>>,
    discard_query: Single<(Entity, &DiscardedBy), With<CurrentDiscard>>,
    mut game: ResMut<GameState>,
    mut current_turn: ResMut<CurrentTurn>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut lock: ResMut<CallLock>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands,
) {
    if !undecided.is_empty() || !higher_priority.is_empty() {
        return;
    }
    if lock.0 {
        return;
    }

    for (player, chi_option, chi_declared) in declared.iter() {
        let (discard_entity, discarded_by) = *discard_query;

        let is_valid = if let (
            Ok((hand, _, self_jikaze)),
            Ok((_, _, discard_jikaze))
        ) = (
            query.get(player),
            query.get(discarded_by.0)
        ) {
            let positions = can_declare_chi(&hand.0, &chi_option.tile);
            !positions.is_empty()
                && positions.contains(&chi_declared.0)
                && self_jikaze.0.is_kamicha_to(&discard_jikaze.0)
        } else {
            false
        };

        if is_valid && let Ok((mut hand, mut open_mentsu, _))= query.get_mut(player) {
            lock.0 = true;

            let pos = &chi_declared.0; // let the player choose
            let tile = &chi_option.tile;

            match pos {
                ChiTilePos::Middle => {
                    println!("{} declares Chi on {:?} (position: {:?})", player, tile, pos);
                    let next = next_tile_sequence(tile).unwrap();
                    let prev = previous_tile_sequence(tile).unwrap();
                    // use the variables as a pointer for removal first b4 moving the value
                    hand.remove_tile_from_hand(&next);
                    hand.remove_tile_from_hand(&prev);
                    open_mentsu.0.push(Mentsu::Shuntsu([prev, *tile, next], MentsuState::Open(1)));
                    commands.entity(player).insert(ForbiddenDiscard(vec![*tile]));
                },
                ChiTilePos::Left => {
                    println!("{} declares Chi on {:?} (position: {:?})", player, tile, pos);
                    let next = next_tile_sequence(tile).unwrap();
                    let next_next = next_tile_sequence(&next).unwrap();

                    // https://riichi.wiki/Kuikae
                    let mut forbidden = Vec::new();
                    if hand.0.contains(tile) {
                        forbidden.push(*tile);
                    }
                    if let Some(n) = next_tile_sequence(&next_next) {
                        forbidden.push(n);
                    }
                    if !forbidden.is_empty() {
                        commands.entity(player).insert(ForbiddenDiscard(forbidden));
                    }

                    hand.remove_tile_from_hand(&next);
                    hand.remove_tile_from_hand(&next_next);
                    open_mentsu.0.push(Mentsu::Shuntsu([*tile, next, next_next], MentsuState::Open(0)));
                },
                ChiTilePos::Right => {
                    println!("{} declares Chi on {:?} (position: {:?})", player, tile, pos);
                    let prev = previous_tile_sequence(tile).unwrap();
                    let prev_prev = previous_tile_sequence(&prev).unwrap();

                    let mut forbidden = Vec::new();
                    if hand.0.contains(tile) {
                        forbidden.push(*tile);
                    }
                    if let Some(p) = previous_tile_sequence(&prev_prev) {
                        forbidden.push(p);
                    }
                    if !forbidden.is_empty() {
                        commands.entity(player).insert(ForbiddenDiscard(forbidden));
                    }

                    hand.remove_tile_from_hand(&prev);
                    hand.remove_tile_from_hand(&prev_prev);
                    open_mentsu.0.push(Mentsu::Shuntsu([prev_prev, prev, *tile], MentsuState::Open(2)));
                },
            }

            commands.entity(player).remove::<ClosedHand>();
            commands.entity(discarded_by.0).insert(DiscardWasCalled); // for nagashi mangan
            commands.entity(discard_entity).insert(TileWasCalled);    // for the kawa rendering
            game.calls_made = true;

            for ippatsu_player in ippatsu_query.iter() {
                commands.entity(ippatsu_player).remove::<Ippatsu>();
            }

            if let Some(ref mut log) = replay_log {
                log.events.push(ReplayEvent::Chi {
                    player,
                    tile: *tile,
                    position: chi_declared.0,
                    from: discarded_by.0,
                });
            }

            current_turn.0 = player;
            next_state.set(TurnState::MainPhase);
            // timer.0.reset();
            break;
        }
    }
}


pub fn player_and_total_kan_count(query: &Query<&OpenMentsu>) -> (u8, u8) {
    let mut players_with_kan = 0;
    let mut total_kan = 0;
    for open in query.iter() {
        let kan = open.0.iter()
            .filter(|mentsu| matches!(mentsu, Mentsu::Ankan(_) | Mentsu::Daiminkan(_, _) | Mentsu::Shouminkan(_, _)))
            .count();

        if kan > 0 {
            players_with_kan += 1;
        }
        total_kan += kan;
    }
    (players_with_kan, total_kan as u8)
}


pub fn ankan_check(
    current_turn: Res<CurrentTurn>,
    query: Query<(&Hand, &DrawnTile, Option<&Tenpai>, Has<Riichi>)>,
    open_query: Query<&OpenMentsu>,
    mut commands: Commands,
) {
    if player_and_total_kan_count(&open_query).1 >= 4 { return; }

    if let Ok((hand, drawn, maybe_tenpai, is_riichi)) = query.get(current_turn.0) {
        let mut full_hand = hand.0.clone();
        full_hand.push(drawn.0);
        let mut kan_tiles = vec![];
        let mut seen = vec![];
        for tile in &full_hand {
            if seen.contains(tile) { continue; }
            seen.push(*tile);
            if full_hand.iter().filter(|t| *t == tile).count() == 4 {
                if is_riichi {
                    let remaining: Vec<Tile> = full_hand.iter()
                        .filter(|t| *t != tile)
                        .copied()
                        .collect();
                    let new_waits = check_tenpai(&remaining);
                    if let Some(tenpai) = maybe_tenpai && new_waits != tenpai.0 { 
                        continue; 
                    }
                }
                kan_tiles.push(*tile);
            }
        }
        if !kan_tiles.is_empty() {
            commands.entity(current_turn.0).insert(AnkanOption(kan_tiles));
        }
    }
}

pub fn daiminkan_check(
    query: Query<(Entity, &Hand), Without<Riichi>>,
    discard: Query<(&DiscardedTile, &DiscardedBy), (With<CurrentDiscard>, Without<Chankan>)>,
    open_query: Query<&OpenMentsu>,
    mut commands: Commands,
) {
    if player_and_total_kan_count(&open_query).1 >= 4 { return; }

    let Ok((tile, discarded_by)) = discard.single() else { return };

    for (player, hand) in &query {
        if player == discarded_by.0 { continue; }
        if can_declare_kan_from_hand(&hand.0, &tile.0) == 3 {
            commands.entity(player).insert(DaiminkanOption(tile.0));
        }
    }
}

pub fn shouminkan_check(
    current_turn: Res<CurrentTurn>,
    query: Query<(&Hand, &OpenMentsu, &DrawnTile)>,
    open_query: Query<&OpenMentsu>,
    mut commands: Commands,
) {
    if player_and_total_kan_count(&open_query).1 >= 4 { return; }

    if let Ok((hand, open, drawn)) = query.get(current_turn.0) {
        let mut full_hand = hand.0.clone();
        full_hand.push(drawn.0);
        let mut kan_tiles = vec![];
        for tile in &full_hand {
            if can_declare_kan_from_pon(&open.0, tile) && !kan_tiles.contains(tile) {
                kan_tiles.push(*tile);
            }
        }
        if !kan_tiles.is_empty() {
            commands.entity(current_turn.0).insert(ShouminkanOption(kan_tiles));
        }
    }
}


// ankan + shouminkan 
pub fn declare_drawn_kan(
    mut messages: MessageReader<DeclareKanMessage>,
    mut query: Query<(&mut Hand, &mut OpenMentsu, Option<&DrawnTile>)>,
    ippatsu_query: Query<Entity, With<Ippatsu>>,
    mut game: ResMut<GameState>,
    mut wall: ResMut<Wall>,
    mut current_turn: ResMut<CurrentTurn>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut lock: ResMut<CallLock>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands
) {
    for message in messages.read() {
        if message.is_discard { continue; } // daiminkan handled in call window
        if lock.0 { return; }

        if let Ok((mut hand, mut open_mentsu, maybe_drawn)) = query.get_mut(message.player) {
            lock.0 = true;
            let tile = &message.tile;

            // merge drawn into hand first so ankan with different drawn doesn't get sent into the shadow realm
            if let Some(drawn) = maybe_drawn {
                hand.0.push(drawn.0);
                commands.entity(message.player).remove::<DrawnTile>();
            }

            let count = can_declare_kan_from_hand(&hand.0, tile);
            let mut kan_successful_type: Option<Kantsu> = None;

            if count == 4 {
                open_mentsu.0.push(Mentsu::Ankan([*tile; 4]));

                wall.dora_count += 1;

                hand.0.retain(|hand_tile| hand_tile != tile);
                kan_successful_type = Some(Kantsu::Ankan);
                game.pending_rinshan = true;
            }
            else {
                for mentsu in &mut open_mentsu.0 {
                    if let Mentsu::Koutsu(tiles, MentsuState::Open(rot_idx)) = mentsu {
                        if tiles[0] == *tile {
                            let r = *rot_idx;
                            *mentsu = Mentsu::Shouminkan([*tile; 4], r);
                            hand.0.retain(|x| x != tile);
                            kan_successful_type = Some(Kantsu::Shouminkan);
                            game.pending_kan_dora = true;
                            game.pending_rinshan = true;
                            commands.spawn((
                                CurrentDiscard,
                                DiscardedTile(*tile),
                                DiscardedBy(message.player),
                                Chankan,
                            ));
                            break;
                        }
                    }
                }
            }

            match kan_successful_type {
                Some(Kantsu::Ankan) => {
                    println!("{} declares Ankan on {:?}", message.player, tile);

                    if let Some(ref mut log) = replay_log {
                        log.events.push(ReplayEvent::Ankan {
                            player: message.player,
                            tile: *tile,
                        });
                        log.events.push(ReplayEvent::DoraRevealed {
                            indicator: *wall.get_dora_indicators().last().unwrap(),
                        });
                    }

                    game.calls_made = true;
                    for player in ippatsu_query.iter() {
                        commands.entity(player).remove::<Ippatsu>();
                    }
                    current_turn.0 = message.player;
                    next_state.set(TurnState::RinshanDraw);
                },
                Some(Kantsu::Shouminkan) => {
                    println!("{} declares Shouminkan on {:?}", message.player, tile);

                    if let Some(ref mut log) = replay_log {
                        log.events.push(ReplayEvent::Shouminkan {
                            player: message.player,
                            tile: *tile,
                        });
                    }

                    game.calls_made = true;
                    for player in ippatsu_query.iter() {
                        commands.entity(player).remove::<Ippatsu>();
                    }
                    current_turn.0 = message.player;
                    next_state.set(TurnState::CallWindow);
                },
                None => println!("{} attempted closed Kan on {:?} but failed", message.player, tile),
            }

        }
    }
}


// daiminkan
pub fn declare_discarded_kan(
    declared: Query<(Entity, &DaiminkanOption), With<DaiminkanDeclared>>,
    undecided: Query<(), (With<DaiminkanOption>, Without<DaiminkanDeclared>)>,
    higher_priority: Query<(), With<RonOption>>,
    mut query: Query<(&mut Hand, &mut OpenMentsu)>,
    ippatsu_query: Query<Entity, With<Ippatsu>>,
    discard_query: Single<(Entity, &DiscardedBy), With<CurrentDiscard>>,
    jikaze_query: Query<&Jikaze>,
    mut game: ResMut<GameState>,
    mut current_turn: ResMut<CurrentTurn>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut lock: ResMut<CallLock>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands
) {
    if !undecided.is_empty() || !higher_priority.is_empty() { return; }
    if lock.0 { return; }

    for (player, daiminkan_option) in declared.iter() {
        if let Ok((mut hand, mut open_mentsu)) = query.get_mut(player) {
            lock.0 = true;
            let tile = &daiminkan_option.0;
            let count = can_declare_kan_from_hand(&hand.0, tile);
            let (discard_entity, discarded_by) = *discard_query;

            if count == 3 {
                let mut rot_idx = 0;
                if let (Ok(p_wind), Ok(d_wind)) = (jikaze_query.get(player), jikaze_query.get(discarded_by.0)) {
                    let distance = (d_wind.0.to_num() + 4 - p_wind.0.to_num()) % 4;
                    rot_idx = match distance { 3 => 0, 2 => 1, 1 => 3, _ => 0 };
                }

                open_mentsu.0.push(Mentsu::Daiminkan([*tile; 4], rot_idx));
                hand.0.retain(|x| x != tile);

                commands.entity(player).remove::<ClosedHand>();
                commands.entity(discarded_by.0).insert(DiscardWasCalled); // for nagashi mangan
                commands.entity(discard_entity).insert(TileWasCalled);    // for the kawa rendering

                game.pending_kan_dora = true;
                game.pending_rinshan = true;
                game.calls_made = true;

                println!("{} declares Daiminkan on {:?}", player, tile);

                if let Some(ref mut log) = replay_log {
                    log.events.push(ReplayEvent::Daiminkan {
                        player,
                        tile: *tile,
                        from: discarded_by.0,
                    });
                }

                for ippatsu_player in ippatsu_query.iter() {
                    commands.entity(ippatsu_player).remove::<Ippatsu>();
                }

                current_turn.0 = player;
                next_state.set(TurnState::RinshanDraw);
            } else {
                println!("{} attempted Daiminkan on {:?} but failed", player, tile);
            }
            break;
        }
    }
}


pub fn nukidora_check(
    current_turn: Res<CurrentTurn>,
    game: Res<GameState>,
    query: Query<(&Hand, &DrawnTile, Has<Riichi>)>,
    mut commands: Commands,
) {
    if game.match_phase == MatchPhase::Yonma { return; }

    if let Ok((hand, drawn, is_riichi)) = query.get(current_turn.0) {
        let mut full_hand = hand.0.clone();
        full_hand.push(drawn.0);

        let mut nuki_options = vec![];
        for tile in full_hand {
            if tile == Tile::Honor(Honor::North) && !nuki_options.contains(&tile) {
                // can only nuki the north if you just drew it
                if is_riichi {
                    if drawn.0 == Tile::Honor(Honor::North) {
                        nuki_options.push(tile);
                    }
                } else {
                    nuki_options.push(tile);
                }
            }
        }

        if !nuki_options.is_empty() {
            commands.entity(current_turn.0).insert(NukidoraOption(nuki_options));
        }
    }
}

pub fn declare_nukidora(
    mut messages: MessageReader<DeclareNukidoraMessage>,
    mut query: Query<(&mut Hand, Option<&DrawnTile>, &mut NukedTiles)>,
    ippatsu_query: Query<Entity, With<Ippatsu>>,
    mut game: ResMut<GameState>,
    mut current_turn: ResMut<CurrentTurn>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut lock: ResMut<CallLock>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands
) {
    for message in messages.read() {
        if lock.0 { return; }

        if let Ok((mut hand, maybe_drawn, mut nuked_tiles)) = query.get_mut(message.player) {
            lock.0 = true;
            let tile = message.tile;

            // merge or extract the tile, same with ankan logic
            if let Some(drawn) = maybe_drawn {
                if drawn.0 == tile {
                    commands.entity(message.player).remove::<DrawnTile>();
                } else {
                    hand.0.push(drawn.0);
                    hand.remove_tile_from_hand(&tile);
                    commands.entity(message.player).remove::<DrawnTile>();
                }
            } else {
                hand.remove_tile_from_hand(&tile);
            }

            nuked_tiles.0.push(tile);

            // ippatsu break
            for player in ippatsu_query.iter() {
                commands.entity(player).remove::<Ippatsu>();
            }

            // rinshan draw
            game.pending_rinshan = true;
            current_turn.0 = message.player;
            next_state.set(TurnState::RinshanDraw);

            println!("{} declares Nuki Pei!", message.player);

            if let Some(ref mut log) = replay_log {
                log.events.push(ReplayEvent::Nukidora {
                    player: message.player,
                    tile,
                });
            }

            break;
        }
    }
}


// !pub fn spawn_camera(mut commands: Commands) { 
// !    commands.spawn(Camera2d::default()); 
// !}

pub fn start_game(
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>
) {
    let mut tiles = vec![];
    for _ in 0..4 {
        tiles.extend(all_tiles());
    }

    let dice_roll = rand::rng().random_range(2..=12) as usize;
    let oya_seat = 0; // East is always seat 0 at the start of the match
    let mut wall = Wall::new(tiles, MatchPhase::Yonma, dice_roll, oya_seat);
    let first_dora = wall.get_dora_indicators()[0];

    let break_seat = match (oya_seat as usize + dice_roll - 1) % 4 {
        0 => "East", 1 => "South", 2 => "West", 3 => "North", _ => "?",
    };
    println!("Dice Roll: {} | Counting from: East Seat | Cut on: {}'s Wall ({} stacks from right edge)",
        dice_roll, break_seat, dice_roll);

    let seats =[Wind::East, Wind::South, Wind::West, Wind::North];
    let mut starting_player = Entity::PLACEHOLDER;

    let mut starting_hands: [Vec<Tile>; 4] = [vec![], vec![], vec![], vec![]];

    // 3 rounds of 4 tiles
    for _ in 0..3 {
        for player_index in 0..4 {
            for _ in 0..4 {
                starting_hands[player_index].push(wall.draw().unwrap());
            }
        }
    }
    // 1 round of 1 tile
    for player_index in 0..4 {
        starting_hands[player_index].push(wall.draw().unwrap());
    }

    let mut seat_info: Vec<(Entity, Wind, i32)> = vec![];
    let mut hand_info: Vec<(Entity, Vec<Tile>)> = vec![];

    for (i, wind) in seats.iter().enumerate() {
        let mut starting_hand = starting_hands[i].clone();
        starting_hand.sort();
        let hand_snapshot = starting_hand.clone();

        // Oya gets their 14th tile directly (Chon-Chon), but because this game triggers `TurnState::Draw` immediately on turn 1,
        // we just deal 13 to everyone and let the TurnState naturally give the 14th tile to East.
        // In real life, East takes two tiles at the end. We mimic the final result mathematically.

        let mut player = commands.spawn((
            PlayerTag,
            Points(25000),
            Jikaze(*wind),
            Seat(i as u8),
            Hand(starting_hand),
            OpenMentsu(vec![]),
            Kawa(vec![]),
            Alive,
            ClosedHand,
            NukedTiles(vec![]),
        ));

        if i == 0 {
            player.insert(HumanPlayer);
        } else {
            player.insert(BotProfile::average());
        }

        if *wind == Wind::East {
            player.insert(Oya);
            starting_player = player.id();
        }

        let player_id = player.id();
        seat_info.push((player_id, *wind, 25000));
        hand_info.push((player_id, hand_snapshot));
    }

    commands.insert_resource(Revolver::new());

    let mut replay_log = ReplayLog::default();
    replay_log.events.push(ReplayEvent::MatchStart {
        phase: MatchPhase::Yonma,
        seats: seat_info,
    });
    replay_log.events.push(ReplayEvent::RoundStart {
        round: 1,
        honba: 0,
        bakaze: Wind::East,
        dora_indicator: first_dora,
        hands: hand_info,
    });
    commands.insert_resource(replay_log);

    commands.insert_resource(
        GameState {
            match_phase: MatchPhase::Yonma,
            rounds: 1,
            honba: 0,
            bakaze: Wind::East,
            bullet: 1,
            calls_made: false,
            riichi_points: 0,
            pending_kan_dora: false,
            pending_rinshan: false,
        }
    );
    commands.insert_resource(CurrentTurn(starting_player));
    commands.insert_resource(wall);

    println!("ゲーム開始");
    next_state.set(TurnState::Draw);
}



pub fn draw_tile(
    current_turn: Res<CurrentTurn>,
    mut wall: ResMut<Wall>,
    mut query: Query<(Entity, Has<Furiten>, Has<Riichi>)>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
    mut replay_log: Option<ResMut<ReplayLog>>,
) {
    // this wouldn't cause a panic because the ryuukyoku check would end the game right there and then
    let drawn = wall.draw().unwrap();
    commands.entity(current_turn.0).insert(DrawnTile(drawn));

    if let Ok((player, _, is_riichi)) = query.get(current_turn.0) {
        if !is_riichi {
            commands.entity(player).remove::<Furiten>();
        }
    }

    if let Some(ref mut log) = replay_log {
        log.events.push(ReplayEvent::Draw {
            player: current_turn.0,
            tile: drawn,
        });
    }

    next_state.set(TurnState::MainPhase);
    println!("{} draws {:?}", current_turn.0, drawn);
}


pub fn rinshan_draw(
    current_turn: Res<CurrentTurn>,
    mut wall: ResMut<Wall>,
    mut game: ResMut<GameState>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
    mut replay_log: Option<ResMut<ReplayLog>>,
) {
    game.pending_rinshan = false;

    let drawn = wall.rinshan_draw().unwrap();
    println!("{} draws {:?} from rinshan", current_turn.0, drawn);

    commands.entity(current_turn.0).insert((
        DrawnTile(drawn),
        DrawnFromRinshan,
    ));

    if let Some(ref mut log) = replay_log {
        log.events.push(ReplayEvent::RinshanDraw {
            player: current_turn.0,
            tile: drawn,
        });
    }

    next_state.set(TurnState::MainPhase);
}


// TODO: ui system (message writer) that greys out forbidden discard tile(s)

pub fn discard_tile(
    mut messages: MessageReader<DiscardTileMessage>,
    mut query: Query<(&mut Hand, Option<&DrawnTile>, &mut Kawa, Option<&mut Riichi>, Option<&ForbiddenDiscard>)>,
    mut commands: Commands,
    current_turn: Res<CurrentTurn>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut game: ResMut<GameState>,
    mut wall: ResMut<Wall>,
    mut replay_log: Option<ResMut<ReplayLog>>,
) {
    let mut processed = false;
    for message in messages.read() {
        if processed || message.player != current_turn.0 { continue; }

        if let Ok((
            mut hand, maybe_drawn,
            mut kawa, maybe_riichi,
            maybe_forbidden
        )) = query.get_mut(message.player) {
            let is_riichi = maybe_riichi.is_some();

            // forced tsumogiri for riichi
            let final_discard = if is_riichi {
                maybe_drawn.expect("riichi should have a drawn tile").0
            } else {
                message.tile
            };

            if let Some(forbidden) = maybe_forbidden && forbidden.0.contains(&final_discard) {
                println!("kuikae nashi on {:?}", final_discard);
                continue; // ! greyed out tiles
            }

            if !is_riichi {
                if let Some(drawn) = maybe_drawn {
                    if !message.is_tsumogiri {
                        hand.0.push(drawn.0);
                        if let Some(idx) = hand.0.iter().position(|x| *x == message.tile) {
                            hand.0.remove(idx);
                        }
                    }
                    commands.entity(message.player).remove::<DrawnTile>();
                } else { // discard after call
                    if let Some(idx) = hand.0.iter().position(|x| *x == message.tile) {
                        hand.0.remove(idx);
                    }
                }
                hand.0.sort();
            } else {
                commands.entity(message.player).remove::<DrawnTile>();
            }

            kawa.0.push(final_discard);

            commands.spawn((
                CurrentDiscard,
                DiscardedTile(final_discard),
                DiscardedBy(message.player),
                IsTsumogiri(message.is_tsumogiri || is_riichi),
            ));
            if let Some(mut riichi) = maybe_riichi {
                if riichi.turns_since > 0 {
                    commands.entity(message.player).remove::<Ippatsu>();
                }
                riichi.turns_since += 1;
            }

            if let Some(ref mut log) = replay_log {
                log.events.push(ReplayEvent::Discard {
                    player: message.player,
                    tile: final_discard,
                    is_tsumogiri: message.is_tsumogiri || is_riichi,
                });
            }

            if game.pending_kan_dora {
                wall.dora_count += 1;
                game.pending_kan_dora = false;

                if let Some(ref mut log) = replay_log {
                    log.events.push(ReplayEvent::DoraRevealed {
                        indicator: *wall.get_dora_indicators().last().unwrap(),
                    });
                }
            }

            commands.entity(message.player)
                .remove::<ForbiddenDiscard>()
                .remove::<DrawnFromRinshan>();
            next_state.set(TurnState::CallWindow);

            println!("{} discards {:?}", message.player, message.tile);
            processed = true;
        }
    }
}


pub fn next_turn(
    mut current_turn: ResMut<CurrentTurn>,
    query: Query<(Entity, &Jikaze), With<Alive>>,
    mut game: ResMut<GameState>,
    wall: Res<Wall>, 
    round_result: Option<Res<RoundResult>>,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    if round_result.is_some() || wall.remaining_draws() == 0 { return; }

    if let Ok((_, current_jikaze)) = query.get(current_turn.0) {
        let mut wind = current_jikaze.0.next_turn_wind();
        for _ in 0..3 {
            if let Some((player, _)) = query.iter().find(|(_, j)| j.0 == wind) {
                current_turn.0 = player;
                next_state.set(TurnState::Draw);
                game.pending_rinshan = false;
                println!("Turn advances to {}", current_turn.0);
                return;
            }
            wind = wind.next_turn_wind();
        }
    }
}


pub fn auto_advance_call_window(
    human_options: Query<(), (
        With<HumanPlayer>,
        Or<(With<RonOption>, With<PonOption>, With<ChiOption>, With<DaiminkanOption>)>
    )>,
    mut next_state: ResMut<NextState<TurnState>>,
    result: Option<Res<RoundResult>>,
    lock: Res<CallLock>,
    game: Res<GameState>,
    busy: Res<AnimationBusy>,
) {
    // wait for the discard animation (or any other animation) to finish
    if busy.0 > 0 { return; }

    // wait for human input
    if !human_options.is_empty() || lock.0 || result.is_some() {
        return;
    }

    if game.pending_rinshan {
        println!("Call window passed, advancing to rinshan draw");
        next_state.set(TurnState::RinshanDraw);
    } else {
        println!("Call window passed, advancing turn");
        next_state.set(TurnState::AdvanceTurn);
    }
}



pub fn start_round(
    mut query: Query<(Entity, &mut Hand, &mut NukedTiles, &Jikaze, &Seat), With<Alive>>,
    alive_check: Query<(), With<Alive>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
    game: Res<GameState>,
    mut replay_log: Option<ResMut<ReplayLog>>,
) {
    if alive_check.iter().count() <= 1 {
        println!("ゲーム終了");
        next_state.set(TurnState::GameOver);
        return;
    }
    println!("--- New Round: {} Bakaze: {:?}, Honba: {} ---", game.rounds, game.bakaze, game.honba);

    let mut tiles = vec![];
    for _ in 0..4 {
        tiles.extend(all_tiles());
    }

    let mut players: Vec<_> = query.iter_mut().collect();
    players.sort_by_key(|(_, _, _, jikaze, _)| jikaze.0.to_num());

    let dice_roll = rand::rng().random_range(2..=12) as usize;
    let dealer_seat = players[0].4.0; // Index 0 is East due to the sort

    let mut wall = Wall::new(tiles, game.match_phase, dice_roll, dealer_seat);
    let first_dora = wall.get_dora_indicators()[0];

    let dealer_seat_name = match dealer_seat { 0 => "East", 1 => "South", 2 => "West", 3 => "North", _ => "?" };
    let break_seat_name = match (dealer_seat as usize + dice_roll - 1) % 4 { 0 => "East", 1 => "South", 2 => "West", 3 => "North", _ => "?" };

    println!("Dice Roll: {} | Counting from: {} Seat | Cut on: {}'s Wall ({} stacks from right edge)",
        dice_roll, dealer_seat_name, break_seat_name, dice_roll);

    // Collect and sort players by Wind to guarantee East gets dealt first
    let mut players: Vec<_> = query.iter_mut().collect();
    players.sort_by_key(|(_, _, _, jikaze, _)| jikaze.0.to_num());

    let mut starting_hands: Vec<Vec<Tile>> = vec![vec![]; players.len()];

    for _ in 0..3 {
        for player_index in 0..players.len() {
            for _ in 0..4 {
                starting_hands[player_index].push(wall.draw().unwrap());
            }
        }
    }
    for player_index in 0..players.len() {
        starting_hands[player_index].push(wall.draw().unwrap());
    }

    let mut hand_info: Vec<(Entity, Vec<Tile>)> = vec![];

    for (index, (entity, mut hand, mut nuked_tiles, _, _)) in players.into_iter().enumerate() {
        hand.0 = starting_hands[index].clone();
        hand.0.sort();
        nuked_tiles.0.clear();
        hand_info.push((entity, hand.0.clone()));
    }

    if let Some(ref mut log) = replay_log {
        log.events.push(ReplayEvent::RoundStart {
            round: game.rounds,
            honba: game.honba,
            bakaze: game.bakaze,
            dora_indicator: first_dora,
            hands: hand_info,
        });
    }

    commands.insert_resource(wall);
    next_state.set(TurnState::Draw);
}


pub fn build_round_summary(
    result: Res<RoundResult>,
    outcome: Option<Res<RoundOutcome>>,
    tenpai_query: Query<Entity, (With<Tenpai>, With<Alive>)>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands,
) {
    let reason_text = match &result.0 {
        RoundEndReason::OyaWin => "Oya Win (Renchan)".to_owned(),
        RoundEndReason::NonOyaWin => "Non-Oya Win".to_owned(),
        RoundEndReason::RyuukyokuOyaTenpai => "Ryuukyoku (Oya Tenpai)".to_owned(),
        RoundEndReason::RyuukyokuOyaNoten => "Ryuukyoku (Oya Noten)".to_owned(),
        RoundEndReason::TochuuRyuukyoku => "Tochuu Ryuukyoku".to_owned(),
    };

    if let Some(ref mut log) = replay_log {
        match &result.0 {
            // tochuu logged by individual tochuu systems
            RoundEndReason::TochuuRyuukyoku => {},
            RoundEndReason::RyuukyokuOyaTenpai | RoundEndReason::RyuukyokuOyaNoten => {
                if let Some(ref outcome) = outcome {
                    // nagashi mangan during ryuukyoku
                    let is_nagashi = outcome.winners.iter().any(|(_, hand_result, _)|
                        hand_result.yaku_names.contains(&"Nagashi Mangan".to_string()));

                    if is_nagashi {
                        for (winner, _, payout) in &outcome.winners {
                            log.events.push(ReplayEvent::NagashiMangan {
                                player: *winner,
                                payout: *payout,
                            });
                        }
                    } else {
                        let tenpai_players: Vec<Entity> = tenpai_query.iter().collect();
                        log.events.push(ReplayEvent::Ryuukyoku { tenpai_players });
                    }
                } else {
                    let tenpai_players: Vec<Entity> = tenpai_query.iter().collect();
                    log.events.push(ReplayEvent::Ryuukyoku { tenpai_players });
                }
            },
            RoundEndReason::OyaWin | RoundEndReason::NonOyaWin => {
                if let Some(ref outcome) = outcome {
                    log.events.push(ReplayEvent::RoundEnd {
                        reason: result.0.clone(),
                        winners: outcome.winners.clone(),
                        loser: outcome.loser,
                        is_tsumo: outcome.is_tsumo,
                    });
                }
            },
        }
    }

    if let Some(outcome) = outcome {
        commands.insert_resource(RoundSummary {
            reason_text,
            winners: outcome.winners.clone(),
            loser: outcome.loser,
            is_tsumo: outcome.is_tsumo,
        });
    } else {
        commands.insert_resource(RoundSummary {
            reason_text,
            winners: vec![],
            loser: None,
            is_tsumo: false,
        });
    }
}

// TODO: despawn tile entity
pub fn round_cleanup(
    mut query: Query<(Entity, &mut Jikaze, Has<Oya>)>,
    alive_query: Query<&Alive>,
    tile_query: Query<Entity, With<DiscardedTile>>,
    player_query: Query<(Entity, &mut Hand, &mut OpenMentsu, &mut Kawa)>,
    result: Res<RoundResult>,
    mut game: ResMut<GameState>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
    outcome: Option<Res<RoundOutcome>>,
    match_end: Option<Res<MatchEndPending>>
) {
    match &result.0 {
        RoundEndReason::OyaWin => println!("Round end: Oya win (renchan)"),
        RoundEndReason::NonOyaWin => println!("Round end: Non-oya win"),
        RoundEndReason::RyuukyokuOyaTenpai => println!("Round end: Ryuukyoku (oya tenpai, renchan)"),
        RoundEndReason::RyuukyokuOyaNoten => println!("Round end: Ryuukyoku (oya noten)"),
        RoundEndReason::TochuuRyuukyoku => println!("Round end: Tochuu ryuukyoku"),
    }
    match result.0 {
        RoundEndReason::OyaWin | RoundEndReason::RyuukyokuOyaTenpai | RoundEndReason::TochuuRyuukyoku => {
            game.honba += 1;
            for (player, _, is_oya) in query.iter() {
                if is_oya {
                    commands.insert_resource(CurrentTurn(player));
                    break;
                }
            }
        },
        RoundEndReason::NonOyaWin | RoundEndReason::RyuukyokuOyaNoten => { 
            game.honba = 0;

            let max_rounds = match game.match_phase {
                MatchPhase::Yonma => 4,
                MatchPhase::Sanma => 3,
                MatchPhase::Nima => 2,
            };

            if game.bakaze == Wind::South && game.rounds == max_rounds {
                match game.match_phase {
                    MatchPhase::Yonma | MatchPhase::Sanma => {
                        commands.insert_resource(MatchEndPending);
                    }
                    MatchPhase::Nima => {
                        game.bakaze = Wind::East;
                        game.rounds = 1;
                    }
                }
            } else if game.bakaze == Wind::East && game.rounds == max_rounds {
                game.bakaze = Wind::South;
                game.rounds = 1;
            } else {
                game.rounds += 1;
            }

            for (player, _, is_oya) in query.iter() {
                if is_oya {
                    commands.entity(player).remove::<Oya>();
                }
            }

            loop {
                for (_, mut jikaze, _) in query.iter_mut() {
                    jikaze.0 = jikaze.0.next_round_wind();
                }

                let mut found = false;
                for (player, jikaze, _) in query.iter() {
                    if jikaze.0 == Wind::East {
                        if alive_query.get(player).is_ok() {
                            commands.entity(player).insert(Oya);
                            commands.insert_resource(CurrentTurn(player));
                            found = true;
                        }
                        break;
                    }
                }
                if found { break; }
            }
         },
    }

    for tile in tile_query.iter() {
        commands.entity(tile).despawn();
    }

    for (player, mut hand, mut open_mentsu, mut kawa) in player_query {
        hand.0.clear();
        open_mentsu.0.clear();
        kawa.0.clear();

        commands.entity(player).remove::<Tenpai>();
        commands.entity(player).remove::<Riichi>();
        commands.entity(player).remove::<Furiten>();
        commands.entity(player).remove::<DrawnTile>();
        commands.entity(player).remove::<RonOption>();      
        commands.entity(player).remove::<RonDeclared>();     
        commands.entity(player).remove::<TsumoOption>();  
        commands.entity(player).remove::<PonOption>();
        commands.entity(player).remove::<ChiOption>();
        commands.entity(player).remove::<DaiminkanOption>();
        commands.entity(player).remove::<AnkanOption>();
        commands.entity(player).remove::<ShouminkanOption>();
        commands.entity(player).remove::<RiichiOption>();
        commands.entity(player).remove::<DoubleRiichi>();
        commands.entity(player).remove::<Ippatsu>();
        commands.entity(player).remove::<KyuushuOption>();
        commands.entity(player).remove::<CalledKawaIndices>();

        commands.entity(player).insert(ClosedHand);
    }
    game.calls_made = false;
    game.pending_kan_dora = false;
    game.pending_rinshan = false;

    commands.remove_resource::<RoundResult>();

    if outcome.is_some() {
        next_state.set(TurnState::Execution);
    } else if match_end.is_some() {
        next_state.set(TurnState::MatchTransition);
    } else {
        next_state.set(TurnState::StartNewRound);
    }
}


pub fn match_transition(
    mut game: ResMut<GameState>,
    mut alive_query: Query<(Entity, &mut Points, &mut Jikaze, Has<HumanPlayer>), With<Alive>>,
    dead_query: Query<Entity, (With<PlayerTag>, Without<Alive>)>,
    tile_query: Query<Entity, With<DiscardedTile>>,
    mut revolver: ResMut<Revolver>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    let expected_players: usize = match game.match_phase {
        MatchPhase::Yonma => 4,
        MatchPhase::Sanma => 3,
        MatchPhase::Nima => unreachable!(),
    };

    // sort by points desc, tiebreak by seat priority asc (lower = better)
    let mut alive: Vec<(Entity, i32, u8, bool)> = alive_query.iter()
        .map(|(player, points, jikaze, is_human)| (player, points.0, jikaze.0.to_num(), is_human))
        .collect();
    alive.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)));

    let mut eliminated_player: Option<Entity> = None;

    // natural end: eliminate lowest scorer
    if alive.len() == expected_players {
        let (loser, _, _, _) = alive.pop().unwrap();
        eliminated_player = Some(loser);

        commands.entity(loser).remove::<Alive>();
        commands.entity(loser).remove::<Hand>();
        commands.entity(loser).remove::<OpenMentsu>();
        commands.entity(loser).remove::<Kawa>();
        commands.entity(loser).remove::<ClosedHand>();
        commands.entity(loser).remove::<Tenpai>();
        commands.entity(loser).remove::<Riichi>();
        commands.entity(loser).remove::<Ippatsu>();
        commands.entity(loser).remove::<DoubleRiichi>();
        commands.entity(loser).remove::<Furiten>();
        commands.entity(loser).remove::<DrawnTile>();
        commands.entity(loser).remove::<Oya>();
    } else {
        // unnatural end: someone was already shot/eliminated during the round
        eliminated_player = dead_query.iter().next();
    }

    // advance phase
    let new_phase = match game.match_phase {
        MatchPhase::Yonma => MatchPhase::Sanma,
        MatchPhase::Sanma => MatchPhase::Nima,
        MatchPhase::Nima => unreachable!(),
    };

    // redistribute points
    let pool = 30000 * alive.len() as i32;
    let cuts: &[i32] = match new_phase {
        MatchPhase::Sanma => &[40, 32, 28],
        MatchPhase::Nima => &[60, 40],
        MatchPhase::Yonma => unreachable!(),
    };

    let seats: &[Wind] = match new_phase {
        MatchPhase::Sanma => &[Wind::East, Wind::South, Wind::West],
        MatchPhase::Nima => &[Wind::East, Wind::South],
        MatchPhase::Yonma => unreachable!(),
    };

    let mut new_standings: Vec<(Entity, Wind, i32)> = vec![];

    for (i, (entity, _, _, _)) in alive.iter().enumerate() {
        let new_points = pool * cuts[i] / 100;
        if let Ok((_, mut points, mut jikaze, _)) = alive_query.get_mut(*entity) {
            points.0 = new_points;
            jikaze.0 = seats[i];
        }

        if i == 0 {
            commands.entity(*entity).insert(Oya);
        } else {
            commands.entity(*entity).remove::<Oya>();
        }

        new_standings.push((*entity, seats[i], new_points));
    }

    if let Some(ref mut log) = replay_log {
        log.events.push(ReplayEvent::MatchTransition {
            new_phase,
            eliminated: eliminated_player.expect("The player did not get eliminated"),
            new_standings,
        });
    }

    // reset game state
    game.match_phase = new_phase;
    game.bakaze = Wind::East;
    game.rounds = 1;
    game.honba = 0;
    game.calls_made = false;
    game.riichi_points = 0;
    game.pending_kan_dora = false;
    game.pending_rinshan = false;

    *revolver = Revolver::new();
    commands.insert_resource(CurrentTurn(alive[0].0));

    // cleanup leftovers
    for tile in tile_query.iter() {
        commands.entity(tile).despawn();
    }
    commands.remove_resource::<MatchEndPending>();
    commands.remove_resource::<RoundResult>();
    commands.remove_resource::<RoundOutcome>();
    commands.remove_resource::<RoundSummary>();
    commands.remove_resource::<ExecuteQueue>();
    commands.remove_resource::<PendingTargetSelection>();

    let human_alive = alive.iter().any(|(_, _, _, is_human)| *is_human);

    if !human_alive {
        next_state.set(TurnState::HumanDeadMenu);
    } else {
        next_state.set(TurnState::StartNewRound);
    }
}


pub fn game_cleanup(
    mut commands: Commands,
    players: Query<Entity, With<PlayerTag>>,
    tiles: Query<Entity, Or<(With<DiscardedTile>, With<CurrentDiscard>)>>,
    mut time: ResMut<Time<Virtual>>,
) {
    for entity in &players { commands.entity(entity).despawn(); }
    for entity in &tiles { commands.entity(entity).despawn(); }

    commands.remove_resource::<RoundResult>();
    commands.remove_resource::<RoundOutcome>();
    commands.remove_resource::<RoundSummary>();
    commands.remove_resource::<ExecuteQueue>();
    commands.remove_resource::<PendingTargetSelection>();
    commands.remove_resource::<AccusationTimer>();
    commands.remove_resource::<KawaSnapshot>();
    commands.remove_resource::<PreBlackoutState>();
    commands.remove_resource::<CheatLog>();
    commands.remove_resource::<BlackoutTimer>();
    commands.remove_resource::<SimulationMode>();
    time.set_relative_speed(1.0);
}


pub fn toggle_vsync(
    simulation: Option<Res<SimulationMode>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = windows.single_mut() else { return };
    if simulation.is_some() {
        if window.present_mode != PresentMode::AutoNoVsync {
            window.present_mode = PresentMode::AutoNoVsync;
        }
    } else {
        if window.present_mode != PresentMode::Fifo {
            window.present_mode = PresentMode::Fifo;
        }
    }
}

pub fn log_game_over(
    query: Query<(Entity, &Points), With<PlayerTag>>,
    mut replay_log: Option<ResMut<ReplayLog>>,
) {
    let Some(mut replay_log) = replay_log else { return };
    let mut standings: Vec<(Entity, i32)> = query.iter()
        .map(|(entity, points)| (entity, points.0))
        .collect();
    standings.sort_by(|left, right| right.1.cmp(&left.1));
    replay_log.events.push(ReplayEvent::GameOver { standings });

    // build the mapping from the starting seats
    let mut entity_map: HashMap<Entity, u8> = HashMap::new();
    if let Some(ReplayEvent::MatchStart { seats, .. }) = replay_log.events.first() {
        // Map based on seat index (0=East, 1=South, 2=West, 3=North)
        for (i, (entity, _, _)) in seats.iter().enumerate() {
            entity_map.insert(*entity, i as u8);
        }
    } else {
        eprintln!("Failed to find MatchStart event. Replay export mapping aborted.");
        return;
    }

    // translate the internal log into the exportable format
    let export_log = ExportableReplayLog {
        events: replay_log.events.iter()
            .map(|event| event.to_export(&entity_map))
            .collect(),
    };

    // serialize and save to TOML
    if let Ok(toml_string) = toml::to_string_pretty(&export_log) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let filename = format!("replay_{}.toml", timestamp);

        if let Err(e) = fs::write(&filename, toml_string) {
            eprintln!("Failed to write replay file: {}", e);
        } else {
            println!("Replay saved successfully to {}", filename);
        }
    } else {
        eprintln!("Failed to serialize replay log to TOML.");
    }
}

