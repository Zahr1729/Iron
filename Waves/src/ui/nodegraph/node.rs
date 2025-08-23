use std::sync::Arc;

use eframe::egui::{
    self, Grid, InnerResponse, Label, Pos2, Rect, Response, RichText, Stroke, Ui, Vec2,
};

use crate::{
    audio::effects::Effect,
    ui::nodegraph::{
        GraphAudioData, GraphStyle,
        nodecircle::{NodeCircle, NodeCircleIdentifier},
    },
};

#[derive(Clone)]
pub struct Node {
    index: usize,

    effect: Arc<dyn Effect>,

    pub input_node_circles: Vec<NodeCircle>,
    pub output_node_circles: Vec<NodeCircle>,
}

impl Node {
    pub fn new(index: usize, effect: Arc<dyn Effect>, radius: f32) -> Self {
        Self {
            index,

            input_node_circles: (0..effect.input_count())
                .into_iter()
                .map(|i| NodeCircle::new(i, true, Pos2::ZERO, radius))
                .collect::<Vec<_>>(),
            output_node_circles: (0..effect.output_count())
                .into_iter()
                .map(|i| NodeCircle::new(i, false, Pos2::ZERO, radius))
                .collect::<Vec<_>>(),

            effect,
        }
    }

    /// This function should be called immediately
    fn draw_header(&mut self, ui: &mut Ui, style: &GraphStyle) -> Response {
        let frame_pos = ui.next_widget_position();

        let header = egui::frame::Frame::new()
            .inner_margin(style.margin)
            .corner_radius(style.corner_radius)
            .show(ui, |ui| {
                let x = ui.available_width();
                let _y = ui.available_height();

                // This little function gets the position of our frame in screen space

                // uncurved
                let rect_bottom = Rect::from_two_pos(
                    frame_pos
                        + Vec2 {
                            x: 0.0,
                            y: style.header_height / 2.0,
                        },
                    frame_pos
                        + Vec2 {
                            x: x + style.margin * 2.0,
                            y: style.header_height,
                        },
                );

                // curved
                let rect_top = Rect::from_two_pos(
                    frame_pos,
                    frame_pos
                        + Vec2 {
                            x: x + style.margin * 2.0,
                            y: style.header_height,
                        },
                );

                // get the shapes
                let shape_top = egui::frame::Frame::new()
                    .corner_radius(style.corner_radius - 1.0)
                    .fill(style.header_colour)
                    .paint(rect_top);
                let shape_bottom = egui::frame::Frame::new()
                    .fill(style.header_colour)
                    .paint(rect_bottom);

                ui.painter().add(shape_top);
                ui.painter().add(shape_bottom);

                ui.add(
                    Label::new(
                        RichText::new(self.effect.name())
                            .heading()
                            .size(
                                style.header_height
                                    - 2.0 * style.margin
                                    - 2.0 * style.node_line_width,
                            )
                            .color(style.header_text_colour),
                    )
                    .selectable(false),
                );
            });
        header.response
    }

    fn draw_main(&mut self, ui: &mut Ui, style: &GraphStyle, audio_data: &GraphAudioData) {
        //println!("{:?}", ui.style().spacing);
        egui::frame::Frame::new()
            .inner_margin(style.margin)
            .show(ui, |ui| {
                Grid::new(format!("the_AWESOME_grid_ {}", self.index))
                    //.spacing(Vec2::ZERO)
                    //.with_row_color(|i, s| Some(Color32::RED))
                    .show(ui, |ui| {
                        for i in 0..self.effect.input_count().max(self.effect.output_count()) {
                            if i < self.effect.input_count() {
                                ui.add(Label::new(
                                    RichText::new("input")
                                        .size(style.main_text_size)
                                        .color(style.main_text_colour),
                                ));
                            } else {
                                ui.add(Label::new(RichText::new("")));
                            }

                            if i < self.effect.output_count() {
                                ui.add(Label::new(
                                    RichText::new("output")
                                        .size(style.main_text_size)
                                        .color(style.main_text_colour),
                                ));
                            } else {
                                ui.add(Label::new(RichText::new("")));
                            }

                            ui.end_row();
                        }
                    });

                // implement node specific data ie gain value
                self.effect.data_ui(ui, style);
            });

        egui::frame::Frame::new()
            .inner_margin(style.plot_margin)
            .show(ui, |ui| {
                // Now lets draw the effect
                self.effect.draw_plot(
                    ui,
                    audio_data.current_sample,
                    audio_data.sample_rate,
                    (style.plot_width, style.plot_height),
                );
            });
    }

    fn draw_node_without_circles(
        &mut self,
        ui: &mut Ui,
        style: &GraphStyle,
        audio_data: &GraphAudioData,
    ) {
        egui::frame::Frame::new()
            .outer_margin(style.node_circle_radius)
            .stroke(Stroke::new(style.node_line_width, style.line_colour))
            .fill(style.main_colour)
            .corner_radius(style.corner_radius)
            .show(ui, |ui| {
                // Header

                self.draw_header(ui, style);

                // Main Content

                self.draw_main(ui, style, audio_data);
            });
    }

