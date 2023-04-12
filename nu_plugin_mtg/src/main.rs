mod values;
use values::*;

use nap::{serve_plugin};
use nap_derive::PluginSignatures;
use nu_protocol::{Value, Span};
use scryfall::{Card, search::{query::{Query}, prelude as scry}};
use tokio::runtime::Builder;

#[derive(PluginSignatures)]
pub enum MtgPlugin {
    #[signature("mtg tutor")]
    #[usage("Searches Scryfall for a single card and returns it.")]
    Tutor {
        #[req]
        card_name: String,

        #[flag]
        #[usage("If set, will search for cards using a fuzzy match on the card name.")]
        fuzzy: bool
    },

    #[signature("mtg search")]
    #[usage("Searches Scryfall for cards matching a query and returns them.")]
    Search {
        #[req]
        #[usage("Name of the card to search for.")]
        name: String,

        #[flag]
        #[usage("Only search this set or edition for cards.")]
        set: Option<String>
    }
}

fn nu_main(call: MtgPlugin, _input: &Value) -> Result<Value, nu_plugin::LabeledError> {
    let runtime = Builder::new_multi_thread()
        .enable_io()
        .build()
        .unwrap();

    let a = async {
        match call {
            MtgPlugin::Tutor { card_name, fuzzy } => {
                match if fuzzy {
                    Card::named_fuzzy(&card_name).await
                } else {
                    Card::named(&card_name).await
                } {
                    Ok(card) => {
                        Ok(card_as_value(&card))
                    },
                    Err(e) => {
                        Err(nu_plugin::LabeledError {
                            label: e.to_string(),
                            msg: "Card not found.".into(),
                            span: None
                        })
                    }
                }
            },

            MtgPlugin::Search { name, set } => {
                let query = Query::And(if let Some(set) = set {
                    vec![
                        scry::name(name),
                        scry::set(set),
                    ]
                } else {
                    vec![
                        scry::name(name)
                    ]
                });
                match Card::search_all(query).await {
                    Ok(cards) => {
                        Ok(Value::List {
                            vals: cards.iter().map(card_as_value).collect(),
                            span: Span::unknown()
                        })
                    },
                    Err(e) => {
                        Err(nu_plugin::LabeledError {
                            label: e.to_string(),
                            msg: "Card not found.".into(),
                            span: None
                        })
                    }
                }
            }
        }
    };

    runtime.block_on(a)
}

fn main() {
    serve_plugin(&mut nu_main)
}
