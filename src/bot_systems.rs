use bevy::prelude::*;
use rand::{RngExt, seq::IndexedRandom};

use crate::core::*;
use crate::scoring::*;
use crate::components::*;
use crate::resources::*;
use crate::messages::*;



// TODO: make bots more assertive to yakuhai calls (or just call in general) if their hand already has other yaku

pub fn determine_bot_strategy(
    hand: &[Tile],
    revolver_chamber: u8,
    profile: &BotProfile,
    wall_len: usize,
    current_shanten: i32,
    dora_indicators: &[Tile],
) -> TargetYaku {
    let mut dora_count = 0;
    for tile in hand {
        for indicator in dora_indicators {
            if *tile == get_dora_from_indicator(indicator) {
                dora_count += 1;
            }
        }
    }

    let death_risk = 1.0 / (7.0 - revolver_chamber as f32);

    let is_high_value = dora_count >= 2;
    let refuses_cheap_win = !is_high_value && death_risk > (profile.aggressiveness * profile.composure);

    // scale strategic decision with stats
    let effective_aggressiveness = profile.aggressiveness * profile.composure;
    let greed_discount = (effective_aggressiveness * 2.0).round() as u8;

    let panic_multiplier = 1.0 + (1.0 - profile.composure);
    let speed_panic_bonus = (profile.speed * 15.0 * panic_multiplier).round() as usize;

    let panic_wall_threshold = match current_shanten {
        3.. => 35 + speed_panic_bonus,
        2 => 20 + speed_panic_bonus,
        1 => 10 + (speed_panic_bonus / 2),
        _ => 0,
    };

    if wall_len < panic_wall_threshold {
        return TargetYaku::Speed;
    }

    if dora_count >= 3 {
        let tanyao_req = 8 - greed_discount;
        let middle_tiles = hand.iter().filter(|t| !is_yaochuuhai(t)).count() as u8;
        if middle_tiles >= tanyao_req { return TargetYaku::Tanyao; }
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
    let pairs = freq.iter().filter(|&&f| f == 2).count();
    let triplets = freq.iter().filter(|&&f| f >= 3).count();

    // count unique orphans for kokushi
    let mut unique_yaochuuhai = 0;
    const YAOCHUUHAI_POS: [usize; 13] =[0, 8, 9, 17, 18, 26, 27, 28, 29, 30, 31, 32, 33];
    for &i in YAOCHUUHAI_POS.iter() {
        if freq[i] > 0 { unique_yaochuuhai += 1; }
    }

    // kokushi
    let kokushi_threshold = 10 - greed_discount;
    if unique_yaochuuhai >= kokushi_threshold { return TargetYaku::Kokushi; }

    // suuankou
    if triplets == 3 && pairs >= (1u8.saturating_sub(greed_discount / 2)).into() { return TargetYaku::Suuankou; }

    // daisangen
    let daisangen_req = if profile.aggressiveness < 0.4 { 3 } else { 2 };
    let mut dragon_pairs = 0;
    let mut dragon_triplets = 0;
    for &h in &[31, 32, 33] { // white, green, red
        if freq[h] >= 3 { dragon_triplets += 1; }
        else if freq[h] == 2 { dragon_pairs += 1; }
    }
    if dragon_triplets + dragon_pairs >= daisangen_req { return TargetYaku::Daisangen; }

    // chuuren poutou
    let chuuren_req = 12 - greed_discount;
    for (suit_offset, suit, count) in[(0, Suit::Man, man_count), (9, Suit::Pin, pin_count), (18, Suit::Sou, sou_count)] {
        if count >= chuuren_req && freq[suit_offset] >= 1 && freq[suit_offset + 8] >= 1 {
            return TargetYaku::ChuurenPoutou(suit);
        }
    }

    // tsuusiou
    let flush_req = 10 - greed_discount;
    if honor_count >= flush_req { return TargetYaku::Tsuuiisou; }

    // ryuuisou
    let green_count = hand.iter().filter(|t| is_green(t)).count() as u8;
    if green_count >= flush_req { return TargetYaku::Ryuuiisou; }

    // chinitsu
    if man_count >= flush_req { return TargetYaku::Chinitsu(Suit::Man); }
    if pin_count >= flush_req { return TargetYaku::Chinitsu(Suit::Pin); }
    if sou_count >= flush_req { return TargetYaku::Chinitsu(Suit::Sou); }

    // honitsu
    let half_flush_req = 9 - greed_discount;
    if man_count + honor_count >= half_flush_req { return TargetYaku::Honitsu(Suit::Man); }
    if pin_count + honor_count >= half_flush_req { return TargetYaku::Honitsu(Suit::Pin); }
    if sou_count + honor_count >= half_flush_req { return TargetYaku::Honitsu(Suit::Sou); }

    // ittsuu
    let ittsuu_req = 7 - (greed_discount / 2);
    for (suit_offset, suit) in[(0, Suit::Man), (9, Suit::Pin), (18, Suit::Sou)] {
        let block1 = freq[suit_offset] + freq[suit_offset+1] + freq[suit_offset+2];
        let block2 = freq[suit_offset+3] + freq[suit_offset+4] + freq[suit_offset+5];
        let block3 = freq[suit_offset+6] + freq[suit_offset+7] + freq[suit_offset+8];
        if block1 > 0 && block2 > 0 && block3 > 0 && (block1 + block2 + block3) >= ittsuu_req {
            return TargetYaku::Ittsuu(suit);
        }
    }

    // sanshoku doujun
    let sanshoku_req = 6 - (greed_discount / 2);
    let mut best_sanshoku: Option<(u8, u8)> = None;
    for i in 0..7 {
        let man = freq[i] + freq[i+1] + freq[i+2];
        let pin = freq[i+9] + freq[i+10] + freq[i+11];
        let sou = freq[i+18] + freq[i+19] + freq[i+20];
        let total = man + pin + sou;
        if man > 0 && pin > 0 && sou > 0 && total >= sanshoku_req
        && best_sanshoku.is_none_or(|(_n, prev)| total > prev) {
            best_sanshoku = Some(((i + 1) as u8, total));
        }
    }
    if let Some((n, _)) = best_sanshoku {
        return TargetYaku::SanshokuDoujun(n);
    }

    // junchan/chanta
    let chanta_tiles = hand.iter().filter(|t| matches!(t,
        Tile::Man(1|2|3|7|8|9) | Tile::Pin(1|2|3|7|8|9) | Tile::Sou(1|2|3|7|8|9) | Tile::Honor(_)
    )).count() as u8;
    let junchan_tiles = hand.iter().filter(|t| matches!(t,
        Tile::Man(1|2|3|7|8|9) | Tile::Pin(1|2|3|7|8|9) | Tile::Sou(1|2|3|7|8|9)
    )).count() as u8;
    let chanta_req = 10 - greed_discount;
    if junchan_tiles >= chanta_req && honor_count == 0 { return TargetYaku::Junchan; }
    if chanta_tiles >= chanta_req { return TargetYaku::Chanta; }

    // pinfu
    if pairs <= 2 && refuses_cheap_win { return TargetYaku::Pinfu; }

    // chiitoitsu
    if pairs >= 4 && triplets == 0 { return TargetYaku::Chiitoitsu; }

    // sanankou
    if triplets == 2 && pairs >= 1 { return TargetYaku::Sanankou; }

    // toiotoi
    if (3..=4).contains(&pairs) && triplets >= 1 { return TargetYaku::Toitoi; }

    // tanyao
    let tanyao_req = 9 - greed_discount;
    let middle_tiles = hand.iter().filter(|t| !is_yaochuuhai(t)).count() as u8;
    if middle_tiles >= tanyao_req { return TargetYaku::Tanyao; }

    // fallback
    if refuses_cheap_win {
        TargetYaku::Tanyao
    } else {
        TargetYaku::Speed
    }
}


pub fn bot_discard_system(
    current_turn: Res<CurrentTurn>,
    query: Query<(
        Entity, Option<&DrawnTile>, &Hand, &OpenMentsu, &BotProfile,
        Has<RiichiSelecting>, Option<&ForbiddenDiscard>, Option<&RiichiOption>, Has<Riichi>,
        &Kawa
    ), (
        Without<HumanPlayer>,
        Without<TsumoOption>,
        Without<AnkanOption>,
        Without<ShouminkanOption>,
        Without<KyuushuOption>
    )>,
    visible_query: Query<(Entity, &OpenMentsu, &Kawa, Has<Riichi>)>,
    wall: Res<Wall>,
    revolver: Res<Revolver>,
    mut messages: MessageWriter<DiscardTileMessage>,
    mut riichi_writer: MessageWriter<DeclareRiichiMessage>,
    mut commands: Commands,
) {
    if let Ok((
        player, maybe_drawn, hand, 
        open_mentsu, profile, is_selecting,
        maybe_forbidden, maybe_riichi, is_riichi, kawa 
    )) = query.get(current_turn.0) {

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

        for tile in wall.get_dora_indicators().iter() {
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

        let target_yaku = determine_bot_strategy(
            &hand_plus_drawn, revolver.chamber, 
            profile, wall.remaining_draws(), 
            current_shanten, &wall.get_dora_indicators(),
        );

        let discard = evaluate_discard(
            &hand_plus_drawn, &open_mentsu.0,
            &visible_tiles, &safe_tiles,
            should_defend, forbidden_slice,
            target_yaku,
            &wall.get_dora_indicators(),
            &kawa.0
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


// TODO: needs testing
// also make this scale with bullet
pub fn bot_call_system(
    query: Query<(
        Entity, &BotProfile, &Hand, &OpenMentsu, &NukedTiles, &Jikaze,
        Option<&RonOption>, Option<&PonOption>, Option<&ChiOption>, Option<&DaiminkanOption>,
    ), (
        Without<HumanPlayer>, 
        Without<RonDeclared>, // lock in decisions
        Without<PonDeclared>, 
        Without<ChiDeclared>, 
        Without<DaiminkanDeclared>
    )>,
    game: Res<GameState>,
    wall: Res<Wall>,
    revolver: Res<Revolver>,
    mut commands: Commands,
) {
    for (player, profile, hand, open_mentsu, nuked_tiles, jikaze, ron, pon, chi, kan) in query.iter() {
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

        let target_yaku = determine_bot_strategy(
            &hand.0, revolver.chamber, profile,
            wall.remaining_draws(), calculate_shanten(&combine_tiles(&hand.0, &open_mentsu.0)),
            &wall.get_dora_indicators(),
        );

        // closed yaku early return
        if matches!(target_yaku, TargetYaku::Pinfu | TargetYaku::Kokushi | TargetYaku::Suuankou | TargetYaku::Sanankou | TargetYaku::Chiitoitsu | TargetYaku::ChuurenPoutou(_)) {
            commands.entity(player)
                .remove::<PonOption>()
                .remove::<ChiOption>()
                .remove::<DaiminkanOption>();
            continue;
        } else if matches!(target_yaku, TargetYaku::Chinitsu(_) | TargetYaku::Honitsu(_) | TargetYaku::Tanyao | TargetYaku::Ryuuiisou | TargetYaku::Tsuuiisou) {
            let called_tile = pon.map(|p| p.0)
                .or_else(|| chi.map(|c| c.tile))
                .or_else(|| kan.map(|k| k.0))
                .unwrap();
            let mut should_refuse = false;

            match target_yaku {
                TargetYaku::Chinitsu(suit) => {
                    should_refuse = match called_tile {
                        Tile::Man(_) => suit != Suit::Man,
                        Tile::Pin(_) => suit != Suit::Pin,
                        Tile::Sou(_) => suit != Suit::Sou,
                        Tile::Honor(_) => true,
                    };
                }
                TargetYaku::Honitsu(suit) => {
                    should_refuse = match called_tile {
                        Tile::Man(_) => suit != Suit::Man,
                        Tile::Pin(_) => suit != Suit::Pin,
                        Tile::Sou(_) => suit != Suit::Sou,
                        Tile::Honor(_) => false,
                    };
                }
                TargetYaku::Tanyao => {
                    if is_yaochuuhai(&called_tile) {
                        should_refuse = true;
                    }
                }
                TargetYaku::Ryuuiisou => {
                    if !is_green(&called_tile) {
                        should_refuse = true;
                    }
                }
                TargetYaku::Tsuuiisou => {
                    if !is_honor(&called_tile) {
                        should_refuse = true;
                    }
                }
                _ => {}
            }

            if should_refuse {
                commands.entity(player)
                    .remove::<PonOption>()
                    .remove::<ChiOption>()
                    .remove::<DaiminkanOption>();
                continue;
            }
        }

        let pre_shanten = calculate_shanten(&combine_tiles(&hand.0, &open_mentsu.0));
        let is_closed = open_mentsu.0.is_empty();

        let mut is_critical_call = false;
        if let Some(called_tile) = pon.map(|p| p.0).or_else(|| kan.map(|k| k.0)).or_else(|| chi.map(|c| c.tile)) {
            match target_yaku {
                TargetYaku::Daisangen => {
                    if matches!(called_tile, Tile::Honor(Honor::White | Honor::Green | Honor::Red)) {
                        is_critical_call = true;
                    }
                }
                TargetYaku::Tsuuiisou => {
                    if is_honor(&called_tile) {
                        is_critical_call = true;
                    }
                }
                TargetYaku::Ryuuiisou => {
                    if is_green(&called_tile) {
                        is_critical_call = true;
                    }
                }
                _ => {}
            }
        }

        // scores a simulated post-call hand (now returns a 4-tuple to isolate dora)
        let score = |hand_tiles: &[Tile], open: &[Mentsu]| -> (i32, i32, i32, i32) {
            let combined = combine_tiles(hand_tiles, open);
            let mut freq = tiles_to_frequency_array(&combined);
            let shanten = calculate_shanten_from_array(&mut freq);
            let ukeire = ukeire_tiles(&mut freq, shanten).len() as i32;
            let han = estimate_yaku_han(&combined, &jikaze.0, &game.bakaze) as i32;
            let dora = count_dora(&combined, &*wall, false, &nuked_tiles.0).dora as i32;
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

        if is_critical_call 
        || (!ruins_closed_tenpai 
            && !is_cheap_and_risky 
            && pre_shanten >= post_shanten 
            && best_han > 0 
            && (rng.random::<f32>() < shanten_chance || rng.random::<f32>() < dora_chance)) {
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


// TODO nukidora doesn't work on bots
pub fn bot_main_phase_system(
    query: Query<(
        Entity,
        Option<&TsumoOption>,
        Option<&RiichiOption>,
        Option<&AnkanOption>,
        Option<&ShouminkanOption>,
        Option<&KyuushuOption>,
        Option<&NukidoraOption>,
        Option<&NukedTiles>,
        Option<&Hand>,
        Option<&DrawnTile>,
        Option<&BotProfile>,
        Option<&Jikaze>,
    ), Without<HumanPlayer>>,
    visible_query: Query<(Entity, Has<Riichi>)>,
    revolver: Res<Revolver>,
    game: Res<GameState>,
    wall: Res<Wall>,
    mut tsumo_writer: MessageWriter<DeclareTsumoMessage>,
    mut kan_writer: MessageWriter<DeclareKanMessage>,
    mut kyuushu_writer: MessageWriter<DeclareKyuushuMessage>,
    mut nuki_writer: MessageWriter<DeclareNukidoraMessage>,
    mut commands: Commands,
) {
    for (
        player, tsumo, riichi,
        ankan, shouminkan, kyuushu, nuki, nuked_tiles_opt,
        hand_opt, drawn_opt, profile_opt, jikaze_opt
    ) in &query {

        if tsumo.is_none() && riichi.is_none() && ankan.is_none() && shouminkan.is_none() && kyuushu.is_none() && nuki.is_none() {
            continue;
        }

        if let Some(n) = nuki {
            nuki_writer.write(DeclareNukidoraMessage { player, tile: n.0[0] });
            break;
        }

        if let Some(t) = tsumo {
            tsumo_writer.write(DeclareTsumoMessage { player, result: t.result.to_owned() });
            break;
        }

        if let Some(r) = riichi {
            let mut should_riichi = true;

            if let (Some(hand), Some(drawn), Some(profile), Some(jikaze), Some(nuked_tiles)) 
            = (hand_opt, drawn_opt, profile_opt, jikaze_opt, nuked_tiles_opt) {
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
                let death_risk = 1.0 / (7.0 - revolver.chamber as f32);
                let effective_aggressiveness = ((profile.aggressiveness * profile.composure).max(profile.aggressiveness - 0.2)).max(0.2);

                // estimate current han value
                let estimated_yaku_han = estimate_yaku_han(&full_hand, &jikaze.0, &game.bakaze);
                let dora_count = count_dora(&full_hand, &*wall, false, &nuked_tiles.0).dora;
                let total_estimated_han = estimated_yaku_han + dora_count;

                if death_risk > effective_aggressiveness + 0.1 { // base addition
                    should_riichi = false;
                } else if total_estimated_han >= 5 {
                    // discourage mangan hand from riichi
                    if effective_aggressiveness < 0.5 {
                        should_riichi = false;
                    }
                } else {
                    // cheap hand ( <4 han)
                    if max_wait_types <= 1 {
                        if someone_riichi && effective_aggressiveness <= 0.4 {
                            should_riichi = false;
                        } else if effective_aggressiveness < 0.2 {
                            should_riichi = false; // tegawari
                        }
                    }
                }
            }

            if should_riichi {
                commands.entity(player).insert(RiichiSelecting);
                commands.entity(player).remove::<AnkanOption>();
                commands.entity(player).remove::<ShouminkanOption>();
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
    mut query: Query<(Entity, &mut Hand, &OpenMentsu, &NukedTiles, Option<&mut DrawnTile>, &BotCheatIntent, Has<Riichi>), Without<HumanPlayer>>,
    mut kawa_query: Query<(Entity, &mut Kawa)>,
    open_query: Query<(Entity, &OpenMentsu)>,
    wall: Res<Wall>,
    timer: Res<BlackoutTimer>,
    mut cheat_log: ResMut<CheatLog>,
    mut replay_log: Option<ResMut<ReplayLog>>,
    mut commands: Commands,
) {
    let elapsed = timer.0.elapsed_secs();
    let mut rng = rand::rng();

    for (bot_entity, mut hand, open_mentsu, nuked_tiles, mut maybe_drawn, intent, is_riichi) in query.iter_mut() {
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
        for tile in &wall.get_dora_indicators() {
            visible_tiles[tile_to_index(tile)] += 1;
        }

        let combined_current = combine_tiles(&full_hand, &open_mentsu.0);
        let mut current_freq = tiles_to_frequency_array(&combined_current);
        let current_shanten = calculate_shanten_from_array(&mut current_freq);
        let current_ukeire: i32 = ukeire_tiles(&mut current_freq, current_shanten).iter()
            .map(|&j| (4 - current_freq[j] as i32 - visible_tiles[j]).max(0))
            .sum();
        let current_dora = count_dora(&combined_current, &*wall, is_riichi, &nuked_tiles.0).dora as i32;

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
                    let dora = count_dora(&combined_temp, &*wall, is_riichi, &nuked_tiles.0).dora as i32;

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

                if let Some(ref mut log) = replay_log {
                    log.events.push(ReplayEvent::Cheat {
                        cheater: bot_entity,
                        target_kawa: target_entity,
                        tile_taken: stolen_tile,
                        tile_left: bot_tile,
                    });
                }

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
