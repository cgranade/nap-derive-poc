use std::{collections::HashMap, borrow::Borrow};

use keyring::Entry;
use nu_plugin::LabeledError;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use surf;

use once_cell::sync::Lazy;

use crate::errors::{SynError, Result};

pub static CLIENT: Lazy<surf::Client> = Lazy::new(||
    surf::Config::default()
        .try_into()
        .unwrap()
);

pub trait IntoParameters<'a> {
    type Output: Borrow<HashMap<&'a str, &'a str>>;
    fn into_parameters(self) -> Self::Output;
}

impl<'a, const N: usize> IntoParameters<'a> for [(&'a str, &'a str); N] {
    type Output = HashMap<&'a str, &'a str>;
    fn into_parameters(self) -> Self::Output {
        HashMap::from(self)
    }
}

impl<'a, 'b> IntoParameters<'a> for &'b HashMap<&'a str, &'a str> {
    type Output = Self;
    fn into_parameters(self) -> Self::Output {
        self
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ErrorCode {
    code: u32
}

pub type NuResult<T> = core::result::Result<T, LabeledError>;

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum SynResult<T> {
    Success { data: T },
    Error { error: ErrorCode }
}

#[derive(Serialize, Deserialize, Debug)]
struct SynResponse<T> {
    #[serde(flatten)]
    data: SynResult<T>,
    success: bool
}

impl<T> Into<Result<T>> for SynResponse<T> {
    fn into(self) -> Result<T> {
        match self.data {
            SynResult::Error { error } => Err(SynError::from_api_error_code(error.code)),
            SynResult::Success { data } => Ok(data)
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Session {
    pub sid: String
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiEndpoint {
    max_version: u32,
    min_version: u32,
    path: String,
    request_format: Option<String>,
}

pub struct Server {
    name: String,
    endpoints: HashMap<String, ApiEndpoint>,
    session_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub display_name: String,
    pub uid: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NotesList {
    pub notes: Vec<Note>,
    pub offset: usize,
    pub total: usize
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[non_exhaustive]
pub struct Note {
    pub acl: serde_json::Value,
    pub archive: bool,
    pub brief: String,
    pub category: String,
    pub ctime: u64,
    pub encrypt: bool,
    pub mtime: u64,
    pub object_id: String,
    pub owner: User,
    pub parent_id: String,
    pub perm: String,
    pub recycle: bool,
    pub thumb: serde_json::Value,
    pub title: String,
    pub ver: String,

    // These fields may be missing when we run list instead of get.
    pub content: Option<String>,
    pub attachment: Option<serde_json::Value>,
    pub commit_msg: Option<serde_json::Value>,
    pub individual_joined: Option<bool>,
    pub individual_shared: Option<bool>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub link_id: Option<String>,
    pub location: Option<String>,
    pub source_url: Option<String>,
    pub tag: Option<Vec<String>>,
}

impl Server {
    pub async fn new<T: AsRef<str>>(name: T) -> Result<Self> {
        let mut new = Self {
            name: name.as_ref().to_string(),
            endpoints: HashMap::new(),
            session_id: None
        };

        new.endpoints = new.get_endpoints().await?;

        Ok(new)
    }

    pub async fn from_keyring() -> Option<Self> {
        let name = Entry::new("nu_syn", "server_name")
            .and_then(|e| e.get_password())
            .ok()?;
        let session_id = Some(Entry::new("nu_syn", "session_id")
            .and_then(|e| e.get_password())
            .ok()?);


        let mut new = Self {
            name,
            endpoints: HashMap::new(),
            session_id
        };

        new.endpoints = new.get_endpoints().await.ok()?;

        Some(new)
    }

    async fn send_get<T: DeserializeOwned>(&self, call: &ApiCall<'_>) -> Result<T> {
        match (*CLIENT).send(surf::get(call.uri(self))).await {
            Ok(mut resp) => {
                resp
                    .body_bytes()
                    .await
                    .and_then(|bytes| {
                        serde_json::from_slice::<SynResponse<T>>(&bytes)
                            .map_err(surf::Error::from)
                    })
                    .map_err(|e| SynError::HttpError(e))
                    .and_then(|ok| ok.into())
            },
            Err(surf_err) => Err(SynError::HttpError(surf_err))
        }
    }

    async fn get_endpoints(&self) -> Result<HashMap<String, ApiEndpoint>> {
        let query_endpoint = ApiCall::query();
        let resp = self
            .send_get::<HashMap<String, ApiEndpoint>>(&query_endpoint)
            .await?;

        Ok(resp)
    }

    pub async fn call<T: DeserializeOwned>(&self, endpoint: impl AsRef<str>, parameters: impl IntoParameters<'_>, version: Option<u32>) -> Result<T> {
        let mut parameters = parameters.into_parameters().borrow().clone();
        if let Some(ref sid) = self.session_id {
            parameters.insert("_sid", sid);
        }
        match self.endpoints.get(endpoint.as_ref()) {
            None => {
                panic!("no such endpoint")
            },
            Some(ApiEndpoint {
                request_format: _,
                max_version, min_version, path
            }) => {
                let version = match version {
                    Some(v) => if v < *min_version || v > *max_version {
                        panic!("Version {} out of supported range for endpoint {}.", v, endpoint.as_ref());
                    } else {
                        v
                    },
                    None => *min_version
                }.to_string();
                self.send_get(&ApiCall {
                    endpoint: endpoint.as_ref().to_string(),
                    path: path.to_string(),
                    version,
                    parameters
                }).await
            }
        }
    }
}

pub trait ServerOption: Sized {
    type Output;
    fn require(self) -> NuResult<Self::Output>;
}

impl ServerOption for Option<Server> {
    type Output = Server;

    fn require(self) -> NuResult<Server> {
        match self {
            None => {
                Err(nu_plugin::LabeledError {
                    label: "Not logged in".to_string(),
                    msg: "Please run `dsn login <server-name>` first.".to_string(),
                    span: None
                })
            },
            Some(server) => Ok(server)
        }
    }
}

struct ApiCall<'a> {
    endpoint: String,
    path: String,
    version: String,
    parameters: HashMap<&'a str, &'a str>
}

impl ApiCall<'_> {
    fn query() -> Self {
        let mut parameters = HashMap::new();
        parameters.insert("method", "query");
        Self {
            endpoint: "SYNO.API.Info".to_string(),
            path: "entry.cgi".to_string(),
            version: "1".to_string(),
            parameters
        }
    }

    fn uri(&self, server: &Server) -> String {
        format!(
            "https://{}/webapi/{}?api={}&version={}{}",
            server.name,
            self.path,
            self.endpoint,
            self.version,
            if self.parameters.is_empty() {
                "".to_string()
            } else {
                format!(
                    "&{}",
                    self
                        .parameters
                        .iter()
                        .map(|p| format!("{}={}", p.0, p.1))
                        .collect::<Vec<_>>()
                        .join("&")
                )
            }
        )
    }
}
