use bevy::prelude::*;
use rand::{RngExt, seq::IndexedRandom};

use crate::core::*;
use crate::scoring::*;
use crate::components::*;
use crate::resources::*;
use crate::messages::*;


pub fn bot_discard_system(
    current_turn: Res<CurrentTurn>,
    query: Query<(Entity, &DrawnTile, &Hand, &OpenMentsu, Has<RiichiSelecting>), Without<HumanPlayer>>,
    visible_query: Query<(&OpenMentsu, &Kawa, Has<Riichi>)>,
    dead_wall: Res<DeadWall>,
    mut messages: MessageWriter<DiscardTileMessage>,
    mut riichi_writer: MessageWriter<DeclareRiichiMessage>,
    mut commands: Commands,
) {
    if let Ok((player, drawn, hand, open_mentsu, is_selecting)) = query.get(current_turn.0) {
        println!("{} draws {:?}", current_turn.0, drawn.0);

        let mut hand_plus_drawn = hand.0.to_owned();
        hand_plus_drawn.push(drawn.0);

        let mut visible_tiles = [0; 34];
        let mut safe_tiles = vec![];

        for (open, kawa, is_riichi) in visible_query.iter() {
            if is_riichi { 
                safe_tiles.push(kawa.0.to_owned());
             }
            for mentsu in open.0.iter() {
                for tile in mentsu.tiles() {
                    visible_tiles[tile_to_index(tile)] += 1;
                }
            }
            for tile in kawa.0.iter() {
                visible_tiles[tile_to_index(tile)] += 1;
            }
        }  

        for tile in dead_wall.dora_indicators.iter() {
            visible_tiles[tile_to_index(tile)] += 1;
        }

        let discard = evaluate_discard(&hand_plus_drawn, &open_mentsu.0, &visible_tiles, &safe_tiles);

        if is_selecting {
            riichi_writer.write(DeclareRiichiMessage { player, tile: discard });
            commands.entity(player).remove::<RiichiSelecting>();
        } else {
            messages.write(DiscardTileMessage {
                player: current_turn.0,
                tile: discard,
                is_tsumogiri: drawn.0 == discard,
            });
        }
        println!("{} discards {:?}", current_turn.0, discard);
    }
}


pub fn bot_call_system(
    query: Query<(
        Entity,
        &Hand,
        &OpenMentsu,
        Option<&RonOption>,
        Option<&PonOption>,
        Option<&ChiOption>,
        Option<&DaiminkanOption>,
    ),
    Without<HumanPlayer>>,
    human_options: Query<(), (
        With<HumanPlayer>,
        Or<(With<RonOption>, With<PonOption>, With<ChiOption>, With<DaiminkanOption>)>
    )>,
    mut pon_writer: MessageWriter<DeclarePonMessage>,
    mut chi_writer: MessageWriter<DeclareChiMessage>,
    mut kan_writer: MessageWriter<DeclareKanMessage>,
    discard_query: Single<&DiscardedTile, With<CurrentDiscard>>,
    discarded_by: Single<&DiscardedBy, With<CurrentDiscard>>,
    mut commands: Commands,
) {
    let human_is_deciding = !human_options.is_empty();

    for (player, hand, open_mentsu, ron, pon, chi, kan) in query {

        if ron.is_none() && pon.is_none() && chi.is_none() && kan.is_none() {
            continue;
        } else if ron.is_some() {
            commands.entity(player).insert(RonDeclared);
            continue;
        }

        if human_is_deciding {
            continue;
        }

        let mut combined_hand = combine_tiles(&hand.0, &open_mentsu.0);
        let pre_call_shanten = calculate_shanten(&combined_hand);
        
        combined_hand.push(discard_query.0);
        let post_call_shanten = calculate_shanten(&combined_hand);

        if post_call_shanten < pre_call_shanten {
            if let Some(k) = kan {
                kan_writer.write(DeclareKanMessage { player, tile: k.0, is_discard: true });
            } else if let Some(p) = pon {
                pon_writer.write(DeclarePonMessage { player, tile: p.0 });  
            } else if let Some(c) = chi {
                // TODO: just auto-picking the first valid chi position for now. more choice(s) later
                chi_writer.write(DeclareChiMessage {
                    player,
                    tile: c.tile,
                    pos: c.positions[0],
                    discarded_by: discarded_by.0,
                });
            }
        }   
    
    }
}

pub fn bot_main_phase_system(
    query: Query<(
        Entity,
        Option<&TsumoOption>,
        Option<&RiichiOption>,
        Option<&AnkanOption>,
        Option<&ShouminkanOption>,
        Option<&KyuushuOption>,
    ), Without<HumanPlayer>>,
    mut tsumo_writer: MessageWriter<DeclareTsumoMessage>,
    mut kan_writer: MessageWriter<DeclareKanMessage>,
    mut kyuushu_writer: MessageWriter<DeclareKyuushuMessage>,
    mut commands: Commands,
) {
     for (player, tsumo, riichi, ankan, shouminkan, kyuushu) in &query {

        if tsumo.is_none() && riichi.is_none() && ankan.is_none() && shouminkan.is_none() && kyuushu.is_none() {
            continue;
        }
        
        if let Some(t) = tsumo {
            tsumo_writer.write(DeclareTsumoMessage { player, result: t.result.to_owned() });
            break;
        }

        if riichi.is_some() {
            commands.entity(player).insert(RiichiSelecting);
        } else if let Some(a) = ankan {
            kan_writer.write(DeclareKanMessage { player, tile: a.0[0], is_discard: false });
        } else if let Some(s) = shouminkan {
            kan_writer.write(DeclareKanMessage { player, tile: s.0[0], is_discard: false });
        } else if kyuushu.is_some() {
            kyuushu_writer.write(DeclareKyuushuMessage { player });
        }

    } 
}


