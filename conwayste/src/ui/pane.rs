/*  Copyright 2019 the Conwayste Developers.
 *
 *  This file is part of conwayste.
 *
 *  conwayste is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  conwayste is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with conwayste.  If not, see
 *  <http://www.gnu.org/licenses/>. */

use ggez::graphics::{self, Color, Rect, DrawMode, DrawParam};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

use super::{
    widget::Widget,
    common::{within_widget},
    UIAction,
    UIError, UIResult,
    WidgetID
};

use crate::constants::colors::*;

pub struct Pane {
    pub id: WidgetID,
    pub dimensions: Rect,
    pub widgets: Vec<Box<dyn Widget>>,
    pub hover: bool,
    pub floating: bool, // can the window be dragged around?
    pub previous_pos: Option<Point2<f32>>,
    pub border: f32,
    pub bg_color: Option<Color>,

    // might need something to track mouse state to see if we are still clicked within the
    // boundaries of the pane in the dragging case
}

/// A container of one or more widgets
impl Pane {
    /// Specify the unique widget identifer for the pane, and its dimensional bounds
    pub fn new(widget_id: WidgetID, dimensions: Rect) -> Self {
        Pane {
            id: widget_id,
            dimensions: dimensions,
            widgets: vec![],
            hover: false,
            floating: true,
            previous_pos: None,
            border: 1.0,
            bg_color: None,
        }
    }

    /// Add a widget to the pane. The added widget's x and y coordinates will be translated by the
    /// x and y coordinates of its new parent. For example, if a widget is at (10,10) and it is
    /// added to a pane at (200,300), the widget will now be at (210, 310).
    ///
    /// # Errors
    ///
    /// It is a `UIError::InvalidDimensions` error if, after being translated as described above,
    /// the widget's box does not fit within the box of its parent.
    pub fn add(&mut self, mut widget: Box<dyn Widget>) -> UIResult<()> {
        let mut dims = widget.size();
        // Widget-to-be-added's coordinates are with respect to the Pane's origin
        dims.translate(self.dimensions.point());

        if dims.w > self.dimensions.w || dims.h > self.dimensions.h {
            return Err(Box::new(UIError::InvalidDimensions{
                reason: format!("Widget of {:?} is larger than Pane of {:?}", widget.id(), self.id)
            }));
        }

        if dims.right() > self.dimensions.right()
        || dims.left() < self.dimensions.left()
        || dims.top() < self.dimensions.top()
        || dims.bottom() > self.dimensions.bottom() {
            println!("{:?} Dims: {:?}", widget.id(), dims);
            println!("Pane: {:?}", self.dimensions);
            return Err(Box::new(UIError::InvalidDimensions{
                reason: format!("Widget of {:?} is not fully enclosed by Pane of {:?}", widget.id(), self.id)
            }));
        }

        widget.set_size(dims)?;
        self.widgets.push(widget);
        Ok(())
    }

    /*
    // TODO: Currently used to reset previous position on mouse release after dragging completes.
    //      Re-evaluate design if this is the best way to do it. See issue #71 (dragging).
    pub fn update(&mut self, is_mouse_released: bool) {
        if is_mouse_released {
            self.previous_pos = None;
        }
    }
    */

    /// Adds the vector of children to the parent, and shrink the parent to fit the widgets with
    /// padding on all sides. The widgets will have the padding added to their x and y coordinates.
    /// Therefore it is suggested that the upper left widget in `children` be at `(0, 0)`.
    ///
    /// # Errors
    ///
    /// Any errors from adding are passed down. NOTE: parent will have already been resized!
    //TODO if a `Container` trait is added, this can be a method of that trait instead. This would
    // allow things other than Pane to contain child Widgets.
    pub fn add_and_shrink_to_fit(&mut self, children: Vec<Box<dyn Widget>>, padding: f32) -> UIResult<()> {
        // find bounding box
        let (mut width, mut height) = (0.0, 0.0);
        for child in &children {
            let child_rect = child.size();
            let w = child_rect.x + child_rect.w;
            let h = child_rect.y + child_rect.h;
            if w > width {
                width = w;
            }
            if h > height {
                height = h;
            }
        }

        // resize parent (use padding)
        let mut dimensions = self.size();
        dimensions.w = width + 2.0 * padding;
        dimensions.h = height + 2.0 * padding;
        self.set_size(dimensions)?;

        for mut child in children {
            // add padding to each child
            let mut dimensions = child.size();
            dimensions.x += padding;
            dimensions.y += padding;
            child.set_size(dimensions)?;

            self.add(child)?;
        }
        Ok(())
    }

}

impl Widget for Pane {
    fn id(&self) -> WidgetID {
        self.id
    }

    fn size(&self) -> Rect {
        self.dimensions
    }

    fn set_size(&mut self, new_dims: Rect) -> UIResult<()> {
        if new_dims.w == 0.0 || new_dims.h == 0.0 {
            return Err(Box::new(UIError::InvalidDimensions{
                reason: "Cannot set the size to a width or height of zero".to_owned()
            }));
        }

        self.dimensions = new_dims;
        Ok(())
    }

    fn translate(&mut self, dest: Vector2<f32>)
    {
        self.dimensions.translate(dest);
    }

    fn on_hover(&mut self, point: &Point2<f32>) {
        self.hover = within_widget(point, &self.dimensions);
        for w in self.widgets.iter_mut() {
            w.on_hover(point);
        }
    }

    fn on_click(&mut self, point: &Point2<f32>) -> Option<(WidgetID, UIAction)> {
        let hover = self.hover;
        self.hover = false;

        if hover {
            for w in self.widgets.iter_mut() {
                let ui_action = w.on_click(point);
                if ui_action.is_some() {
                    return ui_action;
                }
            }
        }
        None
    }


