use nu_protocol::{Value, Span};

use crate::api::{Note};

pub trait IntoValue {
    fn into_value(&self) -> Value;
}

impl<T: IntoValue> IntoValue for Vec<T> {
    fn into_value(&self) -> Value {
        Value::List {
            vals: self
                .iter()
                .map(|i| i.into_value())
                .collect(),
            span: Span::unknown()
        }
    }
}

impl IntoValue for Note {
    fn into_value(&self) -> Value {
        let mut cols = vec![
            "title".to_string(),
        ];
        let mut vals = vec![
            Value::string(self.title.to_owned(), Span::unknown()),
        ];

        if let Some(ref content) = self.content {
            cols.push("content".to_string());
            vals.push(Value::string(content.to_owned(), Span::unknown()));
        }

        
        cols.push("brief".to_string());
        vals.push(Value::string(self.brief.to_owned(), Span::unknown()));

        cols.push("id".to_string());
        vals.push(Value::string(self.object_id.to_owned(), Span::unknown()));

        Value::Record {
            cols,
            vals,
            span: Span::unknown()
        }
    }
}

impl IntoValue for serde_json::Value {
    fn into_value(&self) -> Value {
        Value::string(serde_json::to_string(self).unwrap(), Span::unknown())
    }
}
