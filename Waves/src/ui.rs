use eframe::egui::{self, Response, Widget};
use egui_plot::GridMark;
use symphonia::core::errors::Error;

use std::{
    mem,
    ops::RangeInclusive,
    sync::{
        Arc,
        mpsc::{self, Sender},
    },
    thread::JoinHandle,
};

use crate::{
    common::{self, Channel, track::Track},
    player::AudioCommand,
};

pub mod dagwidget;
pub mod eqwidget;
pub mod graph;
pub mod playpausebutton;
pub mod progresstracker;
pub mod threadtracker;
pub mod waveformwidget;
