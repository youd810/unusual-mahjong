// ! temporary ui for testing only

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::components::*;
use crate::messages::*;
use crate::resources::*;
use crate::core::*;
use crate::scoring::*;
use crate::states::*;

pub fn human_discard_ui_system(
    mut contexts: EguiContexts,
    current_turn: Res<CurrentTurn>,
    query: Query<(Entity, &Hand, Option<&DrawnTile>, Option<&RiichiOption>, Has<RiichiSelecting>, Option<&ForbiddenDiscard>), With<HumanPlayer>>,
    mut discard_writer: MessageWriter<DiscardTileMessage>,
    mut riichi_writer: MessageWriter<DeclareRiichiMessage>,
    mut commands: Commands,
) {
    // TODO: show human hand at all times
    let Ok((player, hand, maybe_drawn, maybe_riichi, is_selecting, maybe_forbidden)) = query.get(current_turn.0) else {
        return;
    };
    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::Window::new("Your Hand")
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -50.0))
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .show(ctx, |ui| {
            if is_selecting {
                ui.label("Select a tile to discard for Riichi:");
            }
            ui.horizontal(|ui| {
                let valid_discards = maybe_riichi.map(|r| &r.0);

                for (i, tile) in hand.0.iter().enumerate() {
                    ui.push_id(i, |ui| { // each button has a unique ID
                        let tile_name = format!("{:?}", tile);
                        let enabled = (!is_selecting || valid_discards.is_some_and(|v| v.contains(tile))) 
                            && !maybe_forbidden.is_some_and(|f| f.0.contains(tile));

                        if ui.add_enabled(enabled, egui::Button::new(tile_name)).clicked() {
                            if is_selecting {
                                riichi_writer.write(DeclareRiichiMessage { player, tile: *tile });
                                commands.entity(player).remove::<RiichiSelecting>();
                            } else {
                                discard_writer.write(DiscardTileMessage { player, tile: *tile, is_tsumogiri: false });
                            }
                        }
                    });
                }

                // separate drawn tile & riichi
                if let Some(drawn) = maybe_drawn {
                    ui.separator();
                    let drawn_name = format!("{:?}", drawn.0);
                    let enabled = (!is_selecting || valid_discards.is_some_and(|v| v.contains(&drawn.0))) 
                        && !maybe_forbidden.is_some_and(|f| f.0.contains(&drawn.0));

                    if ui.add_enabled(enabled, egui::Button::new(drawn_name)).clicked() {
                        if is_selecting {
                            riichi_writer.write(DeclareRiichiMessage { player, tile: drawn.0 });
                            commands.entity(player).remove::<RiichiSelecting>();
                        } else {
                            discard_writer.write(DiscardTileMessage { player, tile: drawn.0, is_tsumogiri: true });
                        }
                    }
                }
            });

            if is_selecting && ui.button("Cancel").clicked() {
                commands.entity(player).remove::<RiichiSelecting>();
            }
        });
}


