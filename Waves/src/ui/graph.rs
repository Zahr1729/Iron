use eframe::{
    egui::{
        self, Area, Color32, Context, DragValue, Frame, Grid, Id, Label, LayerId, Margin, Pos2,
        Rect, Response, RichText, Sense, Shape, Slider, Stroke, StrokeKind, Ui, Vec2, Widget,
        Window, frame, output,
    },
    epaint::{CircleShape, RectShape},
};
use egui_plot::Plot;

use crate::main;

pub struct NodeCircle {}

impl Widget for NodeCircle {
    fn ui(self, ui: &mut Ui) -> Response {
        let frame = Frame::new()
            .fill(Color32::BLUE)
            .corner_radius(30.0)
            .show(ui, |ui| ui.label(""));

        //ui.allocate_rect(rect, sense)

        if frame.response.hovered() {
            //println!("hovered");
        }
        frame.response
    }
}

#[derive(Clone, Copy)]
pub struct Node {
    index: usize,

    margin: f32,
    corner_radius: f32,
    line_width: f32,
    header_height: f32,
    main_text_size: f32,
    grid_row_height: f32,
    radius: f32,
    node_width: f32,

    main_colour: Color32,
    line_colour: Color32,
    header_colour: Color32,
    header_text_colour: Color32,
    main_text_colour: Color32,

    circle_line_colour: Color32,
    circle_colour: Color32,

    input_count: usize,
    output_count: usize,
}

impl Node {
    pub fn new(index: usize, input_count: usize, output_count: usize) -> Self {
        Self {
            index,
            margin: 10.0,
            corner_radius: 10.0,
            line_width: 2.0,
            header_height: 40.0,
            main_text_size: 16.0,
            grid_row_height: 16.0, // same as one above
            radius: 6.0,
            node_width: 12.0, // needs to be at least as big as the radius

            main_colour: Color32::DARK_GRAY,
            line_colour: Color32::from_rgb(200, 200, 200),
            header_colour: Color32::from_rgb(50, 50, 50),

            header_text_colour: Color32::from_rgb(220, 220, 220),
            main_text_colour: Color32::from_rgb(200, 200, 200),

            circle_line_colour: Color32::from_rgb(200, 200, 200),
            circle_colour: Color32::BLUE,

            input_count,
            output_count,
        }
    }

    /// This function should be called immediately
    fn draw_header(&self, ui: &mut Ui) -> Response {
        let frame_pos = ui.next_widget_position();

        let header = egui::frame::Frame::new()
            .inner_margin(self.margin)
            .corner_radius(self.corner_radius)
            .show(ui, |ui| {
                let x = ui.available_width() - self.radius;
                let y = ui.available_height();

                // This little function gets the position of our frame in screen space

                // uncurved
                let rect_bottom = Rect::from_two_pos(
                    frame_pos
                        + Vec2 {
                            x: 0.0,
                            y: self.header_height / 2.0,
                        },
                    frame_pos
                        + Vec2 {
                            x: x + self.margin * 2.0,
                            y: self.header_height,
                        },
                );

                // curved
                let rect_top = Rect::from_two_pos(
                    frame_pos,
                    frame_pos
                        + Vec2 {
                            x: x + self.margin * 2.0,
                            y: self.header_height,
                        },
                );

                // get the shapes
                let shape_top = egui::frame::Frame::new()
                    .corner_radius(self.corner_radius - 1.0)
                    .fill(self.header_colour)
                    .paint(rect_top);
                let shape_bottom = egui::frame::Frame::new()
                    .fill(self.header_colour)
                    .paint(rect_bottom);

                ui.painter().add(shape_top);
                ui.painter().add(shape_bottom);

                ui.add(
                    Label::new(
                        RichText::new("Effect")
                            .heading()
                            .size(self.header_height - 2.0 * self.margin - 2.0 * self.line_width)
                            .color(self.header_text_colour),
                    )
                    .selectable(false),
                );
            });
        header.response
    }

    fn draw_main(&self, ui: &mut Ui) {
        egui::frame::Frame::new()
            .inner_margin(self.margin)
            .show(ui, |ui| {
                Grid::new(format!("the_AWESOME_grid_ {}", self.index)).show(ui, |ui| {
                    ui.add(Label::new(
                        RichText::new("input")
                            .size(self.main_text_size)
                            .color(self.main_text_colour),
                    ));
                    ui.add(Label::new(
                        RichText::new("output")
                            .size(self.main_text_size)
                            .color(self.main_text_colour),
                    ));
                    ui.end_row();

                    ui.add(Label::new(
                        RichText::new("input")
                            .size(self.main_text_size)
                            .color(self.main_text_colour),
                    ));
                });
            });
    }

