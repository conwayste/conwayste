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

custom_error! {pub UIError
    InvalidDimensions {reason: String} = "UIError::InvalidDimensions({reason})",
    WidgetNotFound {reason: String} = "UIError::WidgetNotFound({reason})",
    InvalidAction {reason: String} = "UIError::InvalidAction({reason})",
    ActionRestricted{reason: String} = "UIError::ActionRestricted({reason})"
}

// TODO: use Box<dyn Error> and make this a generic CwResult (project-wide)
pub type UIResult<T> = Result<T, Box<UIError>>;
