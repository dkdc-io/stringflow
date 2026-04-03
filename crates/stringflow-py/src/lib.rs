use pyo3::exceptions::{PyConnectionError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;

fn to_py_err(e: stringflow::Error) -> PyErr {
    match &e {
        stringflow::Error::Unavailable(_) => PyErr::new::<PyConnectionError, _>(e.to_string()),
        stringflow::Error::RequestFailed(_) => PyErr::new::<PyRuntimeError, _>(e.to_string()),
        stringflow::Error::EmptyResponse => PyErr::new::<PyRuntimeError, _>(e.to_string()),
    }
}

// -- Chat ---------------------------------------------------------------------

#[pyfunction]
#[pyo3(signature = (base_url, messages, wire_format="messages", model=None, max_tokens=None, auth_bearer=None, auth_header=None, auth_value=None))]
fn chat(
    base_url: &str,
    messages: Vec<(String, String)>,
    wire_format: &str,
    model: Option<String>,
    max_tokens: Option<u32>,
    auth_bearer: Option<String>,
    auth_header: Option<String>,
    auth_value: Option<String>,
) -> PyResult<String> {
    let config = build_config(
        base_url,
        wire_format,
        model,
        max_tokens,
        auth_bearer,
        auth_header,
        auth_value,
    )?;
    let msgs = to_chat_messages(messages);
    stringflow::chat(&config, &msgs).map_err(to_py_err)
}

// -- Health check -------------------------------------------------------------

#[pyfunction]
fn health_check(base_url: &str) -> PyResult<String> {
    let resp = stringflow::health_check_blocking(base_url).map_err(to_py_err)?;
    Ok(resp.status)
}

// -- Helpers ------------------------------------------------------------------

fn parse_wire_format(s: &str) -> PyResult<stringflow::WireFormat> {
    match s {
        "completions" => Ok(stringflow::WireFormat::Completions),
        "responses" => Ok(stringflow::WireFormat::Responses),
        "messages" => Ok(stringflow::WireFormat::Messages),
        _ => Err(PyErr::new::<PyValueError, _>(format!(
            "unknown wire format '{s}', expected: 'completions', 'responses', or 'messages'"
        ))),
    }
}

fn build_config(
    base_url: &str,
    wire_format: &str,
    model: Option<String>,
    max_tokens: Option<u32>,
    auth_bearer: Option<String>,
    auth_header: Option<String>,
    auth_value: Option<String>,
) -> PyResult<stringflow::ProviderConfig> {
    let auth = if let Some(token) = auth_bearer {
        stringflow::AuthConfig::Bearer(token)
    } else if let (Some(header), Some(value)) = (auth_header, auth_value) {
        stringflow::AuthConfig::ApiKey { header, value }
    } else {
        stringflow::AuthConfig::None
    };

    Ok(stringflow::ProviderConfig {
        name: "python".to_string(),
        base_url: base_url.to_string(),
        wire_format: parse_wire_format(wire_format)?,
        auth,
        model,
        max_tokens,
    })
}

fn to_chat_messages(messages: Vec<(String, String)>) -> Vec<stringflow::ChatMessage> {
    messages
        .into_iter()
        .map(|(role, content)| stringflow::ChatMessage { role, content })
        .collect()
}

// -- Module -------------------------------------------------------------------

#[pymodule]
mod core {
    use super::*;

    #[pymodule_init]
    fn module_init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(chat, m)?)?;
        m.add_function(wrap_pyfunction!(health_check, m)?)?;
        Ok(())
    }
}
