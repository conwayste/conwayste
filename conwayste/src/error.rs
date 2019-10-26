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
/// It consumes a `Result<T, E>` where `E` must be `dyn Error`. The macro attempts to downcast the
/// result into one or more concrete error types as specified in the "match" expressions. A
/// catch-all `else` match expression may be placed at the end.
///
/// If any match arm executes, an Ok result is returned. If no match arms execute, an Err result is
/// returned with the original error (in other words, the original result is passed through
/// unmodified).
///
/// NOTE: be sure to add commas in the right places otherwise you will see a "no rules expected
/// this token in macro call" compile error.
///
/// # Usage
///     handle_error!(input_result [ -> custom_return_type ],
///         ErrorType1 => |err_var| { ... }      // matcher 1
///         ErrorType2 => |err_var| { ... }      // matcher 2
///         ...
///         [ else => |box_err_var| { ... } ]
///     );
///
/// # Arguments
///     input_result - a result type
///     custom_return_type - (optional) specify a type T for Ok(T); Default type is ()
///     ErrorType - a concrete error type which implements Error trait
///     err_var - a variable which will contain the error downcast into ErrorType
///     box_err_var - like `err_var` except boxed, because the else doesn't have a concrete type to
///     unbox into.
///
/// # Examples
/// Here we match on a `SuperError`.
/// ```rust,ignore
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
///     },
///     SuperSideKickError => |e| {
///         // e is a SuperSideKickError concrete error type
///         println!("I got a SuperSideKickError after the downcast!");
///     },
/// )?;  // return any Err(Box<dyn Error>) that doesn't match. This won't happen for
///      //give_me_an_error_result, however.
/// ```
/// There are no matches in this case, to which the original result is preserved, unmodified. Use
/// an "else" to handle this case.
/// ```rust,ignore
/// // See the Error documentation page for the SuperError and SuperSideKickError definitions.
///
/// fn give_me_an_error_result() -> Result<(), Box<dyn Error>> {
///     // I made up SuperErrorNemesis for this specific example
///     Err(Box::new(SuperErrorNemesis{}))
/// }
///
/// let a_result = give_me_an_error_result();
/// handle_error!(a_result,
///     SuperError => |e| {
///         println!("I got a SuperError after the downcast");
///     },
///     SuperSideKickError => |e| {
///         println!("I got a SuperSideKickError after the downcast!");
///     },
///     else => |e| {
///         // e is a Box<dyn Error>
///         println!("I got an unexpected error");
///     }
/// ).unwrap(); // this unwrap will never panic because of the else block
/// ```
///

