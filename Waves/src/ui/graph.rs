use std::{default, hash::Hash, sync::Arc};

use eframe::{
    egui::{
        self, Area, Color32, Context, DragAndDrop, DragValue, Frame, Grid, Id, InnerResponse,
        Label, LayerId, Margin, Order, Pos2, Rect, Response, RichText, Sense, Shape, Slider,
        Stroke, StrokeKind, Ui, UiBuilder, UiStackInfo, Vec2, Widget, Window, ahash::HashMap,
        frame, mutex::Mutex, output, style::default_text_styles,
    },
    epaint::{CircleShape, CubicBezierShape, PathStroke, RectShape},
};

use crate::{
    audio::effects::{Add, Effect, Gain, Output, SineWave, Zero},
    common::{dB, track::Track},
};

#[derive(Default, Debug)]
pub struct GraphAudioData {
    pub current_sample: usize,
    pub sample_rate: u32,
}

impl GraphAudioData {
    pub fn new(current_sample: usize, sample_rate: u32) -> Self {
        Self {
            current_sample,
            sample_rate,
        }
    }
}

/// This is the datastructure for colours, sizes, line width etc,
/// it should be passed throughout the graph and all style information
/// should be accessible through this struct
#[derive(Debug)]
pub struct GraphStyle {
    node_line_width: f32,
    node_circle_radius: f32,

    edge_inner_width: f32,
    edge_line_width: f32,

    edge_inner_colour: Color32,
    edge_outer_colour: Color32,
    edge_drag_colour: Color32,

    circle_colour: Color32,
    line_colour: Color32,

    corner_radius: f32,
    margin: f32,
    plot_margin: f32,

    plot_height: f32,
    plot_width: f32,

    header_height: f32,
    header_text_size: f32,

    header_colour: Color32,
    header_text_colour: Color32,

    main_text_size: f32,
    grid_row_height: f32,

    main_colour: Color32,
    main_text_colour: Color32,
}

impl Default for GraphStyle {
    fn default() -> Self {
        Self {
            node_line_width: 2.0,
            node_circle_radius: 6.0,

            edge_inner_width: 6.0,
            edge_line_width: 2.0,

            edge_inner_colour: Color32::from_rgb(13, 116, 15),
            edge_outer_colour: Color32::from_rgb(200, 200, 200),
            edge_drag_colour: Color32::from_rgb(194, 136, 11),
            // rgba(194, 136, 11, 1)
            circle_colour: Color32::BLUE,
            line_colour: Color32::from_rgb(200, 200, 200),

            corner_radius: 10.0,
            margin: 10.0,
            plot_margin: 5.0,

            plot_height: 75.0,
            plot_width: 150.0,

            header_height: 40.0,
            header_text_size: 20.0,

            header_colour: Color32::from_rgb(50, 50, 50),
            header_text_colour: Color32::from_rgb(220, 220, 220),

            main_text_size: 18.0,
            grid_row_height: 18.0,

            main_colour: Color32::DARK_GRAY,
            main_text_colour: Color32::from_rgb(200, 200, 200),
        }
    }
}

/// This is to collect data for the edge drag and drop to connect things up
#[derive(Debug, Copy, Clone, PartialEq)]
struct NodeCircleIdentifier {
    node_index: usize,
    circle_index: usize,
    circle_is_input: bool,
}

