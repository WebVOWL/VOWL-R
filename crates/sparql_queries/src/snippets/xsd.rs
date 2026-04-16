use grapher::prelude::XSDNode;

use crate::snippets::SparqlSnippet;

impl SparqlSnippet for XSDNode {
    fn snippet(self) -> &'static str {
        match self {
            Self::Int => {
                r"{
                ?id a xsd:int .
                BIND(xsd:int AS ?nodeType)
                }"
            }
            Self::Integer => {
                r"{
                ?id a xsd:integer .
                BIND(xsd:integer AS ?nodeType)
                }"
            }
            Self::NegativeInteger => {
                r"{
                ?id a xsd:negativeInteger .
                BIND(xsd:negativeInteger AS ?nodeType)
                }"
            }
            Self::NonNegativeInteger => {
                r"{
                ?id a xsd:nonNegativeInteger .
                BIND(xsd:nonNegativeInteger AS ?nodeType)
                }"
            }
            Self::NonPositiveInteger => {
                r"{
                ?id a xsd:nonPositiveInteger .
                BIND(xsd:nonPositiveInteger AS ?nodeType)
                }"
            }
            Self::PositiveInteger => {
                r"{
                ?id a xsd:positiveInteger .
                BIND(xsd:positiveInteger AS ?nodeType)
                }"
            }
            Self::UnsignedInt => {
                r"{
                ?id a xsd:unsignedInt .
                BIND(xsd:unsignedInt AS ?nodeType)
                }"
            }
            Self::UnsignedLong => {
                r"{
                ?id a xsd:unsignedLong .
                BIND(xsd:unsignedLong AS ?nodeType)
                }"
            }
            Self::UnsignedShort => {
                r"{
                ?id a xsd:unsignedShort .
                BIND(xsd:unsignedShort AS ?nodeType)
                }"
            }
            Self::Decimal => {
                r"{
                ?id a xsd:decimal .
                BIND(xsd:decimal AS ?nodeType)
                }"
            }
            Self::Float => {
                r"{
                ?id a xsd:float .
                BIND(xsd:float AS ?nodeType)
                }"
            }
            Self::Double => {
                r"{
                ?id a xsd:double .
                BIND(xsd:double AS ?nodeType)
                }"
            }
            Self::Short => {
                r"{
                ?id a xsd:short .
                BIND(xsd:short AS ?nodeType)
                }"
            }
            Self::Long => {
                r"{
                ?id a xsd:long .
                BIND(xsd:long AS ?nodeType)
                }"
            }
            Self::Date => {
                r"{
                ?id a xsd:date .
                BIND(xsd:date AS ?nodeType)
                }"
            }
            Self::DataTime => {
                r"{
                ?id a xsd:dateTime .
                BIND(xsd:dateTime AS ?nodeType)
                }"
            }
            Self::DateTimeStamp => {
                r"{
                ?id a xsd:dateTimeStamp .
                BIND(xsd:dateTimeStamp AS ?nodeType)
                }"
            }
            Self::Duration => {
                r"{
                ?id a xsd:duration .
                BIND(xsd:duration AS ?nodeType)
                }"
            }
            Self::GDay => {
                r"{
                ?id a xsd:gDay .
                BIND(xsd:gDay AS ?nodeType)
                }"
            }
            Self::GMonth => {
                r"{
                ?id a xsd:gMonth .
                BIND(xsd:gMonth AS ?nodeType)
                }"
            }
            Self::GMonthDay => {
                r"{
                ?id a xsd:gMonthDay .
                BIND(xsd:gMonthDay AS ?nodeType)
                }"
            }
            Self::GYear => {
                r"{
                ?id a xsd:gYear .
                BIND(xsd:gYear AS ?nodeType)
                }"
            }
            Self::GYearMonth => {
                r"{
                ?id a xsd:gYearMonth .
                BIND(xsd:gYearMonth AS ?nodeType)
                }"
            }
            Self::Time => {
                r"{
                ?id a xsd:time .
                BIND(xsd:time AS ?nodeType)
                }"
            }
            Self::AnyURI => {
                r"{
                ?id a xsd:anyURI .
                BIND(xsd:anyURI AS ?nodeType)
                }"
            }
            Self::ID => {
                r"{
                ?id a xsd:ID .
                BIND(xsd:ID AS ?nodeType)
                }"
            }
            Self::Idref => {
                r"{
                ?id a xsd:IDREF .
                BIND(xsd:IDREF AS ?nodeType)
                }"
            }
            Self::Language => {
                r"{
                ?id a xsd:language .
                BIND(xsd:language AS ?nodeType)
                }"
            }
            Self::Nmtoken => {
                r"{
                ?id a xsd:NMTOKEN .
                BIND(xsd:NMTOKEN AS ?nodeType)
                }"
            }
            Self::Name => {
                r"{
                ?id a xsd:Name .
                BIND(xsd:Name AS ?nodeType)
                }"
            }
            Self::NCName => {
                r"{
                ?id a xsd:NCName .
                BIND(xsd:NCName AS ?nodeType)
                }"
            }
            Self::QName => {
                r"{
                ?id a xsd:QName .
                BIND(xsd:QName AS ?nodeType)
                }"
            }
            Self::String => {
                r"{
                ?id a xsd:string .
                BIND(xsd:string AS ?nodeType)
                }"
            }
            Self::Token => {
                r"{
                ?id a xsd:token .
                BIND(xsd:token AS ?nodeType)
                }"
            }
            Self::NormalizedString => {
                r"{
                ?id a xsd:normalizedString .
                BIND(xsd:normalizedString AS ?nodeType)
                }"
            }
            Self::Notation => {
                r"{
                ?id a xsd:NOTATION .
                BIND(xsd:NOTATION AS ?nodeType)
                }"
            }
            Self::AnySimpleType => {
                r"{
                ?id a xsd:anySimpleType .
                BIND(xsd:anySimpleType AS ?nodeType)
                }"
            }
            Self::Base64Binary => {
                r"{
                ?id a xsd:base64Binary .
                BIND(xsd:base64Binary AS ?nodeType)
                }"
            }
            Self::Boolean => {
                r"{
                ?id a xsd:boolean .
                BIND(xsd:boolean AS ?nodeType)
                }"
            }
            Self::Entity => {
                r"{
                ?id a xsd:ENTITY .
                BIND(xsd:ENTITY AS ?nodeType)
                }"
            }
            Self::UnsignedByte => {
                r"{
                ?id a xsd:unsignedByte .
                BIND(xsd:unsignedByte AS ?nodeType)
                }"
            }
            Self::Byte => {
                r"{
                ?id a xsd:byte .
                BIND(xsd:byte AS ?nodeType)
                }"
            }
            Self::HexBinary => {
                r"{
                ?id a xsd:hexBinary .
                BIND(xsd:hexBinary AS ?nodeType)
                }"
            }
        }
    }
}
