// Use this to implement (de)encoding error types:
// Encode: https://github.com/leptos-rs/leptos/blob/6dc8ad4bfa71c33bf67aac514204af5dcaf7a112/server_fn/src/error.rs#L264
// Decode: https://github.com/leptos-rs/leptos/blob/6dc8ad4bfa71c33bf67aac514204af5dcaf7a112/server_fn/src/error.rs#L306

use leptos::{
    prelude::{FromServerFnError, ServerFnError, ServerFnErrorErr},
    server_fn::{Decodes, Encodes, codec::RkyvEncoding, error::IntoAppError},
};
use serde::{Deserialize, Serialize};

/// Logs a server message at the error level.
///
/// Please use the `target` argument to define the error type. See Examples.
///
/// Returns a [`leptos::prelude::ServerFnError::ServerError<String>`]
///
/// # Examples
///
/// ```
/// use vowlr_util::error_s;
/// use leptos::prelude::ServerFnError;
///
/// let (err_info, port) = ("No connection", 22);
///
/// let s = error_s!("Error: {err_info} on port {port}");
/// let s1 = error_s!(target: "serializer", "App Error: {err_info}, Port: {port}");
///
/// assert_eq!()
/// ```
#[macro_export]
macro_rules! error_s {
    // error!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // error!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => ({
        $log::error!($($arg)+)
        $leptos::prelude::ServerFnError::ServerError(
                    $std::format_args!($target, $($arg)+)
                )
    });

    // error!(key1 = 42, key2 = true; "a {} event", "log")
    // error!("a {} event", "log")
    ($($arg:tt)+) => ({
        $log::error!($($arg)+)
        $leptos::prelude::ServerFnError::ServerError(
                    $std::format_args!($($arg)+)
                )
    });
}

#[derive(
    Debug, Copy, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Serialize, Deserialize,
)]
pub enum ErrorSeverity {
    Critical,
    Severe,
    Medium,
    Low,
    Warning,
    Unset,
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'e>) -> std::fmt::Result {
        match self {
            Self::Critical => write!(
                f,
                "an unrecoverable error which makes VOWL-R unusable (do not use the output of VOWL-R!)"
            ),
            Self::Severe => write!(
                f,
                "an error which highly disrupts the user experience (the output of VOWL-R is likely incorrect)"
            ),
            Self::Medium => write!(f, "error desc goes here"),
            Self::Low => write!(
                f,
                "error desc goes here (part of the output of VOWL-R could be incorrect, but should be \"insignificant\")"
            ),
            Self::Warning => write!(
                f,
                "something happened which may reduce the user experience (but can otherwise be ignored)"
            ),
            Self::Unset => write!(f, "unknown severity"),
        }
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Serialize,
    Deserialize,
    strum::Display,
)]
pub enum ErrorType {
    /// Errors related to database operations.
    Database,
    /// Errors related to serializing data from backend to frontend (server -> client).
    Serializer,
    /// Errors related to parsing data (e.g. a `.owl` file).
    Parser,
    /// Errors related to the graph renderer (i.e. WasmGrapher)
    Renderer,
    #[strum(serialize = "GUI")]
    /// Errors related to the frontend GUI.
    Gui,
    /// Errors without a type. Equivalent to a "500 Internal Server Error"
    Generic,
}

#[derive(
    Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Serialize, Deserialize,
)]
pub struct ErrorRecord {
    /// The severity of an error.
    ///
    /// Useful for grouping errors by severity and applying custom color schemes in the GUI.
    pub severity: ErrorSeverity,
    /// The type of an error.
    ///
    /// Useful for grouping errors by type and debugging for devs.
    pub error_type: ErrorType,
    /// The actual error message to show.
    pub message: String,

    #[cfg(debug_assertions)]
    /// The location in the source code where the error originated.
    ///
    /// Only enabled with [cfg.debug_assertions]
    pub location: String,
}

impl ErrorRecord {
    pub fn new(
        severity: ErrorSeverity,
        error_type: ErrorType,
        message: String,
        #[cfg(debug_assertions)] location: String,
    ) -> Self {
        Self {
            severity,
            error_type,
            message,
            #[cfg(debug_assertions)]
            location,
        }
    }
}

#[derive(
    Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Serialize, Deserialize,
)]
pub struct VOWLRServerError {
    pub records: Vec<ErrorRecord>,
}

impl<VOWLRServerError> FromServerFnError for ServerFnError<VOWLRServerError> {
    type Encoder = RkyvEncoding;

    fn from_server_fn_error(value: ServerFnErrorErr) -> Self {
        let (severity, error_type, message, location) = match value {
            ServerFnErrorErr::Registration(e) => todo!(),
            ServerFnErrorErr::UnsupportedRequestMethod(e) => todo!(),
            ServerFnErrorErr::Request(e) => todo!(),
            ServerFnErrorErr::ServerError(e) => todo!(),
            ServerFnErrorErr::MiddlewareError(e) => todo!(),
            ServerFnErrorErr::Deserialization(e) => todo!(),
            ServerFnErrorErr::Serialization(e) => todo!(),
            ServerFnErrorErr::Args(e) => todo!(),
            ServerFnErrorErr::MissingArg(e) => todo!(),
            ServerFnErrorErr::Response(e) => todo!(),
        }

    }

    fn ser(&self) -> leptos::server_fn::Bytes {
        Self::Encoder::encode(self).unwrap_or_else(|e| {
            Self::Encoder::encode(&Self::from_server_fn_error(
                ServerFnErrorErr::Serialization(e.to_string()),
            ))
            .expect(
                "error serializing should success at least with the \
                 Serialization error",
            )
        })
    }

    fn de(data: leptos::server_fn::Bytes) -> Self {
        Self::Encoder::decode(data)
            .unwrap_or_else(|e| ServerFnErrorErr::Deserialization(e.to_string()).into_app_error())
    }
}
