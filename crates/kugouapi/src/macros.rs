//! Internal declarative macros for parameter-struct boilerplate.
//!
//! [`kugou_params!`] generates a strongly-typed parameter struct with:
//!
//! - `new(...)` taking the required fields,
//! - chainable setters for optional fields,
//! - serde `Serialize` / `Deserialize` against the endpoint's native wire
//!   names (so the dynamic `client.call(id, json)` path round-trips through the
//!   same type).
//!
//! ```ignore
//! kugou_params! {
//!     /// Search songs (`search.song`).
//!     pub SearchSongParams {
//!         required {
//!             /// Search keyword.
//!             keyword: String => "keyword",
//!         }
//!         optional {
//!             /// Page number.
//!             page: FlexI64 => "page",
//!             /// Page size.
//!             page_size: FlexI64 => "pagesize",
//!         }
//!     }
//! }
//! ```

macro_rules! kugou_params {
    (
        $(#[$meta:meta])*
        $vis:vis $name:ident {
            required {
                $( $(#[$rm:meta])* $rf:ident : $rt:ty => $rw:literal ),* $(,)?
            }
            optional {
                $( $(#[$om:meta])* $of:ident : $ot:ty => $ow:literal ),* $(,)?
            }
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
        $vis struct $name {
            $(
                $(#[$rm])*
                #[serde(rename = $rw)]
                pub $rf: $rt,
            )*
            $(
                $(#[$om])*
                #[serde(rename = $ow, default, skip_serializing_if = "Option::is_none")]
                pub $of: Option<$ot>,
            )*
        }

        impl $name {
            /// Construct with the required parameters; optional fields are
            /// chainable via builder methods.
            #[must_use]
            pub fn new($($rf: impl Into<$rt>),*) -> Self {
                Self {
                    $($rf: $rf.into(),)*
                    $($of: None,)*
                }
            }

            $(
                /// Builder setter for an optional field.
                #[must_use]
                pub fn $of(mut self, value: impl Into<$ot>) -> Self {
                    self.$of = Some(value.into());
                    self
                }
            )*
        }
    };
}
