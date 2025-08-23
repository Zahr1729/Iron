use std::sync::Arc;

use eframe::{
    egui::{Color32, DragAndDrop, InnerResponse, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2},
    epaint::CircleShape,
};

use crate::ui::nodegraph::{GraphStyle, edge::Edge};

/// This is to collect data for the edge drag and drop to connect things up
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct NodeCircleIdentifier {
    pub node_index: usize,
    pub circle_index: usize,
    pub circle_is_input: bool,
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
    pub index: usize,
    pub is_input: bool,
    pub pos: Pos2,
    pub radius: f32,
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

fn get_generic_circle_rect(_ui: &mut Ui, pos: Pos2, radius: f32) -> Rect {
    let radius_offset = Vec2 {
        x: radius,
        y: radius,
    };

    Rect::from_two_pos(pos - radius_offset, pos + radius_offset)
}

impl NodeCircle {
    /// This function's inner response contains identifying information for the node that began the drag and drop when it is released
    pub fn node_circle_ui(
        self,
        ui: &mut Ui,
        style: &GraphStyle,
        parent_index: usize,
        is_connected: bool,
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
                        style.drag_colour,
                        style.line_colour,
                    );

                    // draw mouse circle after the bezier
                    draw_circle(
                        ui,
                        mouse_pos,
                        self.radius,
                        style.edge_line_width,
                        style.drag_colour,
                        style.line_colour,
                    );

                    // Draw the internal bezier so it looks nice
                    Edge::draw_bezier(ui, self.pos, mouse_pos, self.radius, style.drag_colour);
                }
                _ => (),
            }

            // So we don't draw circle again
        } else {
            let colour = match is_connected {
                true => style.connected_colour,
                false => style.disconnected_colour,
            };

            // Just draw circle
            draw_circle(
                ui,
                self.pos,
                self.radius,
                style.node_line_width,
                colour,
                style.line_colour,
            );
        }

        let return_value;

        let payload = r.dnd_release_payload::<NodeCircleIdentifier>();
        return_value = InnerResponse::new(payload, r);

        return_value
    }

    pub fn draw(
        &self,
        ui: &mut Ui,
        line_width: f32,
        circle_colour: Color32,
        boundary_colour: Color32,
    ) {
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
