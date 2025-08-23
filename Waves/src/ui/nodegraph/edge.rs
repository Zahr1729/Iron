use eframe::{
    egui::{Color32, Pos2, Ui},
    epaint::{CubicBezierShape, PathStroke},
};

use crate::ui::nodegraph::{GraphStyle, NodeGraph, nodecircle::NodeCircleIdentifier};

/// This is Expected to be expanded to include data, but for now we move
#[derive(Debug, Clone)]
pub struct Edge {
    pub input: NodeCircleIdentifier,
    pub output: NodeCircleIdentifier,
}

impl Edge {
    pub fn new(input: NodeCircleIdentifier, output: NodeCircleIdentifier) -> Self {
        Self { input, output }
    }

    fn get_cubic_bezier_coords(a: Pos2, b: Pos2) -> [Pos2; 4] {
        [a, Pos2 { x: b.x, y: a.y }, Pos2 { x: a.x, y: b.y }, b]
    }

    pub fn draw_bezier(ui: &mut Ui, start: Pos2, end: Pos2, width: f32, colour: Color32) {
        let points = Edge::get_cubic_bezier_coords(start, end);

        let bezier = CubicBezierShape {
            points,
            closed: false,
            fill: Color32::TRANSPARENT,
            stroke: PathStroke::new(width, colour),
        };

        ui.painter().add(bezier);
    }

    fn get_input_pos(&self, graph: &NodeGraph) -> Pos2 {
        graph.get_node_circle_pos(self.input)
    }

    fn get_output_pos(&self, graph: &NodeGraph) -> Pos2 {
        graph.get_node_circle_pos(self.output)
    }

    fn draw_outer(&self, ui: &mut Ui, style: &GraphStyle, graph: &NodeGraph) {
        //println!("SHOULD BE WORKING");
        let input = self.get_input_pos(graph);
        let output = self.get_output_pos(graph);

        Edge::draw_bezier(
            ui,
            input,
            output,
            style.edge_line_width * 2.0 + style.edge_inner_width,
            style.edge_outer_colour,
        );
    }

    fn draw_inner(&self, ui: &mut Ui, style: &GraphStyle, graph: &NodeGraph, colour: Color32) {
        //println!("{:?}", graph);
        let start = self.get_input_pos(graph);
        let end = self.get_output_pos(graph);

        Edge::draw_bezier(ui, start, end, style.edge_inner_width, colour);
    }

    fn draw_start_node(
        &self,
        ui: &mut Ui,
        style: &GraphStyle,
        graph: &NodeGraph,
        circle_colour: Color32,
    ) {
        let node = &graph.nodes[self.input.node_index];
        node.input_node_circles[self.input.circle_index].draw(
            ui,
            style.edge_line_width,
            circle_colour,
            style.edge_outer_colour,
        );
    }

    fn draw_end_node(
        &self,
        ui: &mut Ui,
        style: &GraphStyle,
        graph: &NodeGraph,
        circle_colour: Color32,
    ) {
        let node = &graph.nodes[self.output.node_index];
        node.output_node_circles[self.output.circle_index].draw(
            ui,
            style.edge_line_width,
            circle_colour,
            style.edge_outer_colour,
        );
    }

    pub fn draw_edge(&self, ui: &mut Ui, style: &GraphStyle, graph: &NodeGraph, colour: Color32) {
        // draw outer edge
        self.draw_outer(ui, style, graph);

        // draw nodes
        self.draw_start_node(ui, style, graph, colour);
        self.draw_end_node(ui, style, graph, colour);

        // draw inner edge
        self.draw_inner(ui, style, graph, colour);
    }
}
