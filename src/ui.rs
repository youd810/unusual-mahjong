use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::components::*;
use crate::messages::*;
use crate::resources::*;
use crate::core::*;


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
                    // TODO: just auto-picking the first valid chi position for now. more choice(s) later
                    if let Some(c) = chi 
                        && ui.button(format!("Chi {:?}", c.tile)).clicked() 
                        && let Ok(discarded_by) = discard_query.single() {
                            chi_writer.write(DeclareChiMessage {
                                player,
                                tile: c.tile,
                                pos: c.positions[0],
                                discarded_by: discarded_by.0,
                            });
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