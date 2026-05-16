// TODO: create a function + ui to instantly form a yaku for testing

use crate::core::*;
use crate::components::*;
use crate::resources::*;
use crate::messages::*;
use crate::states::*;
use crate::scoring::*;
use bevy::prelude::*;
use rand::seq::SliceRandom;


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
    for (winner, result) in &outcome.winners {
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

    // TODO: emotional threshold


    // low han self-shot
    for (winner, result) in &outcome.winners {
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


pub fn select_targets(
    mut pending: ResMut<PendingTargetSelection>,
    mut queue: ResMut<ExecuteQueue>,
    mut next_step: ResMut<NextState<ExecutionSubState>>,
    // !+ ui input later
) {
    // TODO: show clickable opponents
    let clicked: Option<Entity> = None; // placeholder

    if let Some(target) = clicked {
        queue.0.push(Execute {
            shooter: pending.shooter,
            target,
        });
        pending.remaining_picks -= 1;

        if pending.remaining_picks == 0 {
            next_step.set(ExecutionSubState::Processing);
        }
    }
}


pub fn process_shot_queue(
    mut queue: ResMut<ExecuteQueue>,
    mut revolver: ResMut<Revolver>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    for shot in queue.0.drain(..) {
        if revolver.pull() {
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
            commands.entity(shot.target).remove::<DoubleRiichi>();
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

            // one death per execution
            break;
        }
    }

    commands.remove_resource::<ExecuteQueue>();
    commands.remove_resource::<PendingTargetSelection>();
    commands.remove_resource::<RoundOutcome>();
    next_state.set(TurnState::StartNewRound);
}



pub fn check_ryuukyoku(
    oya_tenpai_query: Single<Has<Tenpai>, With<Oya>>,
    wall: Res<Wall>,
    round_result: Option<Res<RoundResult>>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut commands: Commands,
) {
    if round_result.is_some() { 
        return; 
    }

    if wall.0.is_empty() {
        println!("Ryuukyoku! Wall exhausted. Oya tenpai: {}", *oya_tenpai_query);
        if *oya_tenpai_query {
            commands.insert_resource(RoundResult(RoundEndReason::RyuukyokuOyaTenpai));
        } else {
            commands.insert_resource(RoundResult(RoundEndReason::RyuukyokuOyaNoten));
        }
        next_state.set(TurnState::RoundEnd);
    }
}




// TODO: multiple ron
// runs once upon entering CallWindow 
pub fn ron_check(
    query: Query<(
        Entity, 
        &Hand, 
        &OpenMentsu, 
        &Tenpai, 
        &Kawa, 
        &Jikaze, 
        Has<ClosedHand>, 
        Has<Oya>, 
        Has<Riichi>, 
        Has<Ippatsu>, 
        Has<DoubleRiichi>, 
        Has<Furiten>
    )>,
    discard_query: Query<(&DiscardedTile, &DiscardedBy, Has<Chankan>), With<CurrentDiscard>>,
    game: Res<GameState>,
    wall: Res<Wall>,
    dead_wall: Res<DeadWall>,
    mut commands: Commands,
) {
    let Ok((discarded_tile, discarded_by, is_chankan)) = discard_query.single() else {
        return;
    };

    for (player, hand, open_mentsu, tenpai, kawa, jikaze,
         is_closed, is_oya, is_riichi, is_ippatsu, is_double, has_temp_furiten) in &query
    {
        if player == discarded_by.0 { continue; }

        if let Some(result) = can_declare_ron(
            &discarded_tile.0, 
            &hand.0, 
            &open_mentsu.0, 
            tenpai,
            is_closed, 
            is_oya, 
            kawa,
            is_riichi, 
            is_double, 
            is_ippatsu,
            &game.bakaze, 
            &jikaze.0,
            &*wall, 
            &*dead_wall,
            is_chankan, 
            game.calls_made,
            has_temp_furiten
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
    mut game: ResMut<GameState>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut lock: ResMut<CallLock>,
    mut commands: Commands,
) {
    // TODO: consider the possibility of ai not declaring a ron
    // TODO: OR add a human confirmation because humans are slow and would be too late to call ron on multiple ron against ais

    // automatically transitions to the next phase
    let mut winners: Vec<_> = declared.iter().collect();
    if winners.is_empty() { 
        return; 
    }

    lock.0 = true;

    let player_count = alive_check.iter().count();

    // sanchahou
    if winners.len() == (player_count - 1)  && player_count > 2 {
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

    let mut any_oya_won = false;
    let mut ron_winners = vec![];
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
        );

        if let Ok([mut winner_pts, mut loser_pts]) =
            points_query.get_many_mut([*winner, ron_option.discarded_by])
        {
            let final_payout = score.total_won as i32 + game.honba as i32 * ((player_count as i32 - 1) * 100);
            winner_pts.0 += final_payout;
            loser_pts.0 -= final_payout;
        }

        ron_winners.push((*winner, ron_option.result.to_owned()));

        println!("{} declares Ron on {}! {:?} - {}han {}fu - {} points",
            winner, ron_option.discarded_by, ron_option.result.yaku_names,
            ron_option.result.total_han, ron_option.result.total_fu,
            score.total_won
        );
    }

    commands.insert_resource(RoundOutcome{
        winners: ron_winners,  
        loser: Some(winners[0].1.discarded_by),       
        is_tsumo: false,
        tochuu_causer: vec![],
    });

    // riichi sticks go to closest winner (sorted first)
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


// cleanup so player doesn't prepetually qualify for ron
pub fn cleanup_call_options(
    passed: Query<Entity, (With<RonOption>, Without<RonDeclared>)>,
    all_call: Query<Entity, Or<(With<RonOption>, With<RonDeclared>, With<PonOption>, With<ChiOption>, With<DaiminkanOption>)>>,
    discard_query: Query<Entity, With<CurrentDiscard>>,
    round_result: Option<Res<RoundResult>>,
    mut commands: Commands,
) {
    if round_result.is_none() {
        for entity in &passed {
            commands.entity(entity).insert(Furiten);
        }
    }

    for entity in &all_call {
        commands.entity(entity)
            .remove::<RonOption>()
            .remove::<RonDeclared>()
            .remove::<PonOption>()
            .remove::<ChiOption>()
            .remove::<DaiminkanOption>();
    }

    for entity in &discard_query {
        commands.entity(entity).despawn();
    }
}




// refer to ron counterpart
pub fn tsumo_check(
    current_turn: Res<CurrentTurn>,
    query: Query<(&Hand, &OpenMentsu, &Tenpai, &Kawa, &Jikaze, &DrawnTile, Has<ClosedHand>, Has<Oya>, Has<Riichi>, Has<Ippatsu>, Has<DoubleRiichi>)>,
    game: Res<GameState>,
    wall: Res<Wall>,
    dead_wall: Res<DeadWall>,
    mut commands: Commands,
) {
    if let Ok((hand, open_mentsu, tenpai, kawa, jikaze, drawn,
              is_closed, is_oya, is_riichi, is_ippatsu, is_double)) = query.get(current_turn.0)
    
        && let Some(result) = can_declare_tsumo(
            &drawn.0, 
            &hand.0, 
            &open_mentsu.0, 
            tenpai,
            is_closed, 
            is_oya, 
            kawa,
            is_riichi, 
            is_double, 
            is_ippatsu,
            &game.bakaze, 
            &jikaze.0,
            &*wall, 
            &*dead_wall,
            game.pending_rinshan, 
            game.calls_made,
        ) {
            commands.entity(current_turn.0).insert(TsumoOption { result });
        }
    
}


pub fn declare_tsumo(
    mut messages: MessageReader<DeclareTsumoMessage>,
    oya_query: Query<Has<Oya>>,
    mut points_query: Query<(Entity, &mut Points, Has<Oya>), With<Alive>>,
    alive_check: Query<(), With<Alive>>,
    mut game: ResMut<GameState>,
    mut next_state: ResMut<NextState<TurnState>>,
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

        println!("{} declares Tsumo! {:?} - {}han {}fu - {} points",
            message.player, message.result.yaku_names,
            message.result.total_han, message.result.total_fu,
            score.total_won
        );

        commands.insert_resource(RoundOutcome{
            winners: vec![(message.player, message.result.to_owned())],  
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
    query: Query<Entity, Or<(With<TsumoOption>, With<RiichiOption>, With<AnkanOption>, With<ShouminkanOption>, With<KyuushuOption>, With<ForbiddenDiscard>, With<RiichiSelecting>)>>,
    mut commands: Commands,
) {
    for entity in &query {
        commands.entity(entity)
            .remove::<TsumoOption>()
            .remove::<RiichiOption>()
            .remove::<AnkanOption>()
            .remove::<ShouminkanOption>()
            .remove::<KyuushuOption>()
            .remove::<ForbiddenDiscard>()
            .remove::<RiichiSelecting>();
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
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>
) {
    for message in messages.read() {

        if let Ok((hand, kawa, drawn)) = query.get(message.player) {
            let mut combined = hand.0.to_owned();
            combined.push(drawn.0.to_owned());
            if can_declare_kyuushu(&combined, game.calls_made, kawa) {
                println!("{} declares Kyuushu Kyuuhai!", message.player);
                commands.insert_resource(RoundResult(RoundEndReason::TochuuRyuukyoku));
                next_state.set(TurnState::RoundEnd);
            }
        }
    }
}


pub fn suufon_renda(
    game: Res<GameState>,
    query: Query<&Kawa>,
    causer: Single<Entity, With<CurrentDiscard>>,
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
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>
) {
    if query.count() == 4 {
        println!("Suucha Riichi! All four players declared Riichi");
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
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    let (players_with_kan, total_kan) = player_and_total_kan_count(&query);

    if total_kan >= 4 && players_with_kan > 1 {
        println!("Suukaikan! {} kans across {} players", total_kan, players_with_kan);
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
        if is_riichi || !is_closed || points.0 < 1000 || wall.0.len() < 4 {
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

            next_state.set(TurnState::CallWindow);
        }
    }
}


pub fn pon_check(
    query: Query<(Entity, &Hand), Without<Riichi>>,
    discard: Query<(&DiscardedTile, &DiscardedBy), With<CurrentDiscard>>,
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


pub fn declare_pon(
    mut messages: MessageReader<DeclarePonMessage>,
    mut query: Query<(&mut Hand, &mut OpenMentsu)>,
    ippatsu_query: Query<Entity, With<Ippatsu>>,
    tile_query: Single<Entity, With<CurrentDiscard>>,
    mut game: ResMut<GameState>,
    mut current_turn: ResMut<CurrentTurn>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut lock: ResMut<CallLock>,
    mut commands: Commands,
) {
    for message in messages.read(){
        if lock.0 { 
            return; 
        }

        if let Ok((mut hand, mut open_mentsu)) = query.get_mut(message.player) 
            && can_declare_pon(&hand.0 ,&message.tile) { 
                lock.0 = true;

                open_mentsu.0.push(Mentsu::Koutsu(vec![message.tile; 3], false));
                println!("{} declares Pon on {:?}", message.player, message.tile);
                for _ in 0..2 {
                    let idx = hand.0.iter().position(|x| *x == message.tile).unwrap();
                    hand.0.remove(idx);
                }
                // this does compile because of non-lexical lifetimes
                // same with chi and kan
                for player in ippatsu_query.iter() {
                    commands.entity(player).remove::<Ippatsu>();
                }
                commands.entity(message.player).remove::<ClosedHand>();
                commands.entity(*tile_query).despawn(); 
                commands.entity(message.player).insert(ForbiddenDiscard(vec![message.tile]));
                game.calls_made = true;
                current_turn.0 = message.player;
                next_state.set(TurnState::MainPhase);
                // timer.0.reset();
        }
    }
}


pub fn chi_check(
    query: Query<(Entity, &Hand, &Jikaze), Without<Riichi>>,
    discard: Query<(&DiscardedTile, &DiscardedBy), With<CurrentDiscard>>,
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
    mut messages: MessageReader<DeclareChiMessage>,
    mut query: Query<(&mut Hand, &mut OpenMentsu, &Jikaze)>,
    ippatsu_query: Query<Entity, With<Ippatsu>>,
    tile_query: Single<Entity, With<CurrentDiscard>>,
    mut game: ResMut<GameState>,
    mut current_turn: ResMut<CurrentTurn>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut lock: ResMut<CallLock>,
    mut commands: Commands,
) {
    for message in messages.read() {
        if lock.0 { 
            return; 
        }

        let is_valid = if let (
            Ok((hand, _, self_jikaze)),
            Ok((_, _, discard_jikaze))
        ) = (
            query.get(message.player),
            query.get(message.discarded_by)
        ) {
            let positions = can_declare_chi(&hand.0, &message.tile);
            !positions.is_empty()
                && positions.contains(&message.pos)
                && self_jikaze.0.is_kamicha_to(&discard_jikaze.0)
        } else {
            false
        };

        if is_valid && let Ok((mut hand, mut open_mentsu, _))= query.get_mut(message.player) {
            lock.0 = true;

            let pos: &ChiTilePos = &message.pos; // let the player choose 
            let tile = &message.tile;

            match pos {
                    ChiTilePos::Middle => {
                        println!("{} declares Chi on {:?} (position: {:?})", message.player, message.tile, message.pos);
                        let next = next_tile_sequence(tile).unwrap();
                        let prev = previous_tile_sequence(tile).unwrap();
                        // use the variables as a pointer for removal first b4 moving the value 
                        hand.remove_tile_from_hand(&next);
                        hand.remove_tile_from_hand(&prev);
                        open_mentsu.0.push(Mentsu::Shuntsu(vec![prev, *tile, next], false));   
                        commands.entity(message.player).insert(ForbiddenDiscard(vec![*tile]));                   
                    },
                    ChiTilePos::Left => {
                        println!("{} declares Chi on {:?} (position: {:?})", message.player, message.tile, message.pos);
                        let next = next_tile_sequence(tile).unwrap();
                        let next_next = next_tile_sequence(&next).unwrap();

                        // https://riichi.wiki/Kuikae
                        let mut forbidden = vec![];
                        if hand.0.contains(tile) { 
                            forbidden.push(*tile); 
                        }
                        if let Some(n) = next_tile_sequence(&next_next) { 
                            forbidden.push(n); 
                        }
                        if !forbidden.is_empty() {
                            commands.entity(message.player).insert(ForbiddenDiscard(forbidden));
                        }

                        hand.remove_tile_from_hand(&next);
                        hand.remove_tile_from_hand(&next_next);
                        open_mentsu.0.push(Mentsu::Shuntsu(vec![*tile, next, next_next], false));
                        
                    },
                    ChiTilePos::Right => {
                        println!("{} declares Chi on {:?} (position: {:?})", message.player, message.tile, message.pos);
                        let prev = previous_tile_sequence(tile).unwrap();
                        let prev_prev = previous_tile_sequence(&prev).unwrap();

                        let mut forbidden = vec![];
                        if hand.0.contains(tile) { 
                            forbidden.push(*tile); 
                        }
                        if let Some(p) = previous_tile_sequence(&prev_prev) { 
                            forbidden.push(p); 
                        }
                        if !forbidden.is_empty() {
                            commands.entity(message.player).insert(ForbiddenDiscard(forbidden));
                        }
            

                        hand.remove_tile_from_hand(&prev);
                        hand.remove_tile_from_hand(&prev_prev);
                        open_mentsu.0.push(Mentsu::Shuntsu(vec![prev_prev, prev, *tile], false));
                    },
            }
            
            commands.entity(message.player).remove::<ClosedHand>();
            commands.entity(*tile_query).despawn(); 
            game.calls_made = true;
            for player in ippatsu_query.iter() {
                commands.entity(player).remove::<Ippatsu>();
            }
            current_turn.0 = message.player;
            next_state.set(TurnState::MainPhase);
            // timer.0.reset();
        }
    }
}


pub fn player_and_total_kan_count(query: &Query<&OpenMentsu>) -> (u8, u8) {
    let mut players_with_kan = 0;
    let mut total_kan = 0;
    for open in query.iter() {
        let kan = open.0.iter()
            .filter(|mentsu| matches!(mentsu, Mentsu::Ankan(_) | Mentsu::Daiminkan(_) | Mentsu::Shouminkan(_)))
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
    discard: Query<(&DiscardedTile, &DiscardedBy), With<CurrentDiscard>>,
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


pub fn declare_kan(
    mut messages: MessageReader<DeclareKanMessage>,
    mut query: Query<(&mut Hand, &mut OpenMentsu, Option<&DrawnTile>)>,
    ippatsu_query: Query<Entity, With<Ippatsu>>,
    tile_query: Query<Entity, With<CurrentDiscard>>,
    mut game: ResMut<GameState>,
    mut dead_wall: ResMut<DeadWall>,
    mut current_turn: ResMut<CurrentTurn>,
    mut next_state: ResMut<NextState<TurnState>>,
    mut lock: ResMut<CallLock>,
    mut commands: Commands
) { 
    for message in messages.read() {
        if lock.0 { 
            return; 
        } 

        if let Ok((mut hand, mut open_mentsu, maybe_drawn)) = query.get_mut(message.player){
            lock.0 = true;
            let tile = &message.tile;
            let mut full_hand = hand.0.to_owned();
            if let Some(drawn) = maybe_drawn {
                full_hand.push(drawn.0);
            }
            let count = can_declare_kan_from_hand(&full_hand, tile);
            let mut kan_successful_type: Option<Kantsu> = None;


            if message.is_discard && count == 3 {
                open_mentsu.0.push(Mentsu::Daiminkan(vec![*tile; 4]));
                hand.0.retain(|x| x != tile);
                commands.entity(tile_query.single().unwrap()).despawn(); 
                commands.entity(message.player).remove::<ClosedHand>(); 
                kan_successful_type = Some(Kantsu::Daiminkan);
                game.pending_kan_dora = true;
                game.pending_rinshan = true;
            } 

            else if !message.is_discard && count == 4 {
                open_mentsu.0.push(Mentsu::Ankan(vec![*tile; 4]));
                // dora flipping timing 
                let new_dora = dead_wall.filler_tiles.remove(0);
                let new_ura =  dead_wall.filler_tiles.remove(0);
                dead_wall.dora_indicators.push(new_dora);
                dead_wall.ura_indicators.push(new_ura);
                hand.0.retain(|x| x != tile);
                kan_successful_type = Some(Kantsu::Ankan);
                game.pending_rinshan = true;
            }  

            else if !message.is_discard { // this check should be enough hopefully
                for mentsu in &mut open_mentsu.0 {
                    if let Mentsu::Koutsu(tiles, false) = mentsu && tiles[0] == *tile {
                        // deref to mutate
                        *mentsu = Mentsu::Shouminkan(vec![*tile; 4]);
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

            match kan_successful_type {
                Some(Kantsu::Ankan) => println!("{} declares Ankan on {:?}", message.player, tile),
                Some(Kantsu::Daiminkan) => println!("{} declares Daiminkan on {:?}", message.player, tile),
                Some(Kantsu::Shouminkan) => println!("{} declares Shouminkan on {:?}", message.player, tile),
                None => println!("{} attempted Kan on {:?} but failed", message.player, tile),
            }

            if kan_successful_type == Some(Kantsu::Ankan) || kan_successful_type == Some(Kantsu::Daiminkan)  {
                game.calls_made = true;
                for player in ippatsu_query.iter() {
                    commands.entity(player).remove::<Ippatsu>();
                }
                current_turn.0 = message.player;
                next_state.set(TurnState::RinshanDraw);
                // timer.0.reset();
            } else if kan_successful_type == Some(Kantsu::Shouminkan) {
                for player in ippatsu_query.iter() {
                    commands.entity(player).remove::<Ippatsu>();
                }
                current_turn.0 = message.player;
                next_state.set(TurnState::CallWindow);
                // timer.0.reset();
            }
        }
    }
}


pub fn start_game(
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>
) {
    commands.spawn(Camera2d::default());

    let mut wall = vec![];
    for _ in 0..4 {
        wall.extend(all_tiles());
    }
    wall.shuffle(&mut rand::rng());
    
    let seats = [Wind::East, Wind::South, Wind::West, Wind::North];
    let mut starting_player = Entity::PLACEHOLDER;

    commands.insert_resource(DeadWall {
        dora_indicators: wall.drain(..1).collect(),
        ura_indicators: wall.drain(..1).collect(),
        rinshan_tiles: wall.drain(..4).collect(),
        filler_tiles:wall.drain(..8).collect(),
    });

    for (i, wind) in seats.iter().enumerate() {
        
        let mut starting_hand: Vec<Tile> = wall.drain(wall.len() - 13..).collect();
        starting_hand.sort();
        
        let mut player = commands.spawn((
            PlayerTag,
            Points(25000),
            Jikaze(*wind),
            Hand(starting_hand),
            OpenMentsu(vec![]),
            Kawa(vec![]),
            Alive,
            ClosedHand,
        ));

        if i == 0 {
            player.insert(HumanPlayer);
        }

        if *wind == Wind::East {
            player.insert(Oya);
            starting_player = player.id();
        }
    
    }

    commands.insert_resource(Revolver::new());

    commands.insert_resource(
        GameState { 
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
    
    commands.insert_resource(Wall(wall));
    // commands.insert_resource(CallWindowTimer(Timer::from_seconds(1.0, TimerMode::Once)));
    println!("ゲーム開始");
    next_state.set(TurnState::Draw);
}


pub fn draw_tile(
    current_turn: Res<CurrentTurn>,
    mut wall: ResMut<Wall>,
    mut query: Query<(Entity, Has<Furiten>, Has<Riichi>)>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>, // used to change the game phase
) {
    // this wouldn't cause a panic because the ryuukyoku check would end the game right there and then
    let drawn = wall.0.remove(0); 
    commands.entity(current_turn.0).insert(DrawnTile(drawn));

    if let Ok((player, _, is_riichi)) = query.get(current_turn.0) 
    && !is_riichi {
        commands.entity(player).remove::<Furiten>();
    }
    

    next_state.set(TurnState::MainPhase);

    println!("{} draws {:?}", current_turn.0, drawn);
}


pub fn rinshan_draw(
    current_turn: Res<CurrentTurn>,
    mut wall: ResMut<Wall>,
    mut dead_wall: ResMut<DeadWall>,
    mut game: ResMut<GameState>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    game.pending_rinshan = false;

    let drawn = dead_wall.rinshan_tiles.remove(0);
    println!("{} draws {:?} from rinshan", current_turn.0, drawn);

    commands.entity(current_turn.0).insert(DrawnTile(drawn));
    dead_wall.filler_tiles.push(wall.0.pop().unwrap());

    next_state.set(TurnState::MainPhase);
}


// TODO: ui system (message writer) that greys out forbidden discard tile(s)

pub fn discard_tile(
    mut messages: MessageReader<DiscardTileMessage>,
    mut query: Query<(&mut Hand, Option<&DrawnTile>, &mut Kawa, Option<&mut Riichi>, Option<&ForbiddenDiscard>)>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
    mut game: ResMut<GameState>,
    mut dead_wall: ResMut<DeadWall>
) {
    let mut processed = false;
    for message in messages.read() {
        if processed { continue; }

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
            ));

            if let Some(mut riichi) = maybe_riichi {
                if riichi.turns_since > 0 {
                    commands.entity(message.player).remove::<Ippatsu>();
                }
                riichi.turns_since += 1;
            }

            if game.pending_kan_dora {
                let new_dora = dead_wall.filler_tiles.remove(0);
                let new_ura =  dead_wall.filler_tiles.remove(0);
                dead_wall.dora_indicators.push(new_dora);
                dead_wall.ura_indicators.push(new_ura);
                game.pending_kan_dora = false;
            }

            commands.entity(message.player).remove::<ForbiddenDiscard>();
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
    round_result: Option<Res<RoundResult>>,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    if round_result.is_some() { return; }

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


// for testing
pub fn auto_discard_bot(
    current_turn: Res<CurrentTurn>,
    query: Query<&DrawnTile, Without<HumanPlayer>>,
    mut messages: MessageWriter<DiscardTileMessage>,
) {
    if let Ok(drawn) = query.get(current_turn.0) {

        messages.write(DiscardTileMessage {
            player: current_turn.0,
            tile: drawn.0,
            is_tsumogiri: true,
        });
    }
}

pub fn auto_advance_call_window(
    human_options: Query<(), (
        With<HumanPlayer>,
        Or<(With<RonOption>, With<PonOption>, With<ChiOption>, With<DaiminkanOption>)>
    )>,
    mut game: ResMut<GameState>,
    mut next_state: ResMut<NextState<TurnState>>,
    tile_query: Query<(Entity, &DiscardedTile), With<CurrentDiscard>>,
    furiten_check: Query<(Entity, &Tenpai)>,
    mut commands: Commands,
) {
    // wait for human input
    if !human_options.is_empty() {
        return;
    }

    if let Ok((discard_entity, discarded_tile)) = tile_query.single() {
        for (player, tenpai) in &furiten_check {
            if tenpai.0.contains(&discarded_tile.0) {
                commands.entity(player).insert(Furiten);
            }
        }
        commands.entity(discard_entity).despawn();
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
    mut query: Query<&mut Hand, With<Alive>>,
    alive_check: Query<(), With<Alive>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
    game: Res<GameState>
) {
    if alive_check.iter().count() <= 1 {
        println!("ゲーム終了");
        next_state.set(TurnState::GameOver);
        return;
    }
    println!("--- New Round: {} Bakaze: {:?}, Honba: {} ---", game.rounds, game.bakaze, game.honba);
    let mut wall = vec![];
    for _ in 0..4 {
        wall.extend(all_tiles());
    }
    wall.shuffle(&mut rand::rng());

    commands.insert_resource(DeadWall {
        dora_indicators: wall.drain(..1).collect(),
        ura_indicators: wall.drain(..1).collect(),
        rinshan_tiles: wall.drain(..4).collect(),
        filler_tiles:wall.drain(..8).collect(),
    });

    for mut hand in query {
        let starting_hand: Vec<Tile> = wall.drain(wall.len() - 13..).collect();
        hand.0 = starting_hand;
        hand.0.sort();
    }

    commands.insert_resource(Wall(wall));
    next_state.set(TurnState::Draw);
}

// TODO: Parameter `ResMut<'_, ExecuteQueue>` failed validation: Resource does not exist
pub fn round_cleanup(
    mut query: Query<(Entity, &mut Jikaze, Has<Oya>)>,
    alive_query: Query<&Alive>,
    tile_query: Query<Entity, With<DiscardedTile>>,
    player_query: Query<(Entity, &mut Hand, &mut OpenMentsu, &mut Kawa)>,
    result: Res<RoundResult>,
    mut game: ResMut<GameState>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TurnState>>,
    outcome: Option<Res<RoundOutcome>>
) {
    match &result.0 {
        RoundEndReason::OyaWin => println!("Round end: Oya win (renchan)"),
        RoundEndReason::NonOyaWin => println!("Round end: Non-oya win"),
        RoundEndReason::RyuukyokuOyaTenpai => println!("Round end: Ryuukyoku (oya tenpai, renchan)"),
        RoundEndReason::RyuukyokuOyaNoten => println!("Round end: Ryuukyoku (oya noten)"),
        RoundEndReason::TochuuRyuukyoku => println!("Round end: Tochuu ryuukyoku"),
    }
    match result.0 {
        RoundEndReason::OyaWin | RoundEndReason::RyuukyokuOyaTenpai | RoundEndReason::TochuuRyuukyoku => {game.honba += 1},
        RoundEndReason::NonOyaWin | RoundEndReason::RyuukyokuOyaNoten => { 
            game.honba = 0;
            if game.bakaze == Wind::East && game.rounds == 4 {
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
        commands.entity(player).remove::<KyuushuOption>();  

        commands.entity(player).insert(ClosedHand);
    }
    game.calls_made = false;
    game.pending_kan_dora = false;
    game.pending_rinshan = false;

    commands.remove_resource::<RoundResult>();

    if outcome.is_some() {
        next_state.set(TurnState::Execution);
    } else {
        next_state.set(TurnState::StartNewRound);
    }
}