pub fn call_window_ui_system(
    mut contexts: EguiContexts,
    query: Query<(
        Entity,
        Option<&RonOption>,
        Option<&PonOption>,
        Option<&ChiOption>,
        Option<&DaiminkanOption>,
    ), (With<HumanPlayer>, Without<RonDeclared>)>,
    mut pon_writer: MessageWriter<DeclarePonMessage>,
    mut chi_writer: MessageWriter<DeclareChiMessage>,
    mut kan_writer: MessageWriter<DeclareKanMessage>,
    discard_query: Query<&DiscardedBy, With<CurrentDiscard>>,
    mut commands: Commands,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return; };

    for (player, ron, pon, chi, kan) in &query {

        if ron.is_none() && pon.is_none() && chi.is_none() && kan.is_none() {
            continue;
        }

        egui::Window::new("Call Action")
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-50.0, -50.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ron.is_some() && ui.button("Ron").clicked() {
                        commands.entity(player).insert(RonDeclared);
                    }
                    if let Some(p) = pon && ui.button(format!("Pon {:?}", p.0)).clicked() {
                        pon_writer.write(DeclarePonMessage { player, tile: p.0 });  
                    }
                    
                    if let Some(c) = chi && let Ok(discarded_by) = discard_query.single() {
                        for pos in &c.positions {
                            let label = match pos {
                                ChiTilePos::Left => {
                                    let n = next_tile_sequence(&c.tile).unwrap();
                                    let nn = next_tile_sequence(&n).unwrap();
                                    format!("Chi [{:?} {:?} {:?}]", c.tile, n, nn)
                                }
                                ChiTilePos::Middle => {
                                    let p = previous_tile_sequence(&c.tile).unwrap();
                                    let n = next_tile_sequence(&c.tile).unwrap();
                                    format!("Chi [{:?} {:?} {:?}]", p, c.tile, n)
                                }
                                ChiTilePos::Right => {
                                    let p = previous_tile_sequence(&c.tile).unwrap();
                                    let pp = previous_tile_sequence(&p).unwrap();
                                    format!("Chi [{:?} {:?} {:?}]", pp, p, c.tile)
                                }
                            };

                            if ui.button(&label).clicked() {
                                chi_writer.write(DeclareChiMessage {
                                    player,
                                    tile: c.tile,
                                    pos: *pos,
                                    discarded_by: discarded_by.0,
                                });
                            }
                        }
                    }

                    if let Some(k) = kan && ui.button(format!("Kan {:?}", k.0)).clicked() {
                        kan_writer.write(DeclareKanMessage { player, tile: k.0, is_discard: true });
                    }

                    if ui.button("Skip").clicked() {
                        commands.entity(player)
                            .remove::<RonOption>()
                            .remove::<PonOption>()
                            .remove::<ChiOption>()
                            .remove::<DaiminkanOption>();
                    }
                    
                });
            });
    }
}

pub fn main_phase_ui_system(
    mut contexts: EguiContexts,
    query: Query<(
        Entity,
        Option<&TsumoOption>,
        Option<&RiichiOption>,
        Option<&AnkanOption>,
        Option<&ShouminkanOption>,
        Option<&KyuushuOption>,
    ), With<HumanPlayer>>,
    mut tsumo_writer: MessageWriter<DeclareTsumoMessage>,
    mut kan_writer: MessageWriter<DeclareKanMessage>,
    mut kyuushu_writer: MessageWriter<DeclareKyuushuMessage>,
    mut commands: Commands,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return; };

    for (player, tsumo, riichi, ankan, shouminkan, kyuushu) in &query {

        if tsumo.is_none() && riichi.is_none() && ankan.is_none() && shouminkan.is_none() && kyuushu.is_none() {
            continue;
        }

        egui::Window::new("Declare")
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-50.0, -50.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(t) = tsumo && ui.button("Tsumo").clicked() {
                        tsumo_writer.write(DeclareTsumoMessage { player, result: t.result.to_owned() });
                    }
                    if riichi.is_some() && ui.button("Riichi").clicked() {
                        commands.entity(player).insert(RiichiSelecting);
                    }
                    if let Some(a) = ankan {
                        for tile in &a.0 {
                            if ui.button(format!("Ankan {:?}", tile)).clicked() {
                                kan_writer.write(DeclareKanMessage { player, tile: *tile, is_discard: false });
                            }
                        }
                    }
                    if let Some(s) = shouminkan {
                        for tile in &s.0 {
                            if ui.button(format!("Shouminkan {:?}", tile)).clicked() {
                                kan_writer.write(DeclareKanMessage { player, tile: *tile, is_discard: false });
                            }
                        }
                    }
                    if kyuushu.is_some() && ui.button("Kyuushu Kyuuhai").clicked() {
                        kyuushu_writer.write(DeclareKyuushuMessage { player });
                    }

                    if ui.button("Skip").clicked() {
                        commands.entity(player)
                            .remove::<TsumoOption>()
                            .remove::<RiichiOption>()
                            .remove::<AnkanOption>()
                            .remove::<ShouminkanOption>()
                            .remove::<KyuushuOption>();
                    }
                });
            });
    }
}


