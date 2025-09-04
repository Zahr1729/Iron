use std::{hash::Hash, sync::Arc};

use eframe::{
    egui::{
        self, Color32, DragAndDrop, Grid, Id, InnerResponse, Label, LayerId, Order, Pos2, Rect,
        Response, RichText, Sense, Shape, Stroke, Ui, UiBuilder, UiStackInfo, Vec2, ahash::HashMap,
    },
    epaint::{CircleShape, CubicBezierShape, PathStroke},
};

use crate::{
    audio::effects::{
        Effect, add::Add, gain::Gain, output::Output, sinewave::SineWave, zero::Zero,
    },
    common::{dB, track::Track},
    ui::nodegraph::{edge::Edge, node::Node, nodecircle::NodeCircleIdentifier},
};

mod edge;
mod node;
mod nodecircle;

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
    drag_colour: Color32,

    connected_colour: Color32,
    disconnected_colour: Color32,
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

            edge_inner_colour: Color32::RED,
            edge_outer_colour: Color32::from_rgb(200, 200, 200),
            drag_colour: Color32::from_rgb(194, 136, 11),
            // rgba(186, 79, 13, 1)
            connected_colour: Color32::from_rgb(13, 116, 15), // Green
            disconnected_colour: Color32::from_rgb(186, 79, 13),
            line_colour: Color32::from_rgb(200, 200, 200),

            corner_radius: 8.0,
            margin: 10.0,
            plot_margin: 0.0,

            plot_height: 75.0,
            plot_width: 150.0,

            header_height: 48.0,
            header_text_size: 20.0,

            header_colour: Color32::from_rgb(50, 50, 50),
            header_text_colour: Color32::from_rgb(220, 220, 220),

            main_text_size: 16.0,
            grid_row_height: 16.0,

            main_colour: Color32::DARK_GRAY,
            main_text_colour: Color32::from_rgb(200, 200, 200),
        }
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
        let s2 = Arc::new(SineWave::new(0.5, 480.0, 0.0));
        let a1 = Arc::new(Add::new(s.zero.clone(), s.zero.clone()));

        s.add_node(g1);
        s.add_node(g2);
        s.add_node(s1);
        s.add_node(s2);
        s.add_node(a1);

        s.set_node_connection_status();
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
        let e = self.nodes[input.node_index].effect().clone();
        let _ = e.set_input_at_index(
            input.circle_index,
            self.nodes[output.node_index].effect().clone(),
        );
        //self.edges.push(Edge::new(input, output));

        // Now just recalculate the connected
        self.set_node_connection_status();
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
        if current_effect == destination_effect {
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
            &ArcWrapper(self.nodes[output.node_index].effect()),
            &ArcWrapper(self.nodes[input.node_index].effect()),
        );

        // return opposite
        !b
    }

    /// DFS turns is_connected to true and then does the same for all children
    fn iterate_node_connection_status(&mut self, index: usize) {
        //println!("{_index}");
        self.nodes[index].set_is_connected_to_output(true);

        for i in 0..self.nodes[index].effect().input_count() {
            let child = self.nodes[index].effect().get_input_at_index(i).unwrap();

            // use the hash map to get the next index
            let wrapper = ArcWrapper(child);
            let child_index = self.hash.get(&wrapper).unwrap_or(&0);
            self.iterate_node_connection_status(*child_index);
        }
    }

    /// Sets every node is_connected to if it is upstream of output
    fn set_node_connection_status(&mut self) {
        // Reset so all disconnected
        for nodes in &mut self.nodes {
            nodes.set_is_connected_to_output(false);
        }

        // Start from output (ie index zero and iterate)
        self.iterate_node_connection_status(1);
    }

    fn get_node_index_from_effect(&self, effect: Arc<dyn Effect>) -> Option<&usize> {
        self.hash.get(&ArcWrapper(effect))
    }

    pub fn node_graph_ui(&mut self, ui: &mut eframe::egui::Ui) -> Response {
        let scope = tracing::trace_span!("node_graph_ui");
        let _span = scope.enter();

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

        match i.clone() {
            None => (),
            Some(j) => println!("{j:?}"),
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
            for i in 0..node.effect().input_count() {
                let start_effect = node.effect().get_input_at_index(i).unwrap();
                let start_index = self.get_node_index_from_effect(start_effect);

                match start_index {
                    None | Some(0) => (),
                    Some(start_index) => {
                        let edge = Edge::new(
                            NodeCircleIdentifier {
                                node_index: node.index(),
                                circle_index: i,
                                circle_is_input: true,
                            },
                            NodeCircleIdentifier {
                                node_index: *start_index,
                                circle_index: 0,
                                circle_is_input: false,
                            },
                        );

                        let colour = match node.is_connected_to_output() {
                            true => self.style.connected_colour,
                            false => self.style.disconnected_colour,
                        };

                        edge.draw_edge(ui, &self.style, self, colour);
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
    //use super::*;

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
