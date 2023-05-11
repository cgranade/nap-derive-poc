mod console_hacks;

mod login;

use login::*;

mod api;
use api::{Note, NotesList, Server, ServerOption};

mod values;
use values::IntoValue;

mod errors;

use nap::serve_plugin;
use nap_derive::PluginSignatures;
use nu_plugin::LabeledError;
use nu_protocol::{Span, Value};

use dialoguer::theme::ColorfulTheme;

use once_cell::sync::Lazy;

use tokio::runtime::Builder;

use pretty_env_logger;

static THEME: Lazy<ColorfulTheme> = Lazy::new(|| ColorfulTheme::default());

#[derive(PluginSignatures)]
pub enum SynPlugin {
    #[signature("syn login")]
    #[usage("Logs in to a given Synology NAS.")]
    Login {
        #[req]
        #[usage("The domain name of the Synology NAS to log in to.")]
        name: String,
    },

    #[signature("syn note info")]
    Info {},

    #[signature("syn note list")]
    NoteList {},

    #[signature("syn note get")]
    NoteGet {
        #[opt]
        id: Option<String>,
    },
}

fn nu_main(call: SynPlugin, input: &Value) -> Result<Value, nu_plugin::LabeledError> {
    console_hacks::reset_stdin();
    pretty_env_logger::init();

    let runtime = Builder::new_multi_thread().enable_io().build().unwrap();

    let future = async {
        match call {
            SynPlugin::Login { name } => Ok(login(name).await?),

            SynPlugin::Info {} => {
                let server = Server::from_keyring().await.require()?;

                let resp = server
                    .call::<serde_json::Value>(
                        "SYNO.NoteStation.Info",
                        [
                            ("method", "get")
                        ],
                        None,
                    )
                    .await
                    .map_err(|e| LabeledError {
                        label: "Error getting note info".to_string(),
                        msg: format!("Error getting note info: {:?}", e),
                        span: None,
                    })?;

                Ok(Value::String {
                    val: serde_json::to_string(&resp).unwrap(),
                    span: Span::unknown(),
                })
            }

            SynPlugin::NoteList {} => {
                let server = Server::from_keyring().await.require()?;
                let resp = server
                    .call::<NotesList>(
                        "SYNO.NoteStation.Note",
                        [
                            ("method", "list")
                        ],
                        Some(2),
                    )
                    .await
                    .map_err(|e| LabeledError {
                        label: "Error getting notes".to_string(),
                        msg: format!("Error getting notes: {:?}", e),
                        span: None,
                    })?;

                Ok(resp.notes.into_value())
            }

            SynPlugin::NoteGet { id } => {
                let server = Server::from_keyring().await.require()?;

                let ids = match id {
                    Some(id) => vec![id],
                    None => input
                        .as_list()?
                        .iter()
                        .map(|i| i.as_string())
                        .collect::<Result<Vec<_>, _>>()?,
                };

                {
                    let server = &server;
                    futures::future::join_all(ids.iter().map(|id| async move {
                        server
                            .call::<Note>(
                                "SYNO.NoteStation.Note",
                                [
                                    ("method", "get"), ("object_id", id)
                                ],
                                Some(2),
                            )
                            .await
                            .map_err(|e| LabeledError {
                                label: format!("Error getting note {}", id),
                                msg: format!("Error getting notes: {:?}", e),
                                span: None,
                            })
                    }))
                    .await
                    .iter()
                    .cloned()
                    .collect::<Result<Vec<_>, _>>()
                    .map(|v| v.into_value())
                }
            }
        }
    };

    runtime.block_on(future)
}

fn main() {
    serve_plugin(&mut nu_main)
}