pub fn info_display_ui_system(
    mut contexts: EguiContexts,
    game: Res<GameState>,
    wall: Res<Wall>,
    dead_wall: Res<DeadWall>,
    current_turn: Res<CurrentTurn>,
    player_query: Query<(
        Entity,
        &Points,
        &Jikaze,
        &Hand,
        &Kawa,
        &OpenMentsu,
        Has<HumanPlayer>,
        Has<Riichi>,
        Has<Oya>,
        Has<Alive>,
    ), With<PlayerTag>>,
    omniscience: Res<Omniscience>
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    // round info
    egui::Window::new("Round Info")
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 10.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(format!("{:?} {} | Honba: {} | Wall: {}",
                game.bakaze, game.rounds, game.honba, wall.0.len()));
            ui.label(format!("Riichi pool: {}", game.riichi_points));
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Dora:");
                for indicator in &dead_wall.dora_indicators {
                    let dora = crate::scoring::get_dora_from_indicator(indicator);
                    ui.label(format!("{:?}", dora));
                }
            });
            if omniscience.0 {
                ui.label("next draws:");
                ui.horizontal_wrapped(|ui| {
                    for tile in wall.0.iter().take(5) {
                        ui.label(format!("{:?}", tile));
                    }
                });
            }
        });

    // players
    let mut players: Vec<_> = player_query.iter().collect();
    players.sort_by_key(|(_, _, jikaze, _, _, _, _, _, _, _)| jikaze.0.to_num());

    egui::Window::new("Players")
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 160.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            for (player, points, jikaze, hand, kawa, open, is_human, is_riichi, is_oya, is_alive) in &players {
                let is_current = *player == current_turn.0;

                let mut label = String::new();
                label += &format!("{:?}", player);
                if is_current { label += "('s turn)"; }
                label += &format!(" {:?}", jikaze.0);
                if *is_oya { label += " (Oya)"; }
                if *is_human { label += " (You)"; }
                if *is_riichi { label += " (In Riichi)"; }
                if !*is_alive { label += " *ded*"; }
                label += &format!(" — {}pts", points.0);

                egui::CollapsingHeader::new(label)
                    .default_open(true)
                    .show(ui, |ui| {
                        // reveal all hands
                        if *is_human || omniscience.0 {
                            ui.label("hand:");
                            ui.horizontal_wrapped(|ui| {
                                for tile in &hand.0 {
                                    ui.label(format!("{:?}", tile));
                                }
                            });
                        }
                        // open mentsu
                        if !open.0.is_empty() {
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Naki:");
                                for mentsu in &open.0 {
                                    let text = match mentsu {
                                        Mentsu::Koutsu(t, _) => format!("Pon{:?}", t),
                                        Mentsu::Shuntsu(t, _) => format!("Chi{:?}", t),
                                        Mentsu::Ankan(t) => format!("Ankan{:?}", &t[..1]),
                                        Mentsu::Daiminkan(t) => format!("Dkan{:?}", &t[..1]),
                                        Mentsu::Shouminkan(t) => format!("Skan{:?}", &t[..1]),
                                        Mentsu::Jantou(_) => String::new(),
                                    };
                                    if !text.is_empty() { ui.label(text); }
                                }
                            });
                        }

                        // kawa
                        if !kawa.0.is_empty() {
                            ui.label("Kawa:");
                            for chunk in kawa.0.chunks(6) {
                                ui.horizontal(|ui| {
                                    for tile in chunk {
                                        ui.label(format!("{:?}", tile));
                                    }
                                });
                            }
                        }
                    });
            }
        });
}