#[macro_export]
macro_rules! handle_error {
   ($input:ident -> $type:ty $(, $matcher:ty => |$var:ident| $result:expr)*) => {
       $input
        $(
            .or_else(|boxed_error| -> Result<$type, Box<dyn Error>> {
                boxed_error.downcast::<$matcher>()
                    .and_then(|$var: Box<$matcher>| {
                        let $var = *$var; // unbox
                        let val = $result;
                        Ok(val)
                    })
            })
        )*
   };

   ($input:ident -> $type:ty $(, $matcher:ty => |$var:ident| $result:expr)*, else => |$default_var:ident| $default_result:expr) => {
       handle_error!($input -> $type
            $(
                , $matcher => |$var| $result
            )*
       )
        .or_else(|boxed_error| -> Result<$type, Box<dyn Error>> {
            let $default_var = boxed_error;
            let val = $default_result;
            Ok(val)
        })
   };

   ($input:ident $(, $matcher:ty => |$var:ident| $result:expr)*) => {
       handle_error!($input -> ()
            $(
                , $matcher => |$var| $result
            )*
       )
   };

   ($input:ident $(, $matcher:ty => |$var:ident| $result:expr)*, else => |$default_var:ident| $default_result:expr) => {
       handle_error!($input -> ()
            $(
                , $matcher => |$var| $result
            )*
            , else => |$default_var| $default_result
       )
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

    fn give_me_a_custom_result_i32() -> Result<i32, Box<dyn Error>> {
        Ok(5)
    }

    fn give_me_a_custom_result_i32_super_err() -> Result<i32, Box<dyn Error>> {
        Err(Box::new(SuperError{x: 3}))
    }

    #[test]
    fn test_handle_error_super() {
        let a_result = give_me_a_super_result();
        let mut super_x = None;
        let mut sidekick_arm_executed = false;
        let res = handle_error!(a_result,
            SuperError => |e| {
                super_x = Some(e.x);
            },
            SuperSideKickError => |_e| {
                sidekick_arm_executed = true;
            }
        );
        assert_eq!(super_x, Some(3));
        assert_eq!(sidekick_arm_executed, false);
        assert_eq!(res.unwrap(), ());
    }

    #[test]
    fn test_handle_error_sidekick() {
        let a_result = give_me_a_sidekick_result();
        let mut super_x = None;
        let mut sidekick_arm_executed = false;
        let res = handle_error!(a_result,
            SuperError => |e| {
                super_x = Some(e.x);
            },
            SuperSideKickError => |_e| {
                sidekick_arm_executed = true;
            }
        );
        assert_eq!(res.unwrap(), ()); // error should have been handled
        assert_eq!(super_x, None);
        assert_eq!(sidekick_arm_executed, true);
    }

    #[test]
    fn test_handle_error_sidekick_with_default() {
        let a_result = give_me_a_sidekick_result();
        let mut super_x = None;
        let mut sidekick_arm_executed = false;
        let mut default_arm_executed = false;
        let res = handle_error!(a_result,
            SuperError => |e| {
                super_x = Some(e.x);
            },
            SuperSideKickError => |_e| {
                sidekick_arm_executed = true;
            },
            else => |_e| {
                default_arm_executed = true;
            }
        );
        assert_eq!(res.unwrap(), ()); // error should have been handled, and in fact would always be handled because of else
        assert_eq!(super_x, None);
        assert_eq!(sidekick_arm_executed, true);
        assert_eq!(default_arm_executed, false);
    }

    #[test]
    fn test_handle_error_unexpected_no_default() {
        let a_result = give_me_an_unexpected_result();
        let mut super_x = None;
        let mut sidekick_arm_executed = false;
        let res = handle_error!(a_result,
            SuperError => |e| {
                super_x = Some(e.x);
            },
            SuperSideKickError => |_e| {
                sidekick_arm_executed = true;
            }
        );
        assert!(res.unwrap_err().downcast::<io::Error>().is_ok());  // pass through original error unmodified
        assert_eq!(super_x, None);
        assert_eq!(sidekick_arm_executed, false);
    }

    #[test]
    fn test_handle_error_unexpected_with_default() {
        let a_result = give_me_an_unexpected_result();
        let mut super_x = None;
        let mut sidekick_arm_executed = false;
        let mut default_arm_executed = false;
        let res = handle_error!(a_result,
            SuperError => |e| {
                super_x = Some(e.x);
            },
            SuperSideKickError => |_e| {
                sidekick_arm_executed = true;
            },
            else => |_e| {
                default_arm_executed = true;
            }
        );
        assert_eq!(res.unwrap(), ()); // error should have been handled, and in fact would always be handled because of else
        assert_eq!(super_x, None);
        assert_eq!(sidekick_arm_executed, false);
        assert_eq!(default_arm_executed, true);
    }

    #[test]
    fn test_handle_error_custom_type() {
        let a_result = give_me_a_custom_result_i32();

        let res = handle_error!(a_result -> i32,
            SuperError => |_e| {
                10
            },
            SuperSideKickError => |_e| {
                20
            }
        );

        assert_eq!(res.unwrap(), 5);
    }

    #[test]
    fn test_handle_error_custom_type_super_err() {
        let a_result = give_me_a_custom_result_i32_super_err();

        let res = handle_error!(a_result -> i32,
            SuperError => |_e| {
                10
            },
            SuperSideKickError => |_e| {
                20
            }
        );

        assert_eq!(res.unwrap(), 10);
    }

}