pub fn bot_cheat_decision_system(
    query: Query<(Entity, &BotProfile), Without<HumanPlayer>>,
    timer: Res<BlackoutTimer>,
    mut commands: Commands,
) {
    let duration = timer.0.duration().as_secs_f32();
    let mut rng = rand::rng();

    for (entity, profile) in &query {
        let probability = profile.cheat_tendency * (duration / 5.0);

        if rng.random::<f32>() < probability {
            let execute_at = rng.random_range(1.0..=duration);
            commands.entity(entity).insert(BotCheatIntent { execute_at });
        }
    }
}

pub fn bot_cheat_execution_system(
    mut query: Query<(Entity, &mut Hand, &OpenMentsu, Option<&mut DrawnTile>, &BotCheatIntent, Has<Riichi>), Without<HumanPlayer>>,
    mut kawa_query: Query<(Entity, &mut Kawa)>,
    open_query: Query<&OpenMentsu>,
    dead_wall: Res<DeadWall>,
    timer: Res<BlackoutTimer>,
    mut cheat_log: ResMut<CheatLog>,
    mut commands: Commands,
) {
    let elapsed = timer.0.elapsed_secs();
    let mut rng = rand::rng();

    for (bot_entity, mut hand, open_mentsu, mut maybe_drawn, intent, is_riichi) in query.iter_mut() {
        if elapsed < intent.execute_at { continue; }

        if hand.0.is_empty() {
            commands.entity(bot_entity).remove::<BotCheatIntent>();
            continue;
        }

        //  hand + drawn tile (if any)
        let mut full_hand = hand.0.clone();
        if let Some(ref drawn) = maybe_drawn {
            full_hand.push(drawn.0);
        }

        let mut visible_tiles = [0i32; 34];
        for (_, kawa) in kawa_query.iter() {
            for tile in &kawa.0 { visible_tiles[tile_to_index(tile)] += 1; }
        }
        for open in open_query.iter() {
            for mentsu in &open.0 {
                for tile in mentsu.tiles() { visible_tiles[tile_to_index(tile)] += 1; }
            }
        }
        for tile in &dead_wall.dora_indicators {
            visible_tiles[tile_to_index(tile)] += 1;
        }

        let combined_current = combine_tiles(&full_hand, &open_mentsu.0);
        let mut current_freq = tiles_to_frequency_array(&combined_current);
        let current_shanten = calculate_shanten_from_array(&mut current_freq);
        let current_ukeire: i32 = ukeire_tiles(&mut current_freq, current_shanten).iter()
            .map(|&j| (4 - current_freq[j] as i32 - visible_tiles[j]).max(0))
            .sum();
        let current_dora = count_dora(&combined_current, &*dead_wall, is_riichi).dora as i32;

        let baseline_score = (current_shanten, -current_ukeire, -current_dora);
        let mut best_score = baseline_score;
        let mut best_swaps = Vec::new();

        // ! consider bot not stealing from their own kawa (makes it easier to figure out the cheater)
        for (target_entity, target_kawa) in kawa_query.iter() {
            for (kawa_idx, kawa_tile) in target_kawa.0.iter().enumerate() {
                for (hand_idx, hand_tile) in full_hand.iter().enumerate() {
                    let mut temp_hand = full_hand.to_owned();
                    temp_hand[hand_idx] = *kawa_tile;

                    let combined_temp = combine_tiles(&temp_hand, &open_mentsu.0);
                    let mut temp_freq = tiles_to_frequency_array(&combined_temp);
                    let shanten = calculate_shanten_from_array(&mut temp_freq);

                    if shanten > best_score.0 { continue; }

                    let ukeire: i32 = ukeire_tiles(&mut temp_freq, shanten).iter()
                        .map(|&j| (4 - temp_freq[j] as i32 - visible_tiles[j]).max(0))
                        .sum();
                    let dora = count_dora(&combined_temp, &*dead_wall, is_riichi).dora as i32;

                    let score = (shanten, -ukeire, -dora);

                    if score < best_score {
                        best_score = score;
                        best_swaps.clear();
                        best_swaps.push((target_entity, kawa_idx, *kawa_tile, hand_idx, *hand_tile));
                    } else if score == best_score && score < baseline_score {
                        best_swaps.push((target_entity, kawa_idx, *kawa_tile, hand_idx, *hand_tile));
                    }

                }
            }
        }

        if !best_swaps.is_empty() {
            let swap = best_swaps.choose(&mut rng).unwrap();
            let (target_entity, kawa_idx, stolen_tile, hand_idx, bot_tile) = *swap;

            if let Ok((_, mut target_kawa)) = kawa_query.get_mut(target_entity) {
                target_kawa.0[kawa_idx] = bot_tile;

                if hand_idx < hand.0.len() {
                    hand.0[hand_idx] = stolen_tile;
                    hand.0.sort();
                } else if let Some(ref mut drawn) = maybe_drawn {
                    drawn.0 = stolen_tile;
                }

                cheat_log.0.push(CheatEntry {
                    cheater: bot_entity,
                    target_kawa: target_entity,
                    tile_taken: stolen_tile,
                    tile_left: bot_tile,
                });

                println!("Cheat: Bot {:?} grabbed {:?} (gave {:?}) from {:?}",
                    bot_entity, stolen_tile, bot_tile, target_entity);
            }
        }

        commands.entity(bot_entity).remove::<BotCheatIntent>();
    }
}
