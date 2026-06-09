use bevy::prelude::*;
use rand::{RngExt, seq::IndexedRandom};

use crate::core::*;
use crate::scoring::*;
use crate::components::*;
use crate::resources::*;
use crate::messages::*;



// TODO: make bots more assertive to yakuhai calls (or just call in general) if their hand already has other yaku

pub fn bot_discard_system(
    current_turn: Res<CurrentTurn>,
    query: Query<(
        Entity, Option<&DrawnTile>, &Hand, &OpenMentsu, &BotProfile,
        Has<RiichiSelecting>, Option<&ForbiddenDiscard>, Option<&RiichiOption>, Has<Riichi>
    ), (
        Without<HumanPlayer>,
        Without<TsumoOption>,
        Without<AnkanOption>,
        Without<ShouminkanOption>,
        Without<KyuushuOption>
    )>,
    visible_query: Query<(Entity, &OpenMentsu, &Kawa, Has<Riichi>)>,
    wall: Res<Wall>,
    dead_wall: Res<DeadWall>,
    revolver: Res<Revolver>,
    mut messages: MessageWriter<DiscardTileMessage>,
    mut riichi_writer: MessageWriter<DeclareRiichiMessage>,
    mut commands: Commands,
) {
    if let Ok((
        player, maybe_drawn, 
        hand, open_mentsu, 
        profile, is_selecting, 
        maybe_forbidden, maybe_riichi, is_riichi)) = query.get(current_turn.0) {

        if is_riichi {
            if let Some(drawn) = maybe_drawn {
                messages.write(DiscardTileMessage {
                    player: current_turn.0,
                    tile: drawn.0,
                    is_tsumogiri: true,
                });
            }
            return;
        }
        
        let mut hand_plus_drawn = hand.0.to_owned();
        if let Some(drawn) = maybe_drawn {
            hand_plus_drawn.push(drawn.0);
        }

        let mut visible_tiles = [0; 34];
        let mut threat_discards = vec![];
        let mut safe_tiles = vec![];
        let mut is_threat = false;
        let mut rng = rand::rng();

        // using composure as a stat penalty 
        let effective_read = ((profile.read * profile.composure).max(profile.read - 0.2)).max(0.2);
        let effective_aggressiveness = ((profile.aggressiveness * profile.composure).max(profile.aggressiveness - 0.2)).max(0.2);

        // check visible tiles and identify threats
        for (entity, open, kawa, is_riichi) in visible_query.iter() {
            // skip own open mentsu (calculated in `evaluate_discard`)
            if entity != player {
                for mentsu in open.0.iter() {
                    for tile in mentsu.tiles() {
                        visible_tiles[tile_to_index(tile)] += 1;
                    }
                }
            }
            for tile in kawa.0.iter() {
                visible_tiles[tile_to_index(tile)] += 1;
            }

            if is_riichi || open.0.len() >= 3 {
                is_threat = true;
                threat_discards.extend(kawa.0.clone());
            }
        }

        for tile in dead_wall.dora_indicators.iter() {
            visible_tiles[tile_to_index(tile)] += 1;
        }

        let mut should_defend = false;
        let current_shanten = calculate_shanten(&combine_tiles(&hand_plus_drawn, &open_mentsu.0));

        if is_threat {
            let panic_threshold = profile.composure + (effective_aggressiveness * 0.20);
            let panic = rng.random::<f32>() > panic_threshold;

            
            should_defend = panic || current_shanten > 1;

            // genbutsu 
            for tile in &threat_discards {
                if rng.random::<f32>() <= effective_read + 0.1 { // base addition
                    safe_tiles.push(*tile);
                }
            }

            // dead honors (3+ visible)
            for i in 27..34 {
                if visible_tiles[i] >= 3 {
                    let honor_tile = index_to_tile(i);
                    if rng.random::<f32>() <= effective_read + 0.1 {
                        safe_tiles.push(honor_tile);
                    }
                }
            }

            // suji defense
            if effective_read >= 0.5 {
                for tile in &threat_discards {
                    match tile {
                        Tile::Man(4) => { safe_tiles.push(Tile::Man(1)); safe_tiles.push(Tile::Man(7)); }
                        Tile::Man(5) => { safe_tiles.push(Tile::Man(2)); safe_tiles.push(Tile::Man(8)); }
                        Tile::Man(6) => { safe_tiles.push(Tile::Man(3)); safe_tiles.push(Tile::Man(9)); }
                        Tile::Pin(4) => { safe_tiles.push(Tile::Pin(1)); safe_tiles.push(Tile::Pin(7)); }
                        Tile::Pin(5) => { safe_tiles.push(Tile::Pin(2)); safe_tiles.push(Tile::Pin(8)); }
                        Tile::Pin(6) => { safe_tiles.push(Tile::Pin(3)); safe_tiles.push(Tile::Pin(9)); }
                        Tile::Sou(4) => { safe_tiles.push(Tile::Sou(1)); safe_tiles.push(Tile::Sou(7)); }
                        Tile::Sou(5) => { safe_tiles.push(Tile::Sou(2)); safe_tiles.push(Tile::Sou(8)); }
                        Tile::Sou(6) => { safe_tiles.push(Tile::Sou(3)); safe_tiles.push(Tile::Sou(9)); }
                        _ => {}
                    }
                }
            }
        }

        let mut forbidden_tiles = maybe_forbidden.map(|f| f.0.clone()).unwrap_or_default();
        if is_selecting && let Some(riichi) = maybe_riichi {
            for tile in &hand_plus_drawn {
                if !riichi.0.contains(tile) {
                    forbidden_tiles.push(*tile);
                }
            }
        }
        let forbidden_slice = if forbidden_tiles.is_empty() { None } else { Some(forbidden_tiles.as_slice()) };

        let target_yaku = determine_bot_strategy(&hand_plus_drawn, revolver.chamber, profile, wall.0.len(), current_shanten);

        let discard = evaluate_discard(
            &hand_plus_drawn, &open_mentsu.0,
            &visible_tiles, &safe_tiles,
            should_defend, forbidden_slice,
            target_yaku
        );

        if is_selecting {
            riichi_writer.write(DeclareRiichiMessage { player, tile: discard });
            commands.entity(player).remove::<RiichiSelecting>();
        } else {
            messages.write(DiscardTileMessage {
                player: current_turn.0,
                tile: discard,
                is_tsumogiri: maybe_drawn.is_some_and(|drawn| drawn.0 == discard),
            });
        }
    }
}


