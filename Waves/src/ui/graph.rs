use eframe::egui::{
    self, Area, Color32, Frame, Id, Label, Margin, Pos2, Response, RichText, Sense, Stroke, Ui,
    Vec2, Widget, Window,
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

impl Widget for &Node {
    fn ui(self, ui: &mut Ui) -> Response {
        egui::Area::new(egui::Id::new(format!("graph_node {}", self.index)))
            .movable(true)
            .enabled(true)
            .show(ui.ctx(), |ui| {
                let margin = 8;

                egui::frame::Frame::new()
                    .stroke(Stroke::new(
                        1.0,
                        ui.style().visuals.widgets.open.weak_bg_fill,
                    ))
                    .inner_margin(Margin::same(margin))
                    .corner_radius(ui.style().visuals.widgets.open.corner_radius)
                    .show(ui, |ui| {
                        ui.add(
                            Label::new(RichText::new("Header text! (drag me)").heading())
                                .selectable(false),
                        );

                        ui.add(egui::Separator::default().grow(margin as f32));

                        ui.label("Floating text!");

                        ui.dnd_drag_source(
                            Id::new(format!("node_{}_output_0", self.index)),
                            "rose is stinky",
                            |ui| ui.label("This is an output"),
                        );

                        let (resp, dnd) = ui
                            .dnd_drop_zone::<&'static str, Response>(Frame::new(), |ui| {
                                ui.label("This is an input")
                            });

                        if let Some(dnd) = dnd {
                            println!("Dropped '{dnd}' here!")
                        }
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
        let window = egui::Window::new("test").show(ui.ctx(), |ui| ui.label("wot"));

        // let mut area = area.begin(ctx);

        // area.with_widget_info(|| WidgetInfo::labeled(WidgetType::Window, true, title.text()));
        let mut r = None;
        for (i, node) in self.nodes.iter().enumerate() {
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
