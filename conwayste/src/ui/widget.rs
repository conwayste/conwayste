
use ggez::{Context, GameResult};
use ggez::graphics::{Font, Point2};

pub trait Widget<T> {
    fn on_hover(&mut self, point: &Point2);
    fn on_click(&mut self, point: &Point2, t: &mut T);
    fn draw(&self, ctx: &mut Context, font: &Font) -> GameResult<()>;
}