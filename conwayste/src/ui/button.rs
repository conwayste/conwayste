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

use ggez::graphics::{self, Rect, Font, Color, DrawMode, DrawParam, Text, Scale};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};

use super::{
    label::Label,
    widget::Widget,
    helpe::{within_widget, draw_text, color_with_alpha},
    UIAction, WidgetID
};

pub struct Button {
    pub id:     WidgetID,
    pub label: Label,
    pub button_color: Color,
    pub draw_mode: DrawMode,
    pub dimensions: Rect,
    pub hover: bool,
    pub borderless: bool,
    pub action: UIAction
}

/// A named widget that can be clicked to result in an occuring action.
impl Button {

    /// Creates a Button widget.
    ///
    /// # Arguments
    /// * `ctx` - GGEZ context
    /// * `font` - font to be used when drawing the text
    /// * `button_text` - Text to be displayed
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
    ///     let b = Button::new(ctx, font, "TestButton", WidgetID::TestButton1, UIAction::PrintHelloWorld)
    ///         .label_color(Color::from(css::DARKCYAN))
    ///         .button_color(Color::from(css::WHITE));
    ///
    ///     b.draw(ctx, font)?;
    /// }
    /// ```
    ///
    pub fn new(ctx: &mut Context, font: &Font, button_text: &'static str, widget_id: WidgetID, action: UIAction) -> Self {
        const OFFSET_X: f32 = 8.0;
        const OFFSET_Y: f32 = 4.0;

        let mut text = Text::new(button_text);
        let text = text.set_font(*font, Scale::uniform(10.0));
        let width = text.width(ctx) as f32 + OFFSET_X*2.0;
        let height = text.height(ctx) as f32 + OFFSET_Y*2.0;
        let dimensions = Rect::new(30.0, 20.0, width, height);
        let offset = Point2::new(dimensions.x + OFFSET_X, OFFSET_Y);

        Button {
            id: widget_id,
            label: Label::new(ctx, font, button_text, color_with_alpha(css::WHITE, 0.1), offset),
            button_color: color_with_alpha(css::DARKCYAN, 0.8),
            draw_mode: DrawMode::fill(),
            dimensions: dimensions,
            hover: false,
            borderless: false,
            action: action,
        }
    }

    /// Sets the color of the Button's text to the specified ggez `Color`
    pub fn label_color(mut self, color: Color) -> Self {
        self.label = self.label.set_color(color);
        self
    }

    /// Sets the color of the button to the specified ggez `Color`
    pub fn button_color(mut self, color: Color) -> Self {
        self.button_color = color;
        self
    }
}

impl Widget for Button {
    fn id(&self) -> WidgetID {
        self.id
    }

    fn on_hover(&mut self, point: &Point2<f32>) {
        self.hover = within_widget(point, &self.dimensions);
        // if self.hover {
        //     println!("Hovering over Button, \"{}\" {:?}", self.label.text, point);
        // }
    }

    fn on_click(&mut self, _point: &Point2<f32>) -> Option<(WidgetID, UIAction)>
    {
        let hover = self.hover;
        self.hover = false;

        if hover {
            println!("Clicked Button, \"{}\"", self.label.text);
            return Some((self.id, self.action));
        }
        None
    }

    fn draw(&mut self, ctx: &mut Context, font: &Font) -> GameResult<()> {
        let draw_mode = if self.hover {
            DrawMode::fill()
        } else {
            DrawMode::stroke(2.0)
        };

        let button = graphics::Mesh::new_rectangle(ctx, draw_mode, self.dimensions, self.button_color)?;
        graphics::draw(ctx, &button, DrawParam::default())?;
        draw_text(ctx, font, self.label.color, &self.label.text, &self.dimensions.point().into(), None)?;

        Ok(())
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
    }
}
