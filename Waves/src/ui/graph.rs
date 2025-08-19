use eframe::{
    egui::{
        self, Area, Color32, Context, DragValue, Frame, Grid, Id, Label, LayerId, Margin, Pos2,
        Rect, Response, RichText, Sense, Shape, Slider, Stroke, StrokeKind, Ui, Vec2, Widget,
        Window, frame,
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

#[derive(Clone)]
pub struct Node {
    index: usize,
    pos: (f32, f32),
    height: f64,
    width: f64,
    inputs: Vec<Vec2>,
    outputs: Vec<Vec2>,
}

impl Node {
    /// This function should be called immediately
    fn draw_header(
        self,
        ui: &mut Ui,
        margin: f32,
        corner_radius: f32,
        line_width: f32,
        header_height: f32,
        header_colour: Color32,
        radius: f32,
    ) -> Response {
        let frame_pos = ui.next_widget_position();

        let header = egui::frame::Frame::new()
            .inner_margin(margin)
            .corner_radius(corner_radius)
            .show(ui, |ui| {
                let x = ui.available_width() - radius;
                let y = ui.available_height();

                // This little function gets the position of our frame in screen space

                // uncurved
                let rect_bottom = Rect::from_two_pos(
                    frame_pos
                        + Vec2 {
                            x: 0.0,
                            y: header_height / 2.0,
                        },
                    frame_pos
                        + Vec2 {
                            x: x + margin * 2.0,
                            y: header_height,
                        },
                );

                // curved
                let rect_top = Rect::from_two_pos(
                    frame_pos,
                    frame_pos
                        + Vec2 {
                            x: x + margin * 2.0,
                            y: header_height,
                        },
                );

                // get the shapes
                let shape_top = egui::frame::Frame::new()
                    .corner_radius(corner_radius)
                    .fill(header_colour)
                    .paint(rect_top);
                let shape_bottom = egui::frame::Frame::new()
                    .fill(header_colour)
                    .paint(rect_bottom);

                ui.painter().add(shape_top);
                ui.painter().add(shape_bottom);

                ui.add(
                    Label::new(
                        RichText::new("Effect")
                            .heading()
                            .size(header_height - 2.0 * margin - 2.0 * line_width),
                    )
                    .selectable(false),
                );
            });
        header.response
    }
}

impl Widget for Node {
    fn ui(mut self, ui: &mut Ui) -> Response {
        egui::Area::new(egui::Id::new(format!("graph_node {}", self.index)))
            .show(ui.ctx(), |ui| {
                let line_width = 1.5;
                let margin = 10.0;
                let corner_size = 10.0;
                let header_height = 40.0;

                let header_colour = Color32::GRAY;
                let main_colour = Color32::DARK_GRAY;
                let circle_colour = Color32::BLUE;

                let main_text_size = 16.0;
                let grid_row_height = main_text_size + 0.0;

                let radius = 6.0;

                let pos = ui.next_widget_position();

                egui::frame::Frame::new()
                    .stroke(Stroke::new(
                        line_width,
                        ui.style().visuals.widgets.open.weak_bg_fill,
                    ))
                    .fill(main_colour)
                    .corner_radius(corner_size)
                    .show(ui, |ui| {
                        // Header

                        self.clone().draw_header(
                            ui,
                            margin,
                            corner_size,
                            line_width,
                            header_height,
                            header_colour,
                            radius,
                        );

                        // Main Content

                        egui::frame::Frame::new()
                            .inner_margin(margin)
                            .show(ui, |ui| {
                                let grid = Grid::new("the_AWESOME_grid").show(ui, |ui| {
                                    ui.add(Label::new(RichText::new("input").size(main_text_size)));
                                    ui.add(Label::new(
                                        RichText::new("output").size(main_text_size),
                                    ));
                                    ui.end_row();

                                    ui.add(Label::new(RichText::new("input").size(main_text_size)));
                                });
                            });
                    });

                // Do the stuff with the selectible nodes
                let radius = 6.0;

                let offset = Vec2 {
                    x: 0.0,
                    y: grid_row_height * 1.25,
                };

                for i in 0..2 {
                    // Want to get rect that is just offset from this so its on the edge
                    let rect = Rect::from_two_pos(
                        pos + Vec2 {
                            x: -radius,
                            y: header_height - radius + line_width + margin + grid_row_height * 0.7,
                        } + offset * i as f32,
                        pos + Vec2 {
                            x: radius,
                            y: header_height + radius + line_width + margin + grid_row_height * 0.7,
                        } + offset * i as f32,
                    );

                    let circle = Shape::circle_filled(rect.center(), radius, circle_colour);
                    ui.painter().add(circle);

                    let r = ui.allocate_rect(rect, Sense::hover());

                    //println!("{:?}", r);
                    if r.hovered() {
                        println!("UNGA BUNGA");
                    }
                }
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
                Node {
                    index: 0,
                    pos: (100.0, 200.0),
                    height: 0.0,
                    width: 300.0,
                    inputs: vec![],
                    outputs: vec![],
                },
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