pub fn round_end_ui_system(
    mut contexts: EguiContexts,
    summary: Option<Res<RoundSummary>>,
    jikaze_query: Query<&Jikaze>,
    mut commands: Commands,
) {
    let Some(summary) = summary else { return };
    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::Window::new("Round Result")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading(&summary.reason_text);
            ui.separator();

            if summary.winners.is_empty() {
                ui.label("No winners.");
            }

            for (entity, result, hand_score) in &summary.winners {
                let wind = jikaze_query.get(*entity)
                    .map(|j| format!("{:?}", j.0))
                    .unwrap_or("?".to_string());

                ui.label(format!("Winner: {} ({}) — {}",
                    entity, wind,
                    if summary.is_tsumo { "Tsumo" } else { "Ron" }));

               if result.is_yakuman {
                    let count = match result.yaku_names.len() {
                        2 => "DOUBLE ",
                        3 => "TRIPLE ",
                        4 => "QUADRUPLE ",
                        5 => "QUINTUPLE ",
                        6 => "SEXTUPLE ",
                        _ => "",
                    };
                    ui.label(format!("{}YAKUMAN!!!", count));
                } else if result.total_han >= 13 {
                    ui.label("KAZOE YAKUMAN!!!");
                }

                for yaku in &result.yaku_names {
                    ui.label(format!("  • {}", yaku));
                }

                if result.dora_count > 0 {
                    ui.label(format!("Dora {}", result.dora_count));
                }

                if result.ura_dora_count > 0 {
                    ui.label(format!("Ura Dora {}", result.ura_dora_count));
                }

                ui.label(format!("  {}han {}fu", result.total_han, result.total_fu));
                ui.label(format!("{}", hand_score));
                ui.separator();
            }

            if ui.button("Continue").clicked() {
                commands.remove_resource::<RoundSummary>();
            }
        });
}

pub fn target_selection_ui_system(
    mut contexts: EguiContexts,
    query: Query<(Entity, &Jikaze, &Points), With<Alive>>,
    mut pending: ResMut<PendingTargetSelection>,
    mut queue: ResMut<ExecuteQueue>,
    mut next_step: ResMut<NextState<ExecutionSubState>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::Window::new("Execute")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.label(format!("{} shots remaining", pending.remaining_picks));
            for (player, jikaze, points) in query.iter() {
                if player != pending.shooter 
                && ui.button(format!("{} ({:?} seat, {} points)", player, jikaze.0, points.0)).clicked() {
                        queue.0.push(Execute {
                            shooter: pending.shooter,
                            target: player,
                        });
                        pending.remaining_picks -= 1;
                        if pending.remaining_picks == 0 {
                            next_step.set(ExecutionSubState::Processing);
                    }
                }
            }
        });
}