    /// Return the appropriate (input, output) tuple if the edge is clearly (but not necessarily) valid
    /// (ie it doesnt feed into itself and input goes to output)
    /// We must do a more in depth check to make sure we have no cycles in the NodeGraph struct later.
    fn get_edge_tuple(
        &mut self,
        other_data: Option<Arc<NodeCircleIdentifier>>,
        this_data: Arc<NodeCircleIdentifier>,
    ) -> Option<(Arc<NodeCircleIdentifier>, Arc<NodeCircleIdentifier>)> {
        match other_data {
            None => None,
            Some(other_circle) => {
                // If we are referencing circles on different nodes and also their input/output choice differs
                if this_data.node_index != other_circle.node_index
                    && this_data.circle_is_input != other_circle.circle_is_input
                {
                    let this_circle = Arc::<NodeCircleIdentifier>::new(NodeCircleIdentifier {
                        node_index: this_data.node_index,
                        circle_index: this_data.circle_index,
                        circle_is_input: this_data.circle_is_input,
                    });

                    //println!("NEW LINE YAY");

                    match this_data.circle_is_input {
                        true => Some((this_circle, other_circle)),
                        false => Some((other_circle, this_circle)),
                    }
                } else {
                    None
                }
            }
        }
    }

    /// This function both draws and does the logic for each circle
    /// In the event of a released drag and drop that actually should build an edge it returns data about both entries
    /// This data will only ever work if it is between an input node and output node
    /// The return value will be of the form (input, output)
    fn implement_circles(
        &mut self,
        ui: &mut Ui,
        style: &GraphStyle,
        top_left: Pos2,
    ) -> Option<(Arc<NodeCircleIdentifier>, Arc<NodeCircleIdentifier>)> {
        // Iterate through all the inputs first then outputs
        let mut return_value = None; // By default
        for i in 0..(self.effect.input_count() + self.effect.output_count()) {
            // Want to get rect that is just offset from this so its on the edge

            // get a bool for if we are dealing with inputs
            let is_input = i < self.effect.input_count();
            let circle_index = match is_input {
                true => i,
                false => i - self.effect.input_count(),
            };

            let pos =
                self.get_interactable_circle_centres(ui, style, circle_index, top_left, is_input);

            // give position data to our node_circles

            // This trickery also discounts anything that is not input to output or output to input
            // The values will always be of the form (input, output)

            let other_data = match is_input {
                true => {
                    self.input_node_circles[circle_index].pos = pos;
                    // do the ui
                    self.input_node_circles[circle_index]
                        .node_circle_ui(ui, style, self.index)
                        .inner
                }
                false => {
                    self.output_node_circles[circle_index].pos = pos;
                    // do the ui
                    self.output_node_circles[circle_index]
                        .node_circle_ui(ui, style, self.index)
                        .inner
                }
            };
            let this_data = NodeCircleIdentifier {
                node_index: self.index,
                circle_index,
                circle_is_input: is_input,
            };

            // make sure we are actually sending data back
            match return_value {
                Some(_) => continue,
                None => return_value = self.get_edge_tuple(other_data, this_data.into()),
            }

            //println!("{:?} impl circles", return_value);
        }

        return_value
    }

    /// Get the rect for a general interactable circle
    fn get_interactable_circle_centres(
        &mut self,
        ui: &mut Ui,
        style: &GraphStyle,
        index: usize,
        top_left: Pos2,
        is_input: bool,
    ) -> Pos2 {
        let repeated_offset = Vec2 {
            x: 0.0,
            y: style.grid_row_height + 2.0 * ui.style().spacing.item_spacing.y,
        };

        let initial_offset = Vec2 {
            x: style.node_line_width * 0.5 + style.node_circle_radius,
            y: style.header_height
                + style.node_line_width
                + style.margin
                + style.grid_row_height * 0.7
                + style.node_circle_radius,
        };

        let output_offset = match is_input {
            true => Vec2::ZERO,
            false => Vec2 {
                x: match ui.available_width() {
                    0.0..200.0 => {
                        ui.available_width()
                            - style.node_circle_radius * 2.0
                            - style.node_line_width
                    }
                    _ => style.node_circle_radius,
                },
                y: 0.0,
            },
        };

        let pos = top_left + initial_offset + repeated_offset * index as f32 + output_offset;

        pos
    }

    pub fn get_circle_pos(&self, circle_index: usize, is_input: bool) -> Pos2 {
        if is_input {
            self.input_node_circles[circle_index].pos
        } else {
            self.output_node_circles[circle_index].pos
        }
    }

    /// Do the ui of the node
    /// Return data should be inner response with data about building a new edge if it is required
    pub fn node_ui(
        &mut self,
        ui: &mut Ui,
        style: &GraphStyle,
        audio_data: &GraphAudioData,
    ) -> InnerResponse<Option<(Arc<NodeCircleIdentifier>, Arc<NodeCircleIdentifier>)>> {
        let mut new_edge_data = None;
        let resp = egui::Area::new(egui::Id::new(format!("graph_node {}", self.index)))
            .show(ui.ctx(), |ui| {
                let top_left = ui.next_widget_position();

                // Draw the basic node
                self.draw_node_without_circles(ui, style, audio_data);

                // Do the stuff with the selectible nodes
                new_edge_data = self.implement_circles(ui, style, top_left);
                //println!("NODE: {:?}", new_edge_data);
            })
            .response;

        //println!("{:?} AAAAAAAAAAAAAA", new_edge_data);

        InnerResponse {
            inner: new_edge_data,
            response: resp,
        }
    }

    pub fn effect(&self) -> Arc<dyn Effect> {
        self.effect.clone()
    }

    pub fn index(&self) -> usize {
        self.index
    }
}
