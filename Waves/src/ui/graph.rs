use std::sync::Arc;

use eframe::{
    egui::{
        self, Area, Color32, Context, DragAndDrop, DragValue, Frame, Grid, Id, InnerResponse,
        Label, LayerId, Margin, Pos2, Rect, Response, RichText, Sense, Shape, Slider, Stroke,
        StrokeKind, Ui, Vec2, Widget, Window, frame, output, style::default_text_styles,
    },
    epaint::{CircleShape, CubicBezierShape, PathStroke, RectShape},
};
use egui_plot::Plot;

use crate::main;

/// This is to collect data for the edge drag and drop to connect things up
#[derive(Debug, Copy, Clone)]
struct NodeCircleIdentifier {
    node_index: usize,
    circle_index: usize,
    circle_is_input: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeCircle {
    index: usize,
    is_input: bool,
    pos: Pos2,
    radius: f32,
    line_width: f32,
    circle_colour: Color32,
    edge_colour: Color32,
}

impl NodeCircle {
    pub fn new(
        index: usize,
        is_input: bool,
        pos: Pos2,
        radius: f32,
        line_width: f32,
        circle_colour: Color32,
        edge_colour: Color32,
    ) -> Self {
        Self {
            index,
            is_input,
            pos,
            radius,
            line_width,
            circle_colour,
            edge_colour,
        }
    }
}

fn draw_circle(
    ui: &mut Ui,
    pos: Pos2,
    radius: f32,
    line_width: f32,
    circle_colour: Color32,
    edge_colour: Color32,
) {
    let circle_edge = Shape::Circle(CircleShape::stroke(
        pos,
        radius,
        Stroke::new(line_width, edge_colour),
    ));
    let circle = Shape::circle_filled(pos, radius, circle_colour);
    ui.painter().add(circle);
    ui.painter().add(circle_edge);
}

fn get_generic_circle_rect(ui: &mut Ui, pos: Pos2, radius: f32) -> Rect {
    let radius_offset = Vec2 {
        x: radius,
        y: radius,
    };

    Rect::from_two_pos(pos - radius_offset, pos + radius_offset)
}

impl NodeCircle {
    /// This function's inner response contains identifying information for the node that began the drag and drop when it is released
    fn node_circle_ui(
        self,
        ui: &mut Ui,
        parent_index: usize,
    ) -> InnerResponse<Option<Arc<NodeCircleIdentifier>>> {
        // First do logic and draw bezier underneath

        let rect = get_generic_circle_rect(ui, self.pos, self.radius);

        let r = ui.allocate_rect(rect, Sense::all());

        //println!("{:?}", r);
        if r.dragged() {
            // Setup the payload
            let payload = NodeCircleIdentifier {
                node_index: parent_index,
                circle_index: self.index,
                circle_is_input: self.is_input,
            };
            DragAndDrop::set_payload(ui.ctx(), payload);

            match r.hover_pos() {
                Some(mouse_pos) => {
                    // Draw the external boundary bezier
                    Edge::draw_bezier(
                        ui,
                        self.pos,
                        mouse_pos,
                        self.line_width * 2.0 + self.radius,
                        self.edge_colour,
                    );

                    draw_circle(
                        ui,
                        self.pos,
                        self.radius,
                        self.line_width,
                        self.circle_colour,
                        self.edge_colour,
                    );

                    // draw mouse circle after the bezier
                    draw_circle(
                        ui,
                        mouse_pos,
                        self.radius,
                        self.line_width,
                        self.circle_colour,
                        self.edge_colour,
                    );

                    // Draw the internal bezier so it looks nice
                    Edge::draw_bezier(ui, self.pos, mouse_pos, self.radius, self.circle_colour);
                }
                _ => (),
            }

            // So we don't draw circle again
        } else {
            // Just draw circle
            draw_circle(
                ui,
                self.pos,
                self.radius,
                self.line_width,
                self.circle_colour,
                self.edge_colour,
            );
        }

        let return_value;

        let payload = r.dnd_release_payload::<NodeCircleIdentifier>();
        return_value = InnerResponse::new(payload, r);

        return_value
    }
}

#[derive(Debug, Clone)]
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

