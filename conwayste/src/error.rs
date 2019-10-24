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

/// A macro inspired by Go's Type Switch
///
/// [See the Go documentation for additional details.](https://tour.golang.org/methods/16])
///
/// It accepts a `Result<T, E>` where `E` must be `dyn Error`. The macro attempts to downcast the result
/// into one or more errors as specified in the "match" expressions. If no downcasts are successful,
/// then the macro simply passes along the original result without modification.
///
/// # Usage
///     handle_error!(input_result, [custom_return_type, ]
///         [Error(_) => { ...},]*      // matcher
///     );
///
/// # Arguments
///     input_result - a result type
///     custom_return_type - (optional) specify a type T for Ok(T); Default type is ()
///     matcher - (optional) an error which implements Error
///
/// # Examples
/// ```rust
/// // See the Error documentation page for the SuperError and SuperSideKickError definitions
///
/// fn give_me_an_error_result() -> Result<(), Box<dyn Error>> {
///     Err(Box::new(SuperError{}))
/// }
///
/// let a_result = give_me_an_error_result();
/// handle_error!(a_result,
///     SuperError(e) => {
///         println!("I got a SuperError after the downcast");
///     },
///     SuperSideKickError(e) => {
///         println!("I got a SuperSideKickError after the downcast!");
///     },
/// );
/// ```
///
#[macro_export]
macro_rules! handle_error {
   ($input:ident $(, $matcher:ident ($var:ident) => $result:expr)*) => {
        $(
            let $input = $input.or_else(|boxed_error| -> Result<(), Box<dyn Error>> {
            boxed_error.downcast::<$matcher>()
                .and_then(|$var| $result)
            });
        )*
   };

   ($input:ident, $type:ty $(, $matcher:ident ($var:ident) => $result:expr)*) => {
        $(
            let $input = $input.or_else(|boxed_error| -> Result<$type, Box<dyn Error>> {
            boxed_error.downcast::<$matcher>()
                .and_then(|$var| $result)
            });
        )*
   };
}
