extern crate anyhow;
extern crate bincode;
extern crate conway;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate serde;
extern crate snowflake;
extern crate thiserror;

pub mod app;
pub mod common;
pub mod filter;
pub mod protocol;
mod settings;
pub mod transport;

pub use settings::DEFAULT_PORT;

// This project formats log statements in according to the following format.
//
// Date \[Log Level\] - \[Layer Context\] Message
//
// where
//     Date: Follows the short-hand day of the week, 'Thu' for Thursday followed by the datetime in ISO 8601
//
//     Log Level: The log severity level such as 'TRACE', 'INFO', 'DEBUG', 'WARN', or 'ERROR'.
//
//     Layer Context: A shorthand notation for the Netwayste networking architecture's three-layer design.
//         "Log Message Source Layer" [<- "Event Origin Layer", "Event Type"]
//
//         "F<-T,R" means the message was logged by the Filter layer, having received a Transport Response
//         "T<-F,C" means the message was logged by the Transport layer, having received a Command sent by the Filter.
//         "A<-F,N" means the message was logged by the Application layer, having received a Notification sent by the Filter.
//
//         The log message source and event origin layers can be one of
//             A: Application (Client or Server)
//             F: Filter
//             T: Transport
//
//         The event type can be one of:
//             C: Contextual Command, such as a Filter or Transport Command
//             R: Contextual Response, such as a Filter or Transport Response
//             N: Contextual Notice, such as a Filter or Transport Notice
//             UDP: UDP network transmission or reception
//             UGR: Universe Generation Response
//             UGN: Universe Generation Notice
//
//     Message: A variable-length sequence of Unicode characters terminated by a newline, '\n'
#[macro_export(local_inner_macros)]
macro_rules! nwtrace {
    ($self:ident, $string:tt, $($arg:tt)*) => (
        if std::cfg!(feature = "contextual_logging")
        {
            $crate::log::trace!(std::concat!("{}", $string), $self.mode, $($arg)+)
        }
        else
        {
            $crate::log::trace!($string, $($arg)+)
        }
    );
    ($self:ident, $string:tt) => (
        if std::cfg!(feature = "contextual_logging")
        {
            $crate::log::trace!(std::concat!("{}", $string), $self.mode)
        }
        else
        {
            $crate::log::trace!($string)
        }
    )
}

#[macro_export(local_inner_macros)]
macro_rules! nwerror {
    ($self:ident, $string:tt, $($arg:tt)*) => (
        if std::cfg!(feature = "contextual_logging")
        {
            $crate::log::error!(std::concat!("{}", $string), $self.mode, $($arg)+)
        }
        else
        {
            $crate::log::error!($string, $($arg)+)
        }
    );
    ($self:ident, $string:tt) => (
        if std::cfg!(feature = "contextual_logging")
        {
            $crate::log::error!(std::concat!("{}", $string), $self.mode)
        }
        else
        {
            $crate::log::error!($string)
        }
    )
}

#[macro_export(local_inner_macros)]
macro_rules! nwinfo {
    ($self:ident, $string:tt, $($arg:tt)*) => (
        if std::cfg!(feature = "contextual_logging")
        {
            $crate::log::info!(std::concat!("{}", $string), $self.mode, $($arg)+)
        }
        else
        {
            $crate::log::info!($string, $($arg)+)
        }
    );
    ($self:ident, $string:tt) => (
        if std::cfg!(feature = "contextual_logging")
        {
            $crate::log::info!(std::concat!("{}", $string), $self.mode)
        }
        else
        {
            $crate::log::info!($string)
        }
    )
}

#[macro_export(local_inner_macros)]
macro_rules! nwdebug {
    ($self:ident, $string:tt, $($arg:tt)*) => (
        if std::cfg!(feature = "contextual_logging")
        {
            $crate::log::debug!(std::concat!("{}", $string), $self.mode, $($arg)+)
        }
        else
        {
            $crate::log::debug!($string, $($arg)+)
        }
    );
    ($self:ident, $string:tt) => (
        if std::cfg!(feature = "contextual_logging")
        {
            $crate::log::debug!(std::concat!("{}", $string), $self.mode)
        }
        else
        {
            $crate::log::debug!($string)
        }
    )
}

#[macro_export(local_inner_macros)]
macro_rules! nwwarn {
    ($self:ident, $string:tt, $($arg:tt)*) => (
        if std::cfg!(feature = "contextual_logging")
        {
            $crate::log::warn!(std::concat!("{}", $string), $self.mode, $($arg)+)
        }
        else
        {
            $crate::log::warn!($string, $($arg)+)
        }
    );
    ($self:ident, $string:tt) => (
        if std::cfg!(feature = "contextual_logging")
        {
            $crate::log::warn!(std::concat!("{}", $string), $self.mode)
        }
        else
        {
            $crate::log::warn!($string)
        }
    )
}