impl NodeCircleIdentifier {
    pub fn new(node_index: usize, circle_index: usize, circle_is_input: bool) -> Self {
        NodeCircleIdentifier {
            node_index,
            circle_index,
            circle_is_input,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NodeCircle {
    index: usize,
    is_input: bool,
    pos: Pos2,
    radius: f32,
}

impl NodeCircle {
    pub fn new(index: usize, is_input: bool, pos: Pos2, radius: f32) -> Self {
        Self {
            index,
            is_input,
            pos,
            radius,
        }
    }
}

fn draw_circle(
    ui: &mut Ui,
    pos: Pos2,
    radius: f32,
    line_width: f32,
    circle_colour: Color32,
    boundary_colour: Color32,
) {
    let circle_edge = Shape::Circle(CircleShape::stroke(
        pos,
        radius,
        Stroke::new(line_width, boundary_colour),
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
        style: &GraphStyle,
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
                        style.edge_line_width * 2.0 + self.radius,
                        style.line_colour,
                    );

                    self.draw(
                        ui,
                        style.edge_line_width,
                        style.edge_drag_colour,
                        style.line_colour,
                    );

                    // draw mouse circle after the bezier
                    draw_circle(
                        ui,
                        mouse_pos,
                        self.radius,
                        style.edge_line_width,
                        style.edge_drag_colour,
                        style.line_colour,
                    );

                    // Draw the internal bezier so it looks nice
                    Edge::draw_bezier(ui, self.pos, mouse_pos, self.radius, style.edge_drag_colour);
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
                style.node_line_width,
                style.circle_colour,
                style.line_colour,
            );
        }

        let return_value;

        let payload = r.dnd_release_payload::<NodeCircleIdentifier>();
        return_value = InnerResponse::new(payload, r);

        return_value
    }

    fn draw(&self, ui: &mut Ui, line_width: f32, circle_colour: Color32, boundary_colour: Color32) {
        draw_circle(
            ui,
            self.pos,
            self.radius,
            line_width,
            circle_colour,
            boundary_colour,
        );
    }
}

#[derive(Clone)]
pub struct Node {
    index: usize,

    effect: Arc<dyn Effect>,

    input_node_circles: Vec<NodeCircle>,
    output_node_circles: Vec<NodeCircle>,
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
                let y = ui.available_height();

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
}

/// This is Expected to be expanded to include data, but for now we move
#[derive(Debug, Clone)]
pub struct Edge {
    input: NodeCircleIdentifier,
    output: NodeCircleIdentifier,
}

impl Edge {
    pub fn new(input: NodeCircleIdentifier, output: NodeCircleIdentifier) -> Self {
        Self { input, output }
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

    fn draw_inner(&self, ui: &mut Ui, style: &GraphStyle, graph: &NodeGraph) {
        //println!("{:?}", graph);
        let start = self.get_input_pos(graph);
        let end = self.get_output_pos(graph);

        Edge::draw_bezier(
            ui,
            start,
            end,
            style.edge_inner_width,
            style.edge_inner_colour,
        );
    }

    fn draw_start_node(&self, ui: &mut Ui, style: &GraphStyle, graph: &NodeGraph) {
        let node = &graph.nodes[self.input.node_index];
        node.input_node_circles[self.input.circle_index].draw(
            ui,
            style.edge_line_width,
            style.edge_inner_colour,
            style.edge_outer_colour,
        );
    }

    fn draw_end_node(&self, ui: &mut Ui, style: &GraphStyle, graph: &NodeGraph) {
        let node = &graph.nodes[self.output.node_index];
        node.output_node_circles[self.output.circle_index].draw(
            ui,
            style.edge_line_width,
            style.edge_inner_colour,
            style.edge_outer_colour,
        );
    }

    fn draw_edge(&self, ui: &mut Ui, style: &GraphStyle, graph: &NodeGraph) {
        // draw outer edge
        self.draw_outer(ui, style, graph);

        // draw nodes
        self.draw_start_node(ui, style, graph);
        self.draw_end_node(ui, style, graph);

        // draw inner edge
        self.draw_inner(ui, style, graph);
    }
}

pub struct ArcWrapper(Arc<dyn Effect>);

impl PartialEq for ArcWrapper {
    fn eq(&self, other: &Self) -> bool {
        let s = Arc::into_raw(self.0.clone());
        let sother = Arc::into_raw(other.0.clone());

        s == sother
    }
}

impl Eq for ArcWrapper {}

impl Hash for ArcWrapper {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let s = Arc::into_raw(self.0.clone());

