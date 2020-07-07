/*  Copyright 2019 the Conwayste Developers.
 *
 *  This file is part of libconway.
 *
 *  libconway is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  libconway is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with libconway.  If not, see <http://www.gnu.org/licenses/>. */

custom_error! {pub ConwayError
    InvalidData {reason: String} = "ConwayError->InvalidData->{reason}",
    AccessDenied{reason: String} = "ConwayError->AccessDenied->{reason}"
}

pub type ConwayResult<T> = ::std::result::Result<T, ConwayError>;

impl PartialEq for ConwayError {
    fn eq(&self, other: &ConwayError) -> bool {
        use ConwayError::*;
        match *self {
            InvalidData {
                reason: ref self_reason,
            } => {
                if let InvalidData {
                    reason: ref other_reason,
                } = *other
                {
                    self_reason == other_reason
                } else {
                    false
                }
            }
            AccessDenied {
                reason: ref self_reason,
            } => {
                if let AccessDenied {
                    reason: ref other_reason,
                } = *other
                {
                    self_reason == other_reason
                } else {
                    false
                }
            }
        }
    }
}