    input_node_circles: Vec<NodeCircle>,
    output_node_circles: Vec<NodeCircle>,
}

impl Node {
    pub fn new(index: usize, input_count: usize, output_count: usize) -> Self {
        let line_width = 2.0;
        let radius = 6.0;
        let circle_colour = Color32::BLUE;
        let line_colour = Color32::from_rgb(200, 200, 200);

        Self {
            index,
            margin: 10.0,
            corner_radius: 10.0,
            line_width,
            header_height: 40.0,
            main_text_size: 20.0,
            grid_row_height: 20.0, // same as one above
            radius,
            node_width: 12.0, // needs to be at least as big as the radius

            main_colour: Color32::DARK_GRAY,
            line_colour,
            header_colour: Color32::from_rgb(50, 50, 50),

            header_text_colour: Color32::from_rgb(220, 220, 220),
            main_text_colour: Color32::from_rgb(200, 200, 200),

            circle_line_colour: line_colour,
            circle_colour,

            input_count,
            output_count,

            input_node_circles: (0..input_count)
                .into_iter()
                .map(|i| {
                    NodeCircle::new(
                        i,
                        true,
                        Pos2::ZERO,
                        radius,
                        line_width,
                        circle_colour,
                        line_colour,
                    )
                })
                .collect::<Vec<_>>(),
            output_node_circles: (0..output_count)
                .into_iter()
                .map(|i| {
                    NodeCircle::new(
                        i,
                        false,
                        Pos2::ZERO,
                        radius,
                        line_width,
                        circle_colour,
                        line_colour,
                    )
                })
                .collect::<Vec<_>>(),
        }
    }

