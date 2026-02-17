pub mod general;
pub mod void;

use grapher::prelude::SparqlSnippet;
use grapher::prelude::strum::IntoEnumIterator;

pub fn snippets_from_enum<T>() -> Vec<&'static str>
where
    T: IntoEnumIterator + SparqlSnippet,
{
    T::iter().map(|item| item.snippet()).collect::<Vec<_>>()
}
