use nu_protocol::{Value, Span};
use scryfall::Card;

pub fn opt_as_value(value: Option<Value>) -> Value {
    match value {
        Some(v) => v,
        None => Value::Nothing { span: Span::unknown() }
    }
}

pub fn card_as_value(card: &Card) -> Value {
    Value::Record {
        cols: vec![
            "arena_id".to_string(),
            "uuid".to_string(),
            "lang".to_string(),
            "mtgo_id".to_string(),
            // TODO: lots of fields
            "name".to_string(),
            // TODO: lots of fields
            "color_identity".to_string(),
        ],
        vals: vec![
            opt_as_value(card.arena_id.map(|v| Value::string(v.to_string(), Span::unknown()))),
            Value::string(card.id.to_string(), Span::unknown()),
            Value::string(card.lang.to_string(), Span::unknown()),
            opt_as_value(card.mtgo_id.map(|v| Value::string(v.to_string(), Span::unknown()))),
            Value::string(card.name.to_string(), Span::unknown()),
            Value::List {
                vals: card.color_identity.iter().map(|c| Value::string(
                    format!("{:?}", c),
                    Span::unknown()
                ))
                .collect(),
                span: Span::unknown()
            }
        ],
        span: Span::unknown()
    }
}