// TODO: Add chanta/junchan, ryankantsu, san/suuankou, pinfu, sanshoku
pub fn determine_bot_strategy(
    hand: &[Tile],
    revolver_chamber: u8,
    profile: &BotProfile,
    wall_len: usize,
    current_shanten: i32,
) -> TargetYaku {
    let death_risk = 1.0 / (7.0 - revolver_chamber as f32);
    let refuses_cheap_win = death_risk > (profile.aggressiveness * profile.composure);

    // for ngmi hand
    // TODO: needs adjustments
    let panic_wall_threshold = match current_shanten {
        3.. => 40,
        2 => 25,
        1 => 10,
        _ => 0,
    };

    if wall_len < panic_wall_threshold {
        return TargetYaku::Speed;
    }

    let mut freq = [0u8; 34];
    for tile in hand {
        freq[tile_to_index(tile)] += 1;
    }

    let man_count: u8 = (0..9).map(|i| freq[i]).sum();
    let pin_count: u8 = (9..18).map(|i| freq[i]).sum();
    let sou_count: u8 = (18..27).map(|i| freq[i]).sum();
    let honor_count: u8 = (27..34).map(|i| freq[i]).sum();
    let pairs = freq.iter().filter(|&&f| f >= 2).count();

    // count unique orphans for kokushi
    let mut unique_yaochuuhai = 0;
    const YAOCHUUHAI_POS: [usize; 13] =[0, 8, 9, 17, 18, 26, 27, 28, 29, 30, 31, 32, 33];
    for &i in YAOCHUUHAI_POS.iter() {
        if freq[i] > 0 { unique_yaochuuhai += 1; }
    }

    //kokushi musou
    if unique_yaochuuhai >= 9 { return TargetYaku::Kokushi; }

    // chinitsu
    if man_count >= 10 { return TargetYaku::Chinitsu(Suit::Man); }
    if pin_count >= 10 { return TargetYaku::Chinitsu(Suit::Pin); }
    if sou_count >= 10 { return TargetYaku::Chinitsu(Suit::Sou); }

    // honitsu
    if man_count + honor_count >= 9 { return TargetYaku::Honitsu(Suit::Man); }
    if pin_count + honor_count >= 9 { return TargetYaku::Honitsu(Suit::Pin); }
    if sou_count + honor_count >= 9 { return TargetYaku::Honitsu(Suit::Sou); }

     // ittsuu
    for (suit_offset, suit) in [(0, Suit::Man), (9, Suit::Pin), (18, Suit::Sou)] {
        let block1 = freq[suit_offset] + freq[suit_offset+1] + freq[suit_offset+2];
        let block2 = freq[suit_offset+3] + freq[suit_offset+4] + freq[suit_offset+5];
        let block3 = freq[suit_offset+6] + freq[suit_offset+7] + freq[suit_offset+8];
        if block1 > 0 && block2 > 0 && block3 > 0 && (block1 + block2 + block3) >= 7 {
            return TargetYaku::Ittsuu(suit);
        }
    }

    // sanshoku doujun
    let mut best_sanshoku: Option<(u8, u8)> = None; // (starting number, total coverage)
    for i in 0..7 {
        let man = freq[i] + freq[i+1] + freq[i+2];
        let pin = freq[i+9] + freq[i+10] + freq[i+11];
        let sou = freq[i+18] + freq[i+19] + freq[i+20];
        let total = man + pin + sou;
        if man > 0 && pin > 0 && sou > 0 && total >= 6 
        && best_sanshoku.is_none_or(|(_n, prev)| total > prev) {
            best_sanshoku = Some(((i + 1) as u8, total));
        }
    }
    if let Some((n, _)) = best_sanshoku {
        return TargetYaku::SanshokuDoujun(n);
    }

    // junchan / chanta
    let chanta_tiles = hand.iter().filter(|t| matches!(t,
        Tile::Man(1|2|3|7|8|9) | Tile::Pin(1|2|3|7|8|9) | Tile::Sou(1|2|3|7|8|9) | Tile::Honor(_)
    )).count();
    let junchan_tiles = hand.iter().filter(|t| matches!(t,
        Tile::Man(1|2|3|7|8|9) | Tile::Pin(1|2|3|7|8|9) | Tile::Sou(1|2|3|7|8|9)
    )).count();

    if junchan_tiles >= 10 && honor_count == 0 { return TargetYaku::Junchan; }
    if chanta_tiles >= 10 { return TargetYaku::Chanta; }

    // pinfu
    if pairs <= 2 && refuses_cheap_win {
        return TargetYaku::Pinfu;
    }

    // pairs (chiitoitsu / toitoi)
    if pairs >= 4 { return TargetYaku::Pairs; } // raised to 4 to avoid premature pair locking

    // tanyao
    let middle_tiles = hand.iter().filter(|t| !is_yaochuuhai(t)).count() as u8;
    if middle_tiles >= 9 { return TargetYaku::Tanyao; }

    // fallback
    if refuses_cheap_win {
        TargetYaku::Tanyao
    } else {
        TargetYaku::Speed
    }
}