    /// This function should be called immediately
    fn draw_header(&self, ui: &mut Ui) -> Response {
        let frame_pos = ui.next_widget_position();

        let header = egui::frame::Frame::new()
            .inner_margin(self.margin)
            .corner_radius(self.corner_radius)
            .show(ui, |ui| {
                let x = ui.available_width();
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
        //println!("{:?}", ui.style().spacing);
        egui::frame::Frame::new()
            .inner_margin(self.margin)
            .show(ui, |ui| {
                Grid::new(format!("the_AWESOME_grid_ {}", self.index))
                    //.spacing(Vec2::ZERO)
                    //.with_row_color(|i, s| Some(Color32::RED))
                    .show(ui, |ui| {
                        for i in 0..self.input_count.max(self.output_count) {
                            if (i < self.input_count) {
                                ui.add(Label::new(
                                    RichText::new("input")
                                        .size(self.main_text_size)
                                        .color(self.main_text_colour),
                                ));
                            } else {
                                ui.add(Label::new(RichText::new("")));
                            }

                            if (i < self.output_count) {
                                ui.add(Label::new(
                                    RichText::new("output")
                                        .size(self.main_text_size)
                                        .color(self.main_text_colour),
                                ));
                            } else {
                                ui.add(Label::new(RichText::new("")));
                            }

                            ui.end_row();
                        }
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

    /// Return the appropriate (input, output) tuple if the edge is clearly (but not necessarily) valid
    /// (ie it doesnt feed into itself and input goes to output)
    /// We must do a more in depth check to make sure we have no cycles in the NodeGraph struct later.
    fn get_edge_tuple(
        &self,
        other_data: Option<Arc<NodeCircleIdentifier>>,
        this_data: Arc<NodeCircleIdentifier>,
    ) -> Option<(Arc<NodeCircleIdentifier>, Arc<NodeCircleIdentifier>)> {
        match other_data {
            None => None,
            Some(other_circle) => {
                if self.index != other_circle.node_index {
                    let this_circle = Arc::<NodeCircleIdentifier>::new(NodeCircleIdentifier {
                        node_index: self.index,
                        circle_index: this_data.circle_index,
                        circle_is_input: this_data.circle_is_input,
                    });

                    println!("NEW LINE YAY");

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
        mut self,
        ui: &mut Ui,
        top_left: Pos2,
    ) -> Option<(Arc<NodeCircleIdentifier>, Arc<NodeCircleIdentifier>)> {
        // Iterate through all the inputs first then outputs
        let mut return_value = None; // By default
        for i in 0..(self.input_count + self.output_count) {
            // Want to get rect that is just offset from this so its on the edge

            // get a bool for if we are dealing with inputs
            let is_input = i < self.input_count;
            let circle_index = match is_input {
                true => i,
                false => i - self.input_count,
            };

            let pos = self.get_interactable_circle_centres(ui, circle_index, top_left, is_input);

            // give position data to our node_circles

            // This trickery also discounts anything that is not input to output or output to input
            // The values will always be of the form (input, output)

            let other_data = match is_input {
                true => {
                    self.input_node_circles[circle_index].pos = pos;
                    // do the ui
                    self.input_node_circles[circle_index]
                        .node_circle_ui(ui, self.index)
                        .inner
                }
                false => {
                    self.output_node_circles[circle_index].pos = pos;
                    // do the ui
                    self.output_node_circles[circle_index]
                        .node_circle_ui(ui, self.index)
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
        &self,
        ui: &mut Ui,
        index: usize,
        top_left: Pos2,
        is_input: bool,
    ) -> Pos2 {
        let repeated_offset = Vec2 {
            x: 0.0,
            y: self.grid_row_height + 2.0 * ui.style().spacing.item_spacing.y,
        };

        let initial_offset = Vec2 {
            x: self.line_width * 0.5 + self.radius,
            y: self.header_height
                + self.line_width
                + self.margin
                + self.grid_row_height * 0.7
                + self.radius,
        };

        let output_offset = match is_input {
            true => Vec2::ZERO,
            false => Vec2 {
                x: match ui.available_width() {
                    0.0..200.0 => ui.available_width() - self.radius * 2.0 - self.line_width,
                    _ => self.radius,
                },
                y: 0.0,
            },
        };

        let pos = top_left + initial_offset + repeated_offset * index as f32 + output_offset;

        pos
    }

    fn get_circle_pos(&self, circle_index: usize, is_input: bool) -> Pos2 {
        if is_input {
            self.input_node_circles[circle_index].pos
        } else {
            self.output_node_circles[circle_index].pos
        }
    }

    /// Do the ui of the node
    /// Return data should be inner response with data about building a new edge if it is required
    fn node_ui(
        self,
        ui: &mut Ui,
    ) -> InnerResponse<Option<(Arc<NodeCircleIdentifier>, Arc<NodeCircleIdentifier>)>> {
        let mut new_edge_data = None;
        let resp = egui::Area::new(egui::Id::new(format!("graph_node {}", self.index)))
            .show(ui.ctx(), |ui| {
                let top_left = ui.next_widget_position();

                // Draw the basic node
                self.draw_node_without_circles(ui);

                // Do the stuff with the selectible nodes
                new_edge_data = self.implement_circles(ui, top_left);
                //println!("NODE: {:?}", new_edge_data);
            })
            .response;

        //println!("{:?} AAAAAAAAAAAAAA", new_edge_data);

        InnerResponse {
            inner: new_edge_data,
            response: resp,
        }
    }
}

/// This is Expected to be expanded to include data, but for now we move
#[derive(Debug, Clone, Copy)]
pub struct Edge {
    start: NodeCircleIdentifier,
    end: NodeCircleIdentifier,

    inner_colour: Color32,
    outer_colour: Color32,

    inner_width: f32,
    line_width: f32,
}

impl Edge {
    pub fn new(start: NodeCircleIdentifier, end: NodeCircleIdentifier) -> Self {
        let line_width = 2.0;
        let radius = 6.0;
        let circle_colour = Color32::BLUE;
        let line_colour = Color32::from_rgb(200, 200, 200);

        Self {
            start,
            end,
            inner_colour: circle_colour,
            outer_colour: line_colour,
            inner_width: radius,
            line_width,
        }
    }

    fn get_cubic_bezier_coords(a: Pos2, b: Pos2) -> [Pos2; 4] {
        [a, Pos2 { x: b.x, y: a.y }, Pos2 { x: a.x, y: b.y }, b]
    }

    fn draw_bezier(ui: &mut Ui, start: Pos2, end: Pos2, width: f32, colour: Color32) {
        let points = Edge::get_cubic_bezier_coords(start, end);

        let bezier = CubicBezierShape {
            points,
            closed: false,
            fill: Color32::TRANSPARENT,
            stroke: PathStroke::new(width, colour),
        };

        //ui.painter().add(top_bezier);
        ui.painter().add(bezier);
    }

    fn get_start_pos(&self, graph: &NodeGraph) -> Pos2 {
        graph.get_node_circle_pos(self.start)
    }

    fn get_end_pos(&self, graph: &NodeGraph) -> Pos2 {
        graph.get_node_circle_pos(self.end)
    }

    fn draw_outer(&self, ui: &mut Ui, graph: &NodeGraph) {
        //println!("SHOULD BE WORKING");
        let start = self.get_start_pos(graph);
        let end = self.get_end_pos(graph);

        println!("{:?}, {:?}", start, end);

        println!("drawing bezier outer");

        Edge::draw_bezier(ui, start, end, self.line_width, self.outer_colour);
    }

    fn draw_inner(&self, ui: &mut Ui, graph: &NodeGraph) {
        //println!("{:?}", graph);
        let start = self.get_start_pos(graph);
        let end = self.get_end_pos(graph);

        Edge::draw_bezier(ui, start, end, self.inner_width, self.inner_colour);
    }
}

#[derive(Debug)]
pub struct NodeGraph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

impl NodeGraph {
    pub fn new() -> Self {
        Self {
            nodes: vec![Node::new(0, 2, 1), Node::new(1, 1, 2)],
            edges: vec![],
        }
    }

    /// This function is given data to add a new edge
    /// It checks if it is a valid edge, and if so it adds it (if not do a println for now)
    fn add_edge(&mut self, start: NodeCircleIdentifier, end: NodeCircleIdentifier) {
        // Check for updates (primarily this is detecting the instantiation of a)
        //println!("ADDING EDGE");
        self.edges.push(Edge::new(start, end));
    }

    fn get_node_circle_pos(&self, identifier: NodeCircleIdentifier) -> Pos2 {
        self.nodes[identifier.node_index]
            .get_circle_pos(identifier.node_index, identifier.circle_is_input)
    }

    pub fn node_graph_ui(&mut self, ui: &mut eframe::egui::Ui) -> Response {
        // let mut area = area.begin(ctx);

        //println!("{}", self.edges.len());

        // Draw the outer edges
        for i in 0..self.edges.len() {
            let edge = self.edges[i].clone();
            edge.draw_outer(ui, self);
        }

        // do node ui (and if we have done the prerequisites for edge, add new edge);
        let mut r = None;
        for i in 0..self.nodes.len() {
            let node = self.nodes[i].clone();
            let inner_resp = node.node_ui(ui);

            match inner_resp.inner {
                None => (),
                Some((start, end)) => {
                    self.add_edge(*start, *end);
                }
            }

            r = Some(inner_resp.response);
        }

        // Draw the inner edges (doing it this way makes it look cohesive)
        for i in 0..self.edges.len() {
            let edge = self.edges[i].clone();
            edge.draw_inner(ui, self);
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