pub fn blackout_ui_system(
    mut contexts: EguiContexts,
    mut query: Query<(Entity, &mut Hand, &Jikaze, Option<&mut DrawnTile>), With<HumanPlayer>>,
    mut kawa_query: Query<(Entity, &mut Kawa, &Jikaze)>,
    mut selection: ResMut<BlackoutTileSelection>,
    mut cheat_log: ResMut<CheatLog>,
    timer: Res<BlackoutTimer>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let Ok((player, mut hand, _, mut maybe_drawn)) = query.single_mut() else { return };

    egui::Window::new("The room is pitch black!")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(format!("Time remaining: {:.1}s", timer.0.remaining_secs()));
            ui.separator();

            ui.label("Your hand (click to select):");
            ui.horizontal_wrapped(|ui| {
                for (i, tile) in hand.0.iter().enumerate() {
                    let selected = matches!(&selection.selected, SelectedSource::Hand(idx, _) if *idx == i);
                    if ui.selectable_label(selected, format!("{:?}", tile)).clicked() {
                        selection.selected = SelectedSource::Hand(i, *tile);
                    }
                }

                if let Some(drawn) = &maybe_drawn {
                    ui.separator();
                    let selected = matches!(&selection.selected, SelectedSource::Drawn(_));
                    if ui.selectable_label(selected, format!("{:?}", drawn.0)).clicked() {
                        selection.selected = SelectedSource::Drawn(drawn.0);
                    }
                }
            });

            ui.separator();

            let selected_tile = match &selection.selected {
                SelectedSource::Hand(_, tile) => Some(*tile),
                SelectedSource::Drawn(tile) => Some(*tile),
                SelectedSource::None => None,
            };

            if let Some(hand_tile) = selected_tile {
                ui.label(format!("Selected: {:?} — now pick a kawa tile to swap with:", hand_tile));

                for (kawa_owner, mut kawa, jikaze) in kawa_query.iter_mut() {
                    if kawa.0.is_empty() { // || player == kawa_owner <- leave this for now
                        continue;
                    }

                    ui.label(format!("{:?} seat kawa:", jikaze.0));
                    ui.horizontal_wrapped(|ui| {
                        let mut swap_target: Option<(usize, Tile)> = None;
                        for (k, kawa_tile) in kawa.0.iter().enumerate() {
                            if ui.button(format!("{:?}", kawa_tile)).clicked() && swap_target.is_none() {
                                swap_target = Some((k, *kawa_tile));
                            }
                        }

                        if let Some((k, taken)) = swap_target {
                            kawa.0[k] = hand_tile;

                            match &selection.selected {
                                SelectedSource::Hand(idx, _) => {
                                    hand.0[*idx] = taken;
                                    hand.0.sort();
                                }
                                SelectedSource::Drawn(_) => {
                                    if let Some(ref mut drawn) = maybe_drawn {
                                        drawn.0 = taken;
                                    }
                                }
                                _ => {}
                            }

                            cheat_log.0.push(CheatEntry {
                                cheater: player,
                                target_kawa: kawa_owner,
                                tile_taken: taken,
                                tile_left: hand_tile,
                            });

                            selection.selected = SelectedSource::None;

                            println!("Cheat: swapped {:?} for {:?} from {:?}'s kawa",
                                hand_tile, taken, kawa_owner);
                        }
                    });
                }
            } else {
                ui.label("Select a tile from your hand first.");
            }
        });
}


pub fn accusation_ui_system(
    mut contexts: EguiContexts,
    timer: Res<AccusationTimer>,
    human_query: Query<Entity, With<HumanPlayer>>,
    suspects: Query<(Entity, &Jikaze, &Points), (With<Alive>, Without<HumanPlayer>)>,
    mut accuse_writer: MessageWriter<AccuseCheatMessage>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let Ok(human) = human_query.single() else { return };

    egui::Window::new("Was someone cheating?")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(format!("Time remaining: {:.1}s", timer.0.remaining_secs()));
            ui.separator();

            for (entity, jikaze, points) in suspects.iter() {
                if ui.button(format!("Accuse {} ({:?}, {}pts)",
                    entity, jikaze.0, points.0)).clicked()
                {
                    accuse_writer.write(AccuseCheatMessage {
                        accuser: human,
                        suspect: entity,
                    });
                }
            }

            ui.separator();
            ui.label("Or wait for the timer to expire.");
        });
}


fn apply_debug_hand(
    player: Entity,
    mut tiles: Vec<Tile>,
    draw: Tile,
    open: Vec<Mentsu>,
    commands: &mut Commands,
    wall: &mut Wall,
    current_turn: &mut CurrentTurn,
    next_state: &mut ResMut<NextState<TurnState>>,
) {
    tiles.sort();
    let tenpai_waits = check_tenpai(&tiles);

    commands.entity(player).insert(Hand(tiles));
    commands.entity(player).insert(OpenMentsu(open.clone()));
    commands.entity(player).insert(Tenpai(tenpai_waits));
    commands.entity(player).remove::<DrawnTile>();

    if open.is_empty() {
        commands.entity(player).insert(ClosedHand);
    } else {
        commands.entity(player).remove::<ClosedHand>();
    }

    wall.0.insert(0, draw);
    current_turn.0 = player;
    next_state.set(TurnState::Draw);
}


