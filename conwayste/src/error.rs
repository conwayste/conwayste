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

custom_error! {pub ConwaysteError
    InvalidUIAction {reason: String} = "ConwaysteError::InvalidUIAction(reason={reason})",
    NoAssociatedUIAction{reason: String} = "ConwaysteError::NoAssociatedUIAction(reason={reason})"
}

pub type ConwaysteResult<T> = ::std::result::Result<T, ConwaysteError>;

/*

impl PartialEq for ConwaysteError {
    fn eq(&self, other: &ConwaysteError) -> bool {
        use ConwaysteError::*;
        match *self {
            InvalidUIAction{reason: ref self_reason} => {
                if let InvalidUIAction{reason: ref other_reason} = *other {
                    self_reason == other_reason
                } else {
                    false
                }
            }
            NoAssociatedUIAction{reason: ref self_reason} => {
                if let NoAssociatedUIAction{reason: ref other_reason} = *other {
                    self_reason == other_reason
                } else {
                    false
                }
            }
        }
    }
}
*/
