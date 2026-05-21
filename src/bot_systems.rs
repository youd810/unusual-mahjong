use bevy::prelude::*;

use crate::core::*;
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

        let mut hand_plus_drawn = hand.0.clone();
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