        s.hash(state);
    }
}

pub struct NodeGraph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    style: GraphStyle,
    pub zero: Arc<Zero>,
    pub output: Arc<Output>,
    hash: HashMap<ArcWrapper, usize>,
    pub audio_data: GraphAudioData,
}

impl NodeGraph {
    pub fn new_non_trivial() -> Self {
        let mut s = Self::new();

        let g1 = Arc::new(Gain::new(dB(-12.0), s.zero.clone()));
        let g2 = Arc::new(Gain::new(dB(12.0), s.zero.clone()));
        let s1 = Arc::new(SineWave::new(0.5, 440.0, 0.0));
        let a1 = Arc::new(Add::new(s.zero.clone(), s.zero.clone()));

        s.add_node(g1);
        s.add_node(g2);
        s.add_node(s1);
        s.add_node(a1);

        s
    }

    pub fn new() -> Self {
        let zero = Arc::new(Zero);
        let output = Arc::new(Output::new(zero.clone()));
        Self {
            nodes: vec![
                Node::new(0, zero.clone(), 6.0),
                Node::new(1, output.clone(), 6.0),
            ],
            edges: vec![],
            style: GraphStyle::default(),
            zero: zero.clone(),
            output: output.clone(),
            hash: Default::default(),
            audio_data: Default::default(),
        }
    }

    pub fn add_track(&mut self, track: Arc<Track>) {
        self.add_node(track);
    }

    fn add_node(&mut self, effect: Arc<dyn Effect>) {
        let index = self.nodes.len();
        let node = Node::new(index, effect.clone(), 6.0);
        self.hash.insert(ArcWrapper(effect), index);
        self.nodes.push(node);
    }

    /// This function is given data to add a new edge
    /// It checks if it is a valid edge, and if so it adds it (if not do a println for now)
    fn add_edge(&mut self, input: NodeCircleIdentifier, output: NodeCircleIdentifier) {
        // Check for updates (primarily this is detecting the instantiation of a)
        //println!("ADDING EDGE");
        let e = self.nodes[input.node_index].effect.clone();
        let _ = e.set_input_at_index(
            input.circle_index,
            self.nodes[output.node_index].effect.clone(),
        );
        //self.edges.push(Edge::new(input, output));
    }

    fn get_node_circle_pos(&self, identifier: NodeCircleIdentifier) -> Pos2 {
        self.nodes[identifier.node_index]
            .get_circle_pos(identifier.circle_index, identifier.circle_is_input)
    }

    /// Returns true iff the node circle identifiers provided give the same edge as one already existing
    fn check_edge_already_exists(
        &self,
        input: NodeCircleIdentifier,
        output: NodeCircleIdentifier,
    ) -> bool {
        for edge in &self.edges {
            if edge.input == input && edge.output == output {
                return true;
            }
        }
        false
    }

