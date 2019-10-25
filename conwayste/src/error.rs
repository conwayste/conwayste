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
/// It accepts a `Result<T, E>` where `E` must be `dyn Error`. The macro attempts to downcast the
/// result into one or more concrete error types as specified in the "match" expressions. If no
/// downcasts are successful, then the macro simply passes along the original result without
/// modification.
///
/// # Usage
///     handle_error!(input_result, [custom_return_type, ]
///         ErrorType1 => |err_var| { ...}      // matcher 1
///         ErrorType2 => |err_var| { ...}      // matcher 2
///         ...
///     );
///
/// # Arguments
///     input_result - a result type
///     custom_return_type - (optional) specify a type T for Ok(T); Default type is ()
///     ErrorType - a concrete error type which implements Error trait
///     err_var - a variable which will contain the boxed error downcast into a Box<ErrorType>
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
///     SuperError => |e| {
///         // e is a SuperError concrete error type
///         println!("I got a SuperError after the downcast");
///         Ok(())
///     },
///     SuperSideKickError => |e| {
///         // e is a SuperSideKickError concrete error type
///         println!("I got a SuperSideKickError after the downcast!");
///         Ok(())
///     },
/// );
/// ```
///
#[macro_export]
macro_rules! handle_error {
   ($input:ident $(, $matcher:ty => |$var:ident| $result:expr)*) => {
        $(
            let $input = $input.or_else(|boxed_error| -> Result<(), Box<dyn Error>> {
                boxed_error.downcast::<$matcher>()
                    .and_then(|$var| $result)
            });
        )*
        let _ = $input; // suppress warning about unused variable $input
   };

   ($input:ident, $type:ty $(, $matcher:ty => |$var:ident| $result:expr)*) => {
        $(
            let $input = $input.or_else(|boxed_error| -> Result<$type, Box<dyn Error>> {
                boxed_error.downcast::<$matcher>()
                    .and_then(|$var| $result)
            });
        )*
        let _ = $input; // suppress warning about unused variable $input
   };
}


#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::io; // io::Error is used as a placeholder for an unknown error
    use std::fmt;

    #[derive(Debug)]
    struct SuperError{
        x: i64
    }
    impl Error for SuperError {}
    impl fmt::Display for SuperError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    #[derive(Debug)]
    struct SuperSideKickError;
    impl Error for SuperSideKickError {}
    impl fmt::Display for SuperSideKickError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    fn give_me_a_super_result() -> Result<(), Box<dyn Error>> {
        Err(Box::new(SuperError{x: 3}))
    }

    fn give_me_a_sidekick_result() -> Result<(), Box<dyn Error>> {
        Err(Box::new(SuperSideKickError))
    }

    fn give_me_an_unexpected_result() -> Result<(), Box<dyn Error>> {
        Err(Box::new(io::Error::from_raw_os_error(22)))
    }

    #[test]
    fn test_handle_error_super() {
        let a_result = give_me_a_super_result();
        let mut super_x = None;
        let mut sidekick_arm_executed = false;
        handle_error!(a_result,
            SuperError => |e| {
                super_x = Some((*e).x);
                Ok(())
            },
            SuperSideKickError => |_e| {
                sidekick_arm_executed = true;
                Ok(())
            }
        );
        assert_eq!(super_x, Some(3));
        assert_eq!(sidekick_arm_executed, false);
    }

    #[test]
    fn test_handle_error_sidekick() {
        let a_result = give_me_a_sidekick_result();
        let mut super_x = None;
        let mut sidekick_arm_executed = false;
        handle_error!(a_result,
            SuperError => |e| {
                super_x = Some((*e).x);
                Ok(())
            },
            SuperSideKickError => |_e| {
                sidekick_arm_executed = true;
                Ok(())
            }
        );
        assert_eq!(super_x, None);
        assert_eq!(sidekick_arm_executed, true);
    }

    #[test]
    fn test_handle_error_unexpected() {
        let a_result = give_me_an_unexpected_result();
        let mut super_x = None;
        let mut sidekick_arm_executed = false;
        handle_error!(a_result,
            SuperError => |e| {
                super_x = Some((*e).x);
                Ok(())
            },
            SuperSideKickError => |_e| {
                sidekick_arm_executed = true;
                Ok(())
            }
        );
        assert_eq!(super_x, None);
        assert_eq!(sidekick_arm_executed, false);
    }
}
