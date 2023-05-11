use std::collections::HashMap;

use dialoguer::{Input, Password};
use keyring::Entry;
use nu_plugin::LabeledError;

use nu_protocol::{Value, Span};

use super::THEME;
use super::api::{Server, Session};

pub struct LoginCredentials {
    username: String,
    password: String,
    otp: Option<String>
}


pub fn ask_for_credentials() -> Result<LoginCredentials, LabeledError> {    
    let mut username_input = Input::<String>::with_theme(&*THEME);
    username_input.with_prompt("Username");
    let username = username_input.interact_text().or_else(|e| {
        Err(nu_plugin::LabeledError {
            label: e.kind().to_string(),
            msg: format!("I/O error getting username: {}", e.to_string()),
            span: None
        })
    })?;

    let mut password_input = Password::with_theme(&*THEME);
    password_input.with_prompt("Password");
    let password = password_input.interact().or_else(|e| {
        Err(nu_plugin::LabeledError {
            label: e.kind().to_string(),
            msg: format!("I/O error getting username: {}", e.to_string()),
            span: None
        })
    })?;

    let mut otp_input = Input::<String>::with_theme(&*THEME);
    otp_input.allow_empty(true);
    otp_input.with_prompt("One-time code");
    let otp = otp_input.interact_text().or_else(|e| {
        Err(nu_plugin::LabeledError {
            label: e.kind().to_string(),
            msg: format!("I/O error getting username: {}", e.to_string()),
            span: None
        })
    })?;
    let otp = if otp.trim().is_empty() {
        None
    } else {
        Some(otp.trim().to_string())
    };

    Ok(LoginCredentials { username, password, otp })
}


pub async fn login<T: AsRef<str>>(server_name: T) -> Result<Value, LabeledError> {
    let server_name = server_name.as_ref();
    let server = Server
        ::new(server_name)
        .await
        .map_err(|e| LabeledError {
            label: format!("Could not query API endpoints for {}", server_name),
            msg: format!("{:?}", e),
            span: None
        })?;
    let credentials = ask_for_credentials()?;

    let mut parameters = HashMap::new();
    parameters.insert("method", "login");
    parameters.insert("format", "sid");
    parameters.insert("account", &credentials.username);
    parameters.insert("passwd", &credentials.password);
    if let Some(ref otp) = credentials.otp {
        parameters.insert("otp_code", otp);
    }
    // Prevent further mutation.
    let parameters = parameters;
    
    let body = server
        .call::<Session>("SYNO.API.Auth", &parameters, Some(3))
        .await
        .map_err(|e| LabeledError {
            label: format!("Could not log in to {}", server_name),
            msg: format!("{:?}", e),
            span: None
        })?;

    // If we made it this far, save the secrets out to the keyring.
    Entry::new("nu_syn", "server_name")
        .and_then(|e| e.set_password(server_name))
        .map_err(|e| LabeledError {
            label: format!("Could not save server name to keyring."),
            msg: format!("{:?}", e),
            span: None
        })?;
    Entry::new("nu_syn", "session_id")
        .and_then(|e| e.set_password(&body.sid))
        .map_err(|e| LabeledError {
            label: format!("Could not save session to keyring."),
            msg: format!("{:?}", e),
            span: None
        })?;


    Ok(Value::String {
        val: format!("Successfully logged into {}", server_name),
        span: Span::unknown()
    })
}
