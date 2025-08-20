use eframe::{
    egui::{
        self, Area, Color32, Context, DragAndDrop, DragValue, Frame, Grid, Id, Label, LayerId,
        Margin, Pos2, Rect, Response, RichText, Sense, Shape, Slider, Stroke, StrokeKind, Ui, Vec2,
        Widget, Window, frame, output, style::default_text_styles,
    },
    epaint::{CircleShape, CubicBezierShape, PathStroke, RectShape},
};
use egui_plot::Plot;

use crate::main;

#[derive(Clone, Copy)]
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

fn draw_bezier(ui: &mut Ui, start: Pos2, end: Pos2, width: f32, colour: Color32) {
    fn get_cubic_bezier_coords(a: Pos2, b: Pos2) -> [Pos2; 4] {
        [a, Pos2 { x: b.x, y: a.y }, Pos2 { x: a.x, y: b.y }, b]
    }

    let points = get_cubic_bezier_coords(start, end);

    let bezier = CubicBezierShape {
        points,
        closed: false,
        fill: Color32::TRANSPARENT,
        stroke: PathStroke::new(width, colour),
    };

    //ui.painter().add(top_bezier);
    ui.painter().add(bezier);
}

impl Widget for NodeCircle {
    fn ui(self, ui: &mut Ui) -> Response {
        // First do logic and draw bezier underneath

        let rect = get_generic_circle_rect(ui, self.pos, self.radius);

        let r = ui.allocate_rect(rect, Sense::all());

        if self.is_input
            && DragAndDrop::has_payload_of_type::<&'static str>(ui.ctx())
            && r.contains_pointer()
        {
            let payload = DragAndDrop::payload::<&'static str>(ui.ctx()).unwrap();

            println!("Hovering over input while dragging {:?}", payload);
        }

        //println!("{:?}", r);
        if !self.is_input && r.dragged() {
            // Draw line segment
            DragAndDrop::set_payload(ui.ctx(), "poopoo");

            match r.hover_pos() {
                Some(mouse_pos) => {
                    // Draw the external boundary bezier
                    draw_bezier(
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
                    draw_bezier(ui, self.pos, mouse_pos, self.radius, self.circle_colour);
                }
                _ => (),
            }

            return r; // So we don't draw circle again
        }

        // Then draw circle first
        draw_circle(
            ui,
            self.pos,
            self.radius,
            self.line_width,
            self.circle_colour,
            self.edge_colour,
        );

        r
    }
}

#[derive(Clone)]
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

    /// This function both draws and does the logic for each circle
    fn implement_circles(mut self, ui: &mut Ui, top_left: Pos2) {
        // Iterate through all the inputs first then outputs
        for i in 0..(self.input_count + self.output_count) {
            // Want to get rect that is just offset from this so its on the edge

            // get a bool for if we are dealing with inputs
            let is_input = i < self.input_count;
            let y_pos = match is_input {
                true => i,
                false => i - self.input_count,
            };

            let pos = self.get_interactable_circle_centres(ui, y_pos, top_left, is_input);

            // give position data to our node_circles

            match is_input {
                true => {
                    self.input_node_circles[y_pos].pos = pos;
                    ui.add(self.input_node_circles[y_pos]);
                }
                false => {
                    self.output_node_circles[y_pos].pos = pos;
                    ui.add(self.output_node_circles[y_pos]);
                }
            }
        }
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
}

impl Widget for Node {
    fn ui(mut self, ui: &mut Ui) -> Response {
        egui::Area::new(egui::Id::new(format!("graph_node {}", self.index)))
            .show(ui.ctx(), |ui| {
                let top_left = ui.next_widget_position();

                // Draw the basic node
                self.draw_node_without_circles(ui);

                // Do the stuff with the selectible nodes
                self.implement_circles(ui, top_left);
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
