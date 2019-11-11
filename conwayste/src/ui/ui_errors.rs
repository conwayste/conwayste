/*  Copyright 2019 the Conwayste Developers.
 *
 *  This file is part of Conwayste.
 *
 *  Conwayste is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Conwayste is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with libconway.  If not, see <http://www.gnu.org/licenses/>. */

use crate::ggez::GameError;

custom_error! {pub UIError
    InvalidDimensions {reason: String} = "UIError::InvalidDimensions({reason})",
    WidgetNotFound {reason: String} = "UIError::WidgetNotFound({reason})",
    InvalidAction {reason: String} = "UIError::InvalidAction({reason})",
    ActionRestricted{reason: String} = "UIError::ActionRestricted({reason})",
    WidgetIDCollision{reason: String} = "UIError::WidgetIDCollision({reason})",
    GgezError{reason: String} = "UIError::GgezError({reason})"
}

pub type UIResult<T> = Result<T, Box<UIError>>;

impl From<GameError> for UIError {
    fn from(e: GameError) -> UIError {
        GameError::from(e).into()
    }
}

impl From<GameError> for Box<UIError> {
    fn from(e: GameError) -> Box<UIError> {
        GameError::from(e).into()
    }
}

impl From<Box<UIError>> for GameError {
    fn from(e: Box<UIError>) -> GameError {
        e.into()
    }
}