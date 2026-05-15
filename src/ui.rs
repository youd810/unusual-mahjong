use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::components::*;
use crate::messages::*;
use crate::resources::*;
use crate::core::*;

pub fn human_discard_ui_system(
    mut contexts: EguiContexts,
    current_turn: Res<CurrentTurn>,
    query: Query<(Entity, &Hand, Option<&DrawnTile>), With<HumanPlayer>>,
    mut messages: MessageWriter<DiscardTileMessage>,
) {
    // TODO: show human hand at all times
    let Ok((player_entity, hand, maybe_drawn)) = query.get(current_turn.0) else {
        return;
    };

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::Window::new("Your Hand")
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -50.0))
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Render tiles in hand
                for (i, tile) in hand.0.iter().enumerate() {
                    ui.push_id(i, |ui| { // each button has a unique ID
                        let tile_name = format!("{:?}", tile);
                        if ui.button(tile_name).clicked() {
                            println!("Clicked tile!");
                            messages.write(DiscardTileMessage {
                                player: player_entity,
                                tile: *tile,
                                is_tsumogiri: false,
                            });
                        }
                    });
                }

                // separate drawn tile 
                if let Some(drawn) = maybe_drawn {
                    ui.separator();
                    let drawn_name = format!("{:?}", drawn.0);
                    if ui.button(&drawn_name).clicked() {
                        println!("UI Clicked on {:?}", drawn_name);
                        messages.write(DiscardTileMessage {
                            player: player_entity,
                            tile: drawn.0,
                            is_tsumogiri: true,
                        });
                    }
                }
            });
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

    for (entity, ron, pon, chi, kan) in &query {

        if ron.is_none() && pon.is_none() && chi.is_none() && kan.is_none() {
            continue;
        }

        egui::Window::new("Call Action")
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-50.0, -50.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ron.is_some() && ui.button("Ron").clicked() {
                        commands.entity(entity).insert(RonDeclared);
                    }
                    if let Some(p) = pon && ui.button(format!("Pon {:?}", p.0)).clicked() {
                        pon_writer.write(DeclarePonMessage { player: entity, tile: p.0 });  
                    }
                    // TODO: just auto-picking the first valid chi position for now. more choice(s) later
                    if let Some(c) = chi 
                        && ui.button(format!("Chi {:?}", c.tile)).clicked() 
                        && let Ok(discarded_by) = discard_query.single() {
                            chi_writer.write(DeclareChiMessage {
                                player: entity,
                                tile: c.tile,
                                pos: c.positions[0],
                                discarded_by: discarded_by.0,
                            });
                    }
                    if let Some(k) = kan && ui.button(format!("Kan {:?}", k.0)).clicked() {
                        kan_writer.write(DeclareKanMessage { player: entity, tile: k.0, is_discard: true });
                    }

                    if ui.button("Skip").clicked() {
                        commands.entity(entity)
                            .remove::<RonOption>()
                            .remove::<PonOption>()
                            .remove::<ChiOption>()
                            .remove::<DaiminkanOption>();
                    }
                });
            });
    }
}