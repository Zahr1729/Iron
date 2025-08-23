use std::sync::{Arc, mpsc::Sender};

use eframe::egui::{self, Widget};

use crate::{audio::dag::EffectDAG, player::AudioCommand};

pub struct DAGWidget<'a> {
    dag: &'a Arc<EffectDAG>,
    allow_zoom: egui::Vec2b,
    allow_drag: egui::Vec2b,
    allow_scroll: egui::Vec2b,
    tx_commands: Sender<AudioCommand>,
}

impl<'a> DAGWidget<'a> {
    pub fn new(dag: &'a Arc<EffectDAG>, tx_commands: Sender<AudioCommand>) -> Self {
        Self {
            dag,
            allow_zoom: [true, false].into(),
            allow_drag: [true, false].into(),
            allow_scroll: [true, false].into(),
            tx_commands,
        }
    }
}

impl Widget for DAGWidget<'_> {
    fn ui(self, _ui: &mut egui::Ui) -> egui::Response {
        // Iterate over each element and place it in the ui

        todo!();
    }
}