// TODO: needs testing
// also make this scale with bullet
pub fn bot_call_system(
    query: Query<(
        Entity, &BotProfile, &Hand, &OpenMentsu, &Jikaze,
        Option<&RonOption>, Option<&PonOption>, Option<&ChiOption>, Option<&DaiminkanOption>,
    ), (
        Without<HumanPlayer>,
        Without<RonDeclared>,       // lock in decisions
        Without<PonDeclared>,       
        Without<ChiDeclared>,       
        Without<DaiminkanDeclared>  
    )>,
    game: Res<GameState>,
    dead_wall: Res<DeadWall>,
    revolver: Res<Revolver>,
    mut commands: Commands,
) {
    for (player, profile, hand, open_mentsu, jikaze, ron, pon, chi, kan) in query.iter() {
        if ron.is_none() && pon.is_none() && chi.is_none() && kan.is_none() { continue; }

        let death_risk = 1.0 / (7.0 - revolver.chamber as f32);
        let refuses_cheap_win = death_risk > (profile.aggressiveness * profile.composure);

        if let Some(r) = ron {
            if refuses_cheap_win && r.result.total_han < 2 {
                commands.entity(player).remove::<RonOption>();
            } else {
                commands.entity(player)
                    .insert(RonDeclared)
                    .remove::<PonOption>()
                    .remove::<ChiOption>()
                    .remove::<DaiminkanOption>();
                continue;
            }
        }

        if pon.is_none() && chi.is_none() && kan.is_none() { continue; }

        let pre_shanten = calculate_shanten(&combine_tiles(&hand.0, &open_mentsu.0));
        let is_closed = open_mentsu.0.is_empty();

        // scores a simulated post-call hand (now returns a 4-tuple to isolate dora)
        let score = |hand_tiles: &[Tile], open: &[Mentsu]| -> (i32, i32, i32, i32) {
            let combined = combine_tiles(hand_tiles, open);
            let mut freq = tiles_to_frequency_array(&combined);
            let shanten = calculate_shanten_from_array(&mut freq);
            let ukeire = ukeire_tiles(&mut freq, shanten).len() as i32;
            let han = estimate_yaku_han(&combined, &jikaze.0, &game.bakaze) as i32;
            let dora = count_dora(&combined, &*dead_wall, false).dora as i32;
            (han, -shanten, ukeire, dora)
        };

        // candidates: (score, action_index)
        // 0 = kan, 1 = pon, 2+ = chi at positions[idx - 2]
        let mut candidates: Vec<((i32, i32, i32, i32), usize)> = vec![];

        if let Some(k) = kan {
            let mut temp_hand = hand.0.clone();
            temp_hand.retain(|t| *t != k.0);
            let mut temp_open = open_mentsu.0.clone();
            temp_open.push(Mentsu::Daiminkan([k.0; 4]));
            candidates.push((score(&temp_hand, &temp_open), 0));
        }

        if let Some(p) = pon {
            let mut temp_hand = Hand(hand.0.clone());
            for _ in 0..2 { temp_hand.remove_tile_from_hand(&p.0); }
            let mut temp_open = open_mentsu.0.clone();
            temp_open.push(Mentsu::Koutsu([p.0; 3], false));
            candidates.push((score(&temp_hand.0, &temp_open), 1));
        }

        if let Some(c) = chi {
            for (i, &pos) in c.positions.iter().enumerate() {
                let (first, second, shuntsu_array) = match pos {
                    ChiTilePos::Left => {
                        let next = next_tile_sequence(&c.tile).unwrap();
                        let next_next = next_tile_sequence(&next).unwrap();
                        (next, next_next, [c.tile, next, next_next])
                    },
                    ChiTilePos::Middle => {
                        let prev = previous_tile_sequence(&c.tile).unwrap();
                        let next = next_tile_sequence(&c.tile).unwrap();
                        (prev, next,[prev, c.tile, next])
                    },
                    ChiTilePos::Right => {
                        let prev = previous_tile_sequence(&c.tile).unwrap();
                        let prev_prev = previous_tile_sequence(&prev).unwrap();
                        (prev_prev, prev,[prev_prev, prev, c.tile])
                    },
                };
                let mut temp_hand = Hand(hand.0.clone());
                temp_hand.remove_tile_from_hand(&first);
                temp_hand.remove_tile_from_hand(&second);
                let mut temp_open = open_mentsu.0.clone();
                temp_open.push(Mentsu::Shuntsu(shuntsu_array, false));
                candidates.push((score(&temp_hand.0, &temp_open), 2 + i));
            }
        }

        let Some(&(best_score, best_idx)) = candidates.iter().max_by_key(|(s, _)| *s) else {
            // bot skips if no candidates
            commands.entity(player)
                .remove::<RonOption>()
                .remove::<PonOption>()
                .remove::<ChiOption>()
                .remove::<DaiminkanOption>();
            continue;
        };

        let (best_han, neg_shanten, _, best_dora) = best_score;
        let post_shanten = -neg_shanten;


        let mut rng = rand::rng();

        // scale aggressiveness drastically based on estimated han value
        let total_estimated_han = best_han + best_dora;
        let value_multiplier = 1.0 + (total_estimated_han as f32 * 0.2);

        let shanten_chance = (profile.aggressiveness * value_multiplier / (post_shanten as f32 + 1.0)) * profile.speed;
        let dora_chance = (profile.speed * value_multiplier) + (best_dora as f32 * 0.25);
        
        let ruins_closed_tenpai = is_closed && pre_shanten == 0;
        let is_cheap_and_risky = refuses_cheap_win && total_estimated_han < 2;

        if !ruins_closed_tenpai 
        && !is_cheap_and_risky 
        && pre_shanten >= post_shanten 
        && best_han > 0 
        && (rng.random::<f32>() < shanten_chance || rng.random::<f32>() < dora_chance) {
            match best_idx {
                0 => {
                    commands.entity(player)
                        .insert(DaiminkanDeclared)
                        .remove::<PonOption>()
                        .remove::<ChiOption>();
                },
                1 => {
                    commands.entity(player)
                        .insert(PonDeclared)
                        .remove::<DaiminkanOption>()
                        .remove::<ChiOption>();
                },
                _ => {
                    let c = chi.unwrap();
                    commands.entity(player)
                        .insert(ChiDeclared(c.positions[best_idx - 2]))
                        .remove::<PonOption>()
                        .remove::<DaiminkanOption>();
                }
            }
        } else {
            // bot decides to skip
            commands.entity(player)
                .remove::<RonOption>()
                .remove::<PonOption>()
                .remove::<ChiOption>()
                .remove::<DaiminkanOption>();
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
        Option<&Hand>,       
        Option<&DrawnTile>,  
        Option<&BotProfile>, 
    ), Without<HumanPlayer>>,
    visible_query: Query<(Entity, Has<Riichi>)>,
    mut tsumo_writer: MessageWriter<DeclareTsumoMessage>,
    mut kan_writer: MessageWriter<DeclareKanMessage>,
    mut kyuushu_writer: MessageWriter<DeclareKyuushuMessage>,
    mut commands: Commands,
) {
     for (player, tsumo, riichi, ankan, shouminkan, kyuushu, hand_opt, drawn_opt, profile_opt) in &query {

        if tsumo.is_none() && riichi.is_none() && ankan.is_none() && shouminkan.is_none() && kyuushu.is_none() {
            continue;
        }

        if let Some(t) = tsumo {
            tsumo_writer.write(DeclareTsumoMessage { player, result: t.result.to_owned() });
            break;
        }

        if let Some(r) = riichi {
            let mut should_riichi = true;

            if let (Some(hand), Some(drawn), Some(profile)) = (hand_opt, drawn_opt, profile_opt) {
                let mut full_hand = hand.0.clone();
                full_hand.push(drawn.0);

                // calculate best wait quality
                let mut max_wait_types = 0;
                for discard in &r.0 {
                    let mut temp = full_hand.clone();
                    if let Some(pos) = temp.iter().position(|x| *x == *discard) {
                        temp.remove(pos);
                    }
                    let waits = check_tenpai(&temp);
                    max_wait_types = max_wait_types.max(waits.len());
                }

                // check board danger
                let someone_riichi = visible_query.iter().any(|(e, is_riichi)| e != player && is_riichi);

                // discourage lower opt wait
                if max_wait_types <= 1 {
                    if someone_riichi && profile.aggressiveness < 0.7 {
                        should_riichi = false; // damaten
                    } else if profile.aggressiveness < 0.5 {
                        should_riichi = false; // tegawari
                    }
                }
            }

            if should_riichi {
                commands.entity(player).insert(RiichiSelecting);
            }
        }
        else if let Some(a) = ankan {
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

    if duration >= 2.0 {
        for (entity, profile) in &query {
            let probability = profile.cheat_tendency * (duration / 5.0);

            if rng.random::<f32>() < probability {
                let execute_at = rng.random_range(1.0..=duration);
                commands.entity(entity).insert(BotCheatIntent { execute_at });
            }
        }
    }
}

pub fn bot_cheat_execution_system(
    mut query: Query<(Entity, &mut Hand, &OpenMentsu, Option<&mut DrawnTile>, &BotCheatIntent, Has<Riichi>), Without<HumanPlayer>>,
    mut kawa_query: Query<(Entity, &mut Kawa)>,
    open_query: Query<(Entity, &OpenMentsu)>,
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
        for (entity, open) in open_query.iter() {
            if entity == bot_entity { continue; }
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
        let mut best_swaps = vec![];

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


pub fn bot_accusation_decision_system(
    query: Query<(Entity, &BotProfile, &Jikaze), (Without<HumanPlayer>, With<Alive>)>,
    all_alive: Query<(Entity, &Jikaze), With<Alive>>,
    jikaze_query: Query<&Jikaze>,
    snapshot: Res<KawaSnapshot>,
    kawa_query: Query<(Entity, &Kawa, &Jikaze), With<Alive>>,
    cheat_log: Res<CheatLog>,
    revolver: Res<Revolver>,
    timer: Res<AccusationTimer>,
    mut commands: Commands,
) {
    let mut rng = rand::rng();
    let death_risk = 1.0 / (7 - revolver.chamber) as f32;

    for (bot_entity, profile, bot_jikaze) in &query {
        
        let mut detected_tampering: Vec<Entity> = vec![];

        // scan kawa
        for (snap_entity, snap_kawa) in &snapshot.all_kawa {
            let Ok(target_jikaze) = jikaze_query.get(*snap_entity) else { continue };
            let distance = bot_jikaze.0.distance_to(&target_jikaze.0);

            let can_remember = match distance {
                0 => true,
                1 | 3 => profile.read >= 0.4, // kamicha | shimocha
                2 => profile.read >= 0.8,
                _ => false,
            };

            if !can_remember { continue; }

            if let Some((_, current_kawa, _)) = kawa_query.iter().find(|(e, _, _)| *e == *snap_entity) 
            && current_kawa.0.len() == snap_kawa.len()
            && current_kawa.0.iter().zip(snap_kawa.iter()).any(|(a, b)| a != b) {
                detected_tampering.push(*snap_entity);
            }
    
        }

        if detected_tampering.is_empty() { continue; }

        // weigh read to get confidence
        // TODO: confidence needs to be lowered a bit
        let (suspect, confidence) = if rng.random::<f32>() < profile.read {
            let suspect = cheat_log.0.iter()
                .find(|e| detected_tampering.contains(&e.target_kawa))
                .map(|e| e.cheater);
            (suspect, 0.8) // prev: 0.9
        } else {
            let others: Vec<Entity> = all_alive.iter()
                .filter(|(e, _)| *e != bot_entity)
                .map(|(e, _)| e)
                .collect();
            (others.choose(&mut rng).copied(), 0.5)
        };

        let Some(suspect) = suspect else { continue };
        if suspect == bot_entity { continue; }

        // willingness check
        let willingness = profile.aggressiveness * confidence;
        if willingness <= death_risk { continue; }

        let duration = timer.0.duration().as_secs_f32();
        let accuse_at = rng.random_range(0.5..=(duration - 0.5).max(0.5));

        commands.entity(bot_entity).insert(BotAccusationIntent {
            suspect,
            confidence,
            accuse_at,
        });

        println!("Bot {:?} plans to accuse {:?} at {:.1}s (conf {:.2})",
            bot_entity, suspect, accuse_at, confidence);
    }
}


pub fn bot_accusation_execution_system(
    query: Query<(Entity, &BotAccusationIntent), With<Alive>>,
    timer: Res<AccusationTimer>,
    mut accuse_writer: MessageWriter<AccuseCheatMessage>,
    mut commands: Commands,
) {
    let elapsed = timer.0.elapsed_secs();

    let mut earliest: Option<(Entity, &BotAccusationIntent)> = None;

    for (entity, intent) in &query {
        if elapsed < intent.accuse_at {
            continue;
        }

        if earliest.is_none() || intent.accuse_at < earliest.unwrap().1.accuse_at {
            earliest = Some((entity, intent));
        }
    }

    if let Some((accuser, intent)) = earliest {
        accuse_writer.write(AccuseCheatMessage {
            accuser,
            suspect: intent.suspect,
        });

        for (entity, _) in &query {
            commands.entity(entity).remove::<BotAccusationIntent>();
        }
    }
}
