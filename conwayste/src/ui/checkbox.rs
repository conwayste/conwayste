
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
use chromatica::css;

use ggez::graphics::{self, Rect, Font, Color, DrawMode, DrawParam};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

use super::{
    label::Label,
    widget::Widget,
    helpe::{within_widget, draw_text},
    UIAction,
    WidgetID,
};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ToggleState {
    Disabled,
    Enabled
}

pub struct Checkbox {
    pub id:     WidgetID,
    pub label: Label,
    pub state: ToggleState,
    pub dimensions: Rect,
    pub hover: bool,
    pub action: UIAction
}

const LABEL_OFFSET_X: f32 = 30.0;
const LABEL_OFFSET_Y: f32 = -5.0;

/// A standard checkbox widget that can be enabled or disabled via the ToggleState structure.
impl Checkbox {
    /// Creates a Checkbox widget.
    ///
    /// # Arguments
    /// * `ctx` - GGEZ context
    /// * `font` - font to be used when drawing the text
    /// * `text` - Label text
    /// * `widget_id` - Unique widget identifier
    /// * `action` - Unique action identifer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ui::Button;
    ///
    /// fn new(ctx: &mut Context) -> GameResult<MainState> {
    ///     let font = Font::default();
    ///     let checkbox = Box::new(Checkbox::new(ctx, &font,
    ///         "Toggle Me",
    ///         Rect::new(10.0, 210.0, 20.0, 20.0),
    ///         WidgetID::TestCheckbox,
    ///         UIAction::Toggle( if cfg!(target_os = "linux") { ToggleState::Enabled } else { ToggleState::Disabled } ),
    ///     ));;
    ///     checkbox.draw(ctx, font);
    /// }
    /// ```
    ///
    pub fn new(ctx: &mut Context, font: &Font, text: &'static str, dimensions: Rect, widget_id: WidgetID, action: UIAction) -> Self {
        let label_origin = Point2::new(dimensions.x + dimensions.w + LABEL_OFFSET_X, dimensions.y + dimensions.h + LABEL_OFFSET_Y);

        Checkbox {
            id: widget_id,
            label: Label::new(text, Color::from(css::WHITE), label_origin),
            state: ToggleState::Disabled,
            dimensions: dimensions,
            hover: false,
            action: action
        }
    }

    /// Toggles the checkbox from either enabled to disasbled, or vis-a-versa.
    pub fn toggle(&mut self) -> ToggleState {
        if self.state == ToggleState::Disabled {
            self.state = ToggleState::Enabled;
        } else {
            self.state = ToggleState::Disabled;
        }
        self.state
    }

}


impl Widget for Checkbox {
    fn id(&self) -> WidgetID {
        self.id
    }

    fn size(&self) -> Rect {
        self.dimensions
    }

    fn set_size(&mut self, new_dims: Rect) {
        self.dimensions = new_dims;
    }

    fn translate(&mut self, point: Vector2<f32>)
    {
        self.dimensions.translate(point);
        self.label.translate(point);
    }

    fn on_hover(&mut self, point: &Point2<f32>) {
        let label_dimensions = self.label.size();
        self.hover = within_widget(point, &self.dimensions) || within_widget(point, &label_dimensions);
        if self.hover {
            println!("Hovering over Checkbox, '{:?}'", label_dimensions);
        }
    }

    fn on_click(&mut self, _point: &Point2<f32>) -> Option<(WidgetID, UIAction)>
    {
        let hover = self.hover;
        self.hover = false;

        if hover {
            println!("Clicked Checkbox, '{}'", self.label.text);
            return Some(( self.id, UIAction::Toggle(self.toggle()) ));
        }
        None
    }

    fn draw(&mut self, ctx: &mut Context, font: &Font) -> GameResult<()> {

        let draw_mode = if self.state == ToggleState::Enabled {
            DrawMode::fill()
        } else {
            DrawMode::stroke(1.0)
        };

        if self.hover {
            // Add in a violet border/fill while hovered. Color checkbox differently to indicate  hovered state.
            let border_rect = Rect::new(self.dimensions.x-1.0, self.dimensions.y-1.0, self.dimensions.w + 4.0, self.dimensions.h + 4.0);
            let hovered_border = graphics::Mesh::new_rectangle(ctx, DrawMode::stroke(2.0), border_rect, Color::from(css::VIOLET))?;
            graphics::draw(ctx, &hovered_border, DrawParam::default())?;
        }

        let border = graphics::Mesh::new_rectangle(ctx, DrawMode::stroke(2.0), self.dimensions, self.label.color)?;
        graphics::draw(ctx, &border, DrawParam::default())?;
        let label_border = graphics::Mesh::new_rectangle(ctx, DrawMode::stroke(2.0), self.dimensions, self.label.color)?;
        graphics::draw(ctx, &label_border, DrawParam::default())?;

        //draw_text(ctx, font, self.label.color, &self.label.text, &self.label.dimensions.point().into(), None)?;
        self.label.draw(ctx, font)?;

        Ok(())
    }
}
