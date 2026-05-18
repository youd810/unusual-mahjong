use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::components::*;
use crate::messages::*;
use crate::resources::*;
use crate::core::*;
use crate::scoring;

pub fn human_discard_ui_system(
    mut contexts: EguiContexts,
    current_turn: Res<CurrentTurn>,
    query: Query<(Entity, &Hand, Option<&DrawnTile>, Option<&RiichiOption>, Has<RiichiSelecting>), With<HumanPlayer>>,
    mut discard_writer: MessageWriter<DiscardTileMessage>,
    mut riichi_writer: MessageWriter<DeclareRiichiMessage>,
    mut commands: Commands,
) {
    // TODO: show human hand at all times
    let Ok((player, hand, maybe_drawn, maybe_riichi, is_selecting)) = query.get(current_turn.0) else {
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
                        let enabled = !is_selecting || valid_discards.is_some_and(|v| v.contains(tile));

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
                    let enabled = !is_selecting || valid_discards.map_or(false, |v| v.contains(&drawn.0));

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
        &Kawa,
        &OpenMentsu,
        Has<HumanPlayer>,
        Has<Riichi>,
        Has<Oya>,
        Has<Alive>,
    ), With<PlayerTag>>,
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
        });

    // players
    let mut players: Vec<_> = player_query.iter().collect();
    players.sort_by_key(|(_, _, jikaze, _, _, _, _, _, _)| jikaze.0.to_num());

    egui::Window::new("Players")
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 120.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            for (player, points, jikaze, kawa, open, is_human, is_riichi, is_oya, is_alive) in &players {
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
                    ui.label("YAKUMAN!!!");
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