    /* TODO: fix all the drag issues
    /// original_pos is the mouse position at which the button was held before any dragging occurred
    /// current_pos is the latest mouse position after any movement
    fn on_drag(&mut self, original_pos: &Point2<f32>, current_pos: &Point2<f32>) {

        if !self.floating || !self.hover {
            return;
        }

        let mut drag_ok = true;

        // Check that the mouse down event is bounded by the pane but not by a sub-widget
        if within_widget(original_pos, &self.dimensions) {
            for widget in self.widgets.iter() {
                if within_widget(original_pos, &widget.size()) && self.previous_pos.is_none() {
                    drag_ok = false;
                    break;
                }
            }
        } else {
            // The original mouse down event may be no longer bounded if the pane moved enough,
            // so check if we were dragging at a previous spot
            drag_ok = self.previous_pos.is_some();
        }

        if drag_ok {
            // Note where the pane was previously to calculate the delta in position
            if let None = self.previous_pos {
                self.previous_pos = Some(*current_pos);
            }

            if let Some(origin) = self.previous_pos {
                let tl_corner_offset = current_pos - origin;

                if tl_corner_offset[0] != 0.0 && tl_corner_offset[1] != 0.0 {
                    //println!("Dragging! {}, {}, {}", origin, current_pos, tl_corner_offset);
                }

                self.translate(tl_corner_offset);
                for ref mut widget in self.widgets.iter_mut() {
                    widget.translate(tl_corner_offset);
                }
            }

            self.previous_pos = Some(*current_pos);
        }
    }
    */

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        if let Some(bg_color) = self.bg_color {
            let mesh = graphics::Mesh::new_rectangle(ctx, DrawMode::fill(), self.dimensions, bg_color)?;
            graphics::draw(ctx, &mesh, DrawParam::default())?;
        }

        if self.border > 0.0 {
            let mesh = graphics::Mesh::new_rectangle(ctx, DrawMode::stroke(1.0), self.dimensions, *PANE_BORDER_COLOR)?;
            graphics::draw(ctx, &mesh, DrawParam::default())?;
        }

        for widget in self.widgets.iter_mut() {
            widget.draw(ctx)?;
        }

        Ok(())
    }
}

widget_from_id!(Pane);

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{chatbox::Chatbox, common::FontInfo, textfield::TextField};
    use ggez::graphics::Scale;

    fn create_dummy_pane(size: f32) -> Pane {
        Pane::new(WidgetID(0), Rect::new(0.0, 0.0, size, size))
    }

    fn create_dummy_font() -> FontInfo {
        FontInfo {
            font: (), //dummy font because we can't create a real Font without ggez
            scale: Scale::uniform(1.0), // Does not matter
            char_dimensions: Vector2::<f32>::new(5.0, 5.0),  // any positive values will do
        }
    }

    fn create_dummy_textfield(rect: Rect) -> TextField {
        let font_info = create_dummy_font();
        TextField::new(WidgetID(0), font_info, rect)
    }

    #[test]
    fn test_add_widget_to_pane_basic() {
        let mut pane = create_dummy_pane(1000.0);
        let font_info = create_dummy_font();
        let history_len = 5;
        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);

        assert!(pane.add(Box::new(chatbox)).is_ok());

        for (i, w) in pane.widgets.iter().enumerate() {
            assert_eq!(i, 0);
            assert_eq!(w.id(), WidgetID(0));
        }
    }

    #[test]
    fn test_add_larger_widget_to_smaller_pane() {
        let mut pane = create_dummy_pane(10.0);
        let font_info = create_dummy_font();
        let history_len = 5;
        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);

        assert!(pane.add(Box::new(chatbox)).is_err());
    }

    #[test]
    fn test_add_overflowing_widget_to_pane_exceeds_right_boundary() {
        // Exceeds right-hand boundary
        let mut pane = create_dummy_pane(10.0);
        let rect = Rect::new(0.0, 0.0, 20.0, 9.0);
        let textfield = create_dummy_textfield(rect);

        assert!(pane.add(Box::new(textfield)).is_err());
    }


    #[test]
    fn test_add_overflowing_widget_to_pane_exceeds_left_boundary() {
        // Exceeds right-hand boundary
        let mut pane = create_dummy_pane(10.0);
        let rect = Rect::new(-10.0, 0.0, 9.0, 9.0);
        let textfield = create_dummy_textfield(rect);

        assert!(pane.add(Box::new(textfield)).is_err());
    }

    #[test]
    fn test_add_overflowing_widget_to_pane_exceeds_top_boundary() {
        // Exceeds right-hand boundary
        let mut pane = create_dummy_pane(10.0);
        let rect = Rect::new(0.0, -10.0, 9.0, 9.0);
        let textfield = create_dummy_textfield(rect);

        assert!(pane.add(Box::new(textfield)).is_err());
    }

    #[test]
    fn test_add_overflowing_widget_to_pane_exceeds_bottom_boundary() {
        // Exceeds right-hand boundary
        let mut pane = create_dummy_pane(10.0);
        let rect = Rect::new(0.0, 0.0, 9.0, 20.0);
        let textfield = create_dummy_textfield(rect);

        assert!(pane.add(Box::new(textfield)).is_err());
    }

    #[test]
    #[ignore]
    fn test_add_widgets_with_the_same_id_to_pane() {
        let mut pane = create_dummy_pane(10.0);
        let font_info = create_dummy_font();
        let history_len = 5;
        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);
        assert!(pane.add(Box::new(chatbox)).is_ok());

        // TODO: This should return an Error since the Widget ID's collide
        let chatbox = Chatbox::new(WidgetID(0), font_info, history_len);
        assert!(pane.add(Box::new(chatbox)).is_err());
    }
}
