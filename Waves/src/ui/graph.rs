use eframe::egui::{
    self, Area, Color32, Frame, Id, Label, Margin, Pos2, Rect, Response, RichText, Sense, Stroke,
    Ui, Vec2, Widget, Window, frame,
};
use egui_plot::Plot;

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
    ) -> Response {
        let frame_pos = ui.next_widget_position();

        let header = egui::frame::Frame::new()
            .inner_margin(margin)
            .corner_radius(corner_radius)
            .show(ui, |ui| {
                let x = ui.available_width();
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
                            x: x + 2.0 * margin,
                            y: header_height,
                        },
                );

                // curved
                let rect_top = Rect::from_two_pos(
                    frame_pos,
                    frame_pos
                        + Vec2 {
                            x: x + 2.0 * margin,
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

impl Widget for &Node {
    fn ui(self, ui: &mut Ui) -> Response {
        egui::Area::new(egui::Id::new(format!("graph_node {}", self.index)))
            .show(ui.ctx(), |ui| {
                let line_width = 1.5;
                let margin = 10.0;
                let corner_size = 10.0;
                let header_height = 40.0;

                let header_colour = Color32::GRAY;
                let main_colour = Color32::DARK_GRAY;

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
                        );

                        // Main Content

                        egui::frame::Frame::new()
                            .inner_margin(margin)
                            .show(ui, |ui| {
                                ui.dnd_drag_source(
                                    Id::new(format!("node_{}_output_0", self.index)),
                                    "rose is stinky",
                                    |ui| ui.label("This is an output"),
                                );

                                let (resp, dnd) = ui
                                    .dnd_drop_zone::<&'static str, Response>(Frame::new(), |ui| {
                                        ui.label("This is an input")
                                    });

                                let (resp, dnd) = ui
                                    .dnd_drop_zone::<&'static str, Response>(Frame::new(), |ui| {
                                        ui.label("This is an input")
                                    });

                                let (resp, dnd) = ui
                                    .dnd_drop_zone::<&'static str, Response>(Frame::new(), |ui| {
                                        ui.label("This is an input")
                                    });

                                if let Some(dnd) = dnd {
                                    println!("Dropped '{dnd}' here!")
                                }
                            });
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
                Node {
                    index: 0,
                    pos: (100.0, 200.0),
                    height: 200.0,
                    width: 300.0,
                    inputs: vec![],
                    outputs: vec![],
                },
                Node {
                    index: 1,
                    pos: (20.0, 20.0),
                    height: 200.0,
                    width: 300.0,
                    inputs: vec![],
                    outputs: vec![],
                },
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
            let resp = ui.add(&node);

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