    /// Returns true if it detects a path only going upstream between two nodes
    /// ie from the output nodes the input data
    /// This means in terms of circle nodes its really going towards the output circles (which really are inputs to the edges)
    /// The way its designed means its the only really acceptable one
    fn dfs_upstream_path(
        &self,
        current_effect: &ArcWrapper,
        destination_effect: &ArcWrapper,
    ) -> bool {
        //println!("{:?}, {:?}", current_effect, destination_effect);
        if (current_effect == destination_effect) {
            return true;
        }

        // iterate through each edge
        for i in 0..current_effect.0.input_count() {
            let next_effect = current_effect.0.get_input_at_index(i);

            match next_effect {
                Err(_) => continue,
                Ok(next_effect) => {
                    if self.dfs_upstream_path(&ArcWrapper(next_effect), destination_effect) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Returns true iff adding an edge with this input and output will not make a cycle (ie the graph stays a dag)
    /// Note that this function ASSUMES that it was a dag to begin with, ie the new edge is a Necessary part of the path of traversal.
    fn check_will_form_dag(
        &self,
        input: NodeCircleIdentifier,
        output: NodeCircleIdentifier,
    ) -> bool {
        // method is simple, begin with the input node, then depth first search along its inputs until we either end or fail

        // is there a path connecting the two points
        let b = self.dfs_upstream_path(
            &ArcWrapper(self.nodes[output.node_index].effect.clone()),
            &ArcWrapper(self.nodes[input.node_index].effect.clone()),
        );

        // return opposite
        !b
    }

    fn get_node_index_from_effect(&self, effect: Arc<dyn Effect>) -> Option<&usize> {
        self.hash.get(&ArcWrapper(effect))
    }

    pub fn node_graph_ui(&mut self, ui: &mut eframe::egui::Ui) -> Response {
        // let mut area = area.begin(ctx);

        // do node ui and find if we need a new edge
        let mut r = None;
        let mut i = None;
        for j in 0..self.nodes.len() {
            if j == 0 {
                continue;
            }

            let node = &mut self.nodes[j];
            let inner_resp = node.node_ui(ui, &self.style, &self.audio_data);

            r = Some(inner_resp.response);

            i = i.or(inner_resp.inner);
        }

        match i {
            None => (),
            Some((input, output)) => {
                // must check if (a) this edge does not already exist
                // and (b) this edge does not cause the dag to stop being a dag
                if !self.check_edge_already_exists(*input, *output) {
                    if self.check_will_form_dag(*input, *output) {
                        // Now we want to set the input at appropriate index on the node effect

                        self.add_edge(*input, *output);
                        println!("{:?}, {:?}", input, output);
                    } else {
                        println!("NO EDGE AS DAG CONDITION FAILED")
                    }
                }
            }
        }

        // do edge ui

        // This is kinda a little hack to force the edges to exist in front of the nodes (I mean it makes sense tbf)
        let ui = &mut ui.new_child(UiBuilder {
            id_salt: None,
            ui_stack_info: UiStackInfo::default(),
            layer_id: Some(LayerId::new(Order::Foreground, Id::new("UHH I WANT EDGES"))),
            max_rect: None,
            layout: None,
            disabled: false,
            invisible: false,
            sizing_pass: false,
            style: None,
            sense: None,
        });

        // Draw the edges by iterating through the data in each node
        for node in &self.nodes {
            // Get the edge data
            for i in 0..node.effect.input_count() {
                let start_effect = node.effect.get_input_at_index(i).unwrap();
                let start_index = self.get_node_index_from_effect(start_effect);

                match start_index {
                    None | Some(0) => (),
                    Some(start_index) => {
                        let edge = Edge::new(
                            NodeCircleIdentifier {
                                node_index: node.index,
                                circle_index: i,
                                circle_is_input: true,
                            },
                            NodeCircleIdentifier {
                                node_index: *start_index,
                                circle_index: 0,
                                circle_is_input: false,
                            },
                        );

                        edge.draw_edge(ui, &self.style, self);
                    }
                }
            }
        }
        // Draw the edges
        // for edge in &self.edges {
        //     edge.draw_edge(m, &self.style, self);
        // }

        r.unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // #[test]
    // fn test_graph_works() {
    //     let graph = NodeGraph::new_empty();
    // }

    // #[test]
    // fn test_dfs_condition() {
    //     let graph = NodeGraph::new(
    //         vec![
    //             Node::new(0, 1, 1, 6.0),
    //             Node::new(1, 1, 1, 6.0),
    //             Node::new(2, 1, 1, 6.0),
    //         ],
    //         vec![
    //             Edge::new(
    //                 NodeCircleIdentifier::new(0, 0, true),
    //                 NodeCircleIdentifier::new(1, 0, false),
    //             ),
    //             Edge::new(
    //                 NodeCircleIdentifier::new(1, 0, true),
    //                 NodeCircleIdentifier::new(2, 0, false),
    //             ),
    //         ],
    //         None,
    //     );
    // }
}
