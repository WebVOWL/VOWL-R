use grapher::prelude::Characteristic;

use crate::snippets::SparqlSnippet;

impl SparqlSnippet for Characteristic {
    fn snippet(self) -> &'static str {
        match self {
            Self::TransitiveProperty => {
                r"{
                    ?id a owl:TransitiveProperty .
                    FILTER(?id NOT IN (rdfs:subClassOf, rdfs:subPropertyOf))
                    BIND(owl:TransitiveProperty AS ?nodeType)
                }"
            }
            Self::FunctionalProperty => {
                r"{
                    ?id a owl:FunctionalProperty .
                    FILTER(?id NOT IN (rdfs:subClassOf, rdfs:subPropertyOf))
                    BIND(owl:FunctionalProperty AS ?nodeType)
                }"
            }

            Self::InverseFunctionalProperty => {
                r"{
                    ?id a owl:InverseFunctionalProperty .
                    FILTER(?id NOT IN (rdfs:subClassOf, rdfs:subPropertyOf))
                    BIND(owl:InverseFunctionalProperty AS ?nodeType)
                }"
            }

            Self::ReflexiveProperty => {
                r"{
                    ?id a owl:ReflexiveProperty .
                    FILTER(?id NOT IN (rdfs:subClassOf, rdfs:subPropertyOf))
                    BIND(owl:ReflexiveProperty AS ?nodeType)
                }"
            }

            Self::IrreflexiveProperty => {
                r"{
                    ?id a owl:IrreflexiveProperty .
                    FILTER(?id NOT IN (rdfs:subClassOf, rdfs:subPropertyOf))
                    BIND(owl:IrreflexiveProperty AS ?nodeType)
                }"
            }

            Self::SymmetricProperty => {
                r"{
                    ?id a owl:SymmetricProperty .
                    FILTER(?id NOT IN (rdfs:subClassOf, rdfs:subPropertyOf))
                    BIND(owl:SymmetricProperty AS ?nodeType)
                }"
            }
            Self::AsymmetricProperty => {
                r"{
                ?id a owl:AsymmetricProperty
                BIND(owl:AsymmetricProperty AS ?nodeType)
            }"
            }
            Self::HasKey => {
                r"{
                ?id a owl:hasKey
                BIND(owl:hasKey AS ?nodeType)
            }"
            }
        }
    }
}