// !for testing 
pub fn debug_ui_system(
    mut contexts: EguiContexts,
    mut next_state: ResMut<NextState<TurnState>>,
    state: Res<State<TurnState>>,            
    kawa_query: Query<(Entity, &Kawa)>,      
    mut query: Query<(Entity, &mut Hand, Option<&mut DrawnTile>, Has<HumanPlayer>)>,
    mut wall: ResMut<Wall>,
    mut current_turn: ResMut<CurrentTurn>,
    mut omniscience: ResMut<Omniscience>,
    mut commands: Commands,
    mut revolver: ResMut<Revolver>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::Window::new("Debug Tools")
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(10.0, -10.0))
        .show(ctx, |ui| {
            if ui.button("Force Restart Match").clicked() {
                next_state.set(TurnState::Setup);
            }

            ui.separator();
            ui.checkbox(&mut omniscience.0, "Reveal Hands");

            ui.separator();
            if ui.button("Force Blackout").clicked() {
                let all_kawa = kawa_query.iter()
                    .map(|(e, kawa)| (e, kawa.0.clone()))
                    .collect();

                commands.insert_resource(KawaSnapshot { all_kawa });
                commands.insert_resource(PreBlackoutState(state.get().clone()));
                commands.insert_resource(CheatLog::default());
                commands.insert_resource(BlackoutTimer(Timer::from_seconds(5.0, TimerMode::Once)));

                next_state.set(TurnState::Blackout);
                println!("Debug: Forced Blackout.");
            }

            ui.separator();
            ui.label("Yaku Builder");

            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                let Some((player, _, _, _)) = query.iter_mut().find(|(_, _, _, is_human)| *is_human) else { return };

                // waiting on sou 3 
                if ui.button("Tenpai (No Yaku)").clicked() {
                    let hand = vec![
                        Tile::Man(1), Tile::Man(2), Tile::Man(3),
                        Tile::Pin(4), Tile::Pin(5), Tile::Pin(6),
                        Tile::Sou(1), Tile::Sou(2),
                        Tile::Honor(Honor::East), Tile::Honor(Honor::East),
                        Tile::Honor(Honor::White), Tile::Honor(Honor::White), Tile::Honor(Honor::White),
                    ];
                    apply_debug_hand(player, hand, Tile::Sou(3), vec![], &mut commands, &mut wall, &mut current_turn, &mut next_state);
                }

                // waiting on man 5
                if ui.button("1 Han (Tanyao)").clicked() {
                    let hand = vec![
                        Tile::Man(2), Tile::Man(3), Tile::Man(4),
                        Tile::Pin(2), Tile::Pin(3), Tile::Pin(4),
                        Tile::Sou(6), Tile::Sou(7), Tile::Sou(8),
                        Tile::Sou(3), Tile::Sou(3),
                        Tile::Man(6), Tile::Man(7), 
                    ];
                    apply_debug_hand(player, hand, Tile::Man(5), vec![], &mut commands, &mut wall, &mut current_turn, &mut next_state);
                }

                if ui.button("Ryankantsu").clicked() {
                    let hand = vec![
                        Tile::Man(2), Tile::Man(3), Tile::Man(4),
                        Tile::Sou(3), Tile::Sou(3),
                        Tile::Pin(6), Tile::Pin(7),
                    ];
                    let open = vec![
                        Mentsu::Ankan([Tile::Honor(Honor::White); 4]),
                        Mentsu::Ankan([Tile::Honor(Honor::Green); 4]),
                    ];
                    apply_debug_hand(player, hand, Tile::Pin(5), open, &mut commands, &mut wall, &mut current_turn, &mut next_state);
                }

                // mangan (5 han): honitsu(3) + yakuhai green(1) + tsumo(1)
                if ui.button("Mangan (5 han)").clicked() {
                    let hand = vec![
                        Tile::Pin(1), Tile::Pin(2), Tile::Pin(3),
                        Tile::Pin(4), Tile::Pin(5), Tile::Pin(6),
                        Tile::Pin(5), Tile::Pin(6),
                        Tile::Honor(Honor::Green), Tile::Honor(Honor::Green), Tile::Honor(Honor::Green),
                        Tile::Pin(2), Tile::Pin(2),
                    ];
                    apply_debug_hand(player, hand, Tile::Pin(7), vec![], &mut commands, &mut wall, &mut current_turn, &mut next_state);
                }

                // haneman (6 han): honitsu(3) + yakuhai green(1) + yakuhai red(1) + tsumo(1)
                if ui.button("Haneman (6 han)").clicked() {
                    let hand = vec![
                        Tile::Pin(1), Tile::Pin(2), Tile::Pin(3),
                        Tile::Pin(5), Tile::Pin(6),
                        Tile::Honor(Honor::Green), Tile::Honor(Honor::Green), Tile::Honor(Honor::Green),
                        Tile::Honor(Honor::Red), Tile::Honor(Honor::Red), Tile::Honor(Honor::Red),
                        Tile::Pin(8), Tile::Pin(8),
                    ];
                    apply_debug_hand(player, hand, Tile::Pin(7), vec![], &mut commands, &mut wall, &mut current_turn, &mut next_state);
                }

                // baiman (8 han): honitsu(3) + yakuhai green(1) + yakuhai red(1) + shousangen(2) + tsumo(1)
                if ui.button("Baiman (8 han)").clicked() {
                    let hand = vec![
                        Tile::Pin(1), Tile::Pin(2),
                        Tile::Pin(5), Tile::Pin(6), Tile::Pin(7),
                        Tile::Honor(Honor::Green), Tile::Honor(Honor::Green), Tile::Honor(Honor::Green),
                        Tile::Honor(Honor::Red), Tile::Honor(Honor::Red), Tile::Honor(Honor::Red),
                        Tile::Honor(Honor::White), Tile::Honor(Honor::White),
                    ];
                    apply_debug_hand(player, hand, Tile::Pin(3), vec![], &mut commands, &mut wall, &mut current_turn, &mut next_state);
                }

                // kazoe yakuman (13 han): chinitsu(6) + ryanpeikou(3) + tanyao(1) + pinfu(1) + riichi(1) + tsumo(1)
                if ui.button("Kazoe Yakuman (13 han)").clicked() {
                    let hand = vec![
                        Tile::Pin(2), Tile::Pin(2), Tile::Pin(3), Tile::Pin(3),
                        Tile::Pin(4), Tile::Pin(4), Tile::Pin(5), Tile::Pin(5),
                        Tile::Pin(6), Tile::Pin(6), Tile::Pin(7),
                        Tile::Pin(8), Tile::Pin(8),
                    ];
                    commands.entity(player).insert(Riichi { turns_since: 0 });
                    apply_debug_hand(player, hand, Tile::Pin(7), vec![], &mut commands, &mut wall, &mut current_turn, &mut next_state);
                }

                // chiitoitsu (tests chiitoitsu evaluation path)
                if ui.button("Chiitoitsu").clicked() {
                    let hand = vec![
                        Tile::Pin(2), Tile::Pin(2), Tile::Pin(4), Tile::Pin(4),
                        Tile::Pin(6), Tile::Pin(6), Tile::Sou(3), Tile::Sou(3),
                        Tile::Sou(5), Tile::Sou(5), Tile::Man(8), Tile::Man(8),
                        Tile::Man(2),
                    ];
                    apply_debug_hand(player, hand, Tile::Man(2), vec![], &mut commands, &mut wall, &mut current_turn, &mut next_state);
                }

                // kokushi musou (yakuman, tests kokushi detection path)
                // draws non yaochuuhai to test ron
                if ui.button("Kokushi Musou").clicked() {
                    let hand = vec![
                        Tile::Man(1), Tile::Man(9), Tile::Pin(1), Tile::Pin(9),
                        Tile::Sou(1), Tile::Sou(9),
                        Tile::Honor(Honor::East), Tile::Honor(Honor::South),
                        Tile::Honor(Honor::West), Tile::Honor(Honor::North),
                        Tile::Honor(Honor::White),
                        Tile::Honor(Honor::Green), Tile::Honor(Honor::Red),
                    ];
                    apply_debug_hand(player, hand, Tile::Pin(5), vec![], &mut commands, &mut wall, &mut current_turn, &mut next_state);
                }

                // daisangen (yakuman, tests standard yakuman path)
                if ui.button("Daisangen").clicked() {
                    let hand = vec![
                        Tile::Honor(Honor::White), Tile::Honor(Honor::White), Tile::Honor(Honor::White),
                        Tile::Honor(Honor::Green), Tile::Honor(Honor::Green), Tile::Honor(Honor::Green),
                        Tile::Honor(Honor::Red), Tile::Honor(Honor::Red), Tile::Honor(Honor::Red),
                        Tile::Man(1), Tile::Man(2),
                        Tile::Honor(Honor::East), Tile::Honor(Honor::East),
                    ];
                    apply_debug_hand(player, hand, Tile::Man(3), vec![], &mut commands, &mut wall, &mut current_turn, &mut next_state);
                }

                // sankantsu
                if ui.button("Sankantsu").clicked() {
                    let hand = vec![
                        Tile::Man(1), Tile::Man(1), Tile::Man(1), Tile::Man(1),
                        Tile::Pin(1), Tile::Pin(1), Tile::Pin(1), Tile::Pin(1),
                        Tile::Sou(1), Tile::Sou(1), Tile::Sou(1), Tile::Sou(1),
                        Tile::Man(9)
                    ];
                    apply_debug_hand(player, hand, Tile::Man(9), vec![], &mut commands, &mut wall, &mut current_turn, &mut next_state);
                }
            });

            ui.separator();
            ui.label("Tochuu Ryuukyoku");

            if ui.button("Force Suufon Renda").clicked() {
                for (_, mut hand, drawn, _) in query.iter_mut() {
                    for tile in hand.0.iter_mut() {
                        *tile = Tile::Honor(Honor::West);
                    }
                    
                    if let Some(mut d) = drawn {
                        d.0 = Tile::Honor(Honor::West);
                    }
                }
                println!("Debug: Forced a West wind into all hands/draws.");
            }

            if ui.button("Force Kyuushu Kyuuhai (Human)").clicked() 
            && let Some((_, mut hand, _, _)) = query.iter_mut().find(|(_, _, _, is_human)| *is_human) {
                hand.0 = vec![
                    Tile::Man(1), Tile::Man(9),
                    Tile::Pin(1), Tile::Pin(9),
                    Tile::Sou(1), Tile::Sou(9),
                    Tile::Honor(Honor::East), Tile::Honor(Honor::South), Tile::Honor(Honor::West), Tile::Honor(Honor::North),
                    Tile::Honor(Honor::White), Tile::Honor(Honor::Green), Tile::Honor(Honor::Red),
                ];
                println!("Debug: Set Human hand to 13 orphans base (Kyuushu Kyuuhai).");
            }

            if ui.button("Force Suucha Riichi").clicked() {
                for (entity, _, _, _) in query.iter_mut() {
                    commands.entity(entity).insert(Riichi {turns_since: 0});
                }
                println!("Debug: Forced Riichi onto all players.");
            }

            ui.separator();
            ui.label("Revolver Manipulation");

            ui.add(egui::Slider::new(&mut revolver.bullet, 1..=6).text("Bullet Position"));
            ui.add(egui::Slider::new(&mut revolver.chamber, 1..=6).text("Current Chamber"));

            if revolver.chamber > revolver.bullet {
                revolver.bullet = revolver.chamber;
            }

            ui.horizontal(|ui| {
                if ui.button("Force Next Lethal").clicked() {
                    revolver.chamber = revolver.bullet;
                }
                let is_lethal = revolver.chamber == revolver.bullet;
                if ui.add_enabled(is_lethal, egui::Button::new("Force Next Safe")).clicked() {
                    if revolver.bullet == 6 {
                        revolver.bullet = 2;
                        revolver.chamber = 1;
                    } else {
                        revolver.bullet += 1;
                    }
                }
            });
        });
}


pub fn game_over_ui_system(
    mut contexts: EguiContexts,
    mut next_state: ResMut<NextState<TurnState>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::Window::new("Game Over")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("You Died");
            ui.separator();
            if ui.button("Restart Match").clicked() {
                next_state.set(TurnState::Setup);
            }
        });
}