    fn draw_node_without_circles(&self, ui: &mut Ui) {
        egui::frame::Frame::new()
            .outer_margin(self.radius)
            .stroke(Stroke::new(self.line_width, self.line_colour))
            .fill(self.main_colour)
            .corner_radius(self.corner_radius)
            .show(ui, |ui| {
                // Header

                self.clone().draw_header(ui);

                // Main Content

                self.clone().draw_main(ui);
            });
    }

    /// This function both draws and does the logic for each circle
    fn implement_circles(self, ui: &mut Ui, top_left: Pos2) {
        // Iterate through all the inputs first then outputs
        for i in 0..(self.input_count + self.output_count) {
            // Want to get rect that is just offset from this so its on the edge

            // get a bool for if we are dealing with inputs
            let is_input = i < self.input_count;

            let rect = self.get_interactable_circle_rects(ui, i, top_left, is_input);

            // first draw them
            let circle_edge = Shape::Circle(CircleShape::stroke(
                rect.center(),
                self.radius,
                Stroke::new(self.line_width, self.circle_line_colour),
            ));
            let circle = Shape::circle_filled(rect.center(), self.radius, self.circle_colour);
            ui.painter().add(circle);
            ui.painter().add(circle_edge);

            // now implement logic for them

            let r = ui.allocate_rect(rect, Sense::all());

            //println!("{:?}", r);
            if r.dragged() {
                println!("UNGA BUNGA");
            }
        }
    }

    /// Get the rect for a general interactable circle
    fn get_interactable_circle_rects(
        &self,
        ui: &mut Ui,
        index: usize,
        top_left: Pos2,
        is_input: bool,
    ) -> Rect {
        let repeated_offset = Vec2 {
            x: 0.0,
            y: self.grid_row_height * 1.25,
        };

        let initial_offset = Vec2 {
            x: self.line_width * 0.5 + self.radius,
            y: self.header_height + self.line_width + self.margin + self.grid_row_height * 0.7,
        };

        let radius_offset = Vec2 {
            x: self.radius,
            y: self.radius,
        };

        let output_offset = match is_input {
            true => Vec2::ZERO,
            false => Vec2 {
                x: ui.available_width() - self.radius * 2.0 - self.line_width * 0.5,
                y: 0.0,
            },
        };

        let pos = top_left + initial_offset + repeated_offset * index as f32 + output_offset;

        Rect::from_two_pos(pos - radius_offset, pos + radius_offset)
    }
}

impl Widget for Node {
    fn ui(mut self, ui: &mut Ui) -> Response {
        egui::Area::new(egui::Id::new(format!("graph_node {}", self.index)))
            .show(ui.ctx(), |ui| {
                egui::Frame::new().fill(Color32::RED).show(ui, |ui| {
                    let top_left = ui.next_widget_position();

                    // Draw the basic node
                    self.draw_node_without_circles(ui);

                    // Do the stuff with the selectible nodes
                    self.implement_circles(ui, top_left);

                    self.node_width = ui.available_width();
                    println!("{}", self.node_width);
                });
            })
            .response
    }
}

pub struct Edge {
    start: Vec2,
    end: Vec2,
}

pub struct NodeGraph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

impl NodeGraph {
    pub fn new() -> Self {
        Self {
            nodes: vec![
                Node::new(0, 2, 1),
                // Node {
                //     index: 1,
                //     pos: (20.0, 20.0),
                //     height: 200.0,
                //     width: 300.0,
                //     inputs: vec![],
                //     outputs: vec![],
                // },
            ],
            edges: vec![],
        }
    }
}

impl Widget for NodeGraph {
    fn ui(self, ui: &mut eframe::egui::Ui) -> Response {
        // let mut area = area.begin(ctx);

        // area.with_widget_info(|| WidgetInfo::labeled(WidgetType::Window, true, title.text()));
        let mut r = None;
        for node in self.nodes {
            let resp = ui.add(node);

            r = Some(resp);
        }

        r.unwrap()
    }
}

#[cfg(test)]
mod test {
    use crate::ui::graph::NodeGraph;

    #[test]
    fn test_graph_works() {
        let graph = NodeGraph {
            nodes: vec![],
            edges: vec![],
        };
    }
}
