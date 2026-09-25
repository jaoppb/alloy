//! Network bindings for Rhai scripts (Fase M, PRD-009, ADR-0003, ADR-0011).
//!
//! Provides the [`NETWORK_BINDINGS`] manifest and registers capability-guarded
//! native functions for network interception and request policy.
//! Every binding requires [`Capability::NETWORK_FETCH`].

use std::collections::BTreeMap;
use std::sync::Arc;

use engine::{
    Arity, Capability, EngineError, EngineValue, ExecutionContext, FunctionName, NativeFn,
    RuntimeEngine, SubsystemName, VariableName, profiles,
};
use network::{
    AllowAllPolicy, HeaderName, HeaderValue, HttpRequest, NetworkError, PolicyVerdict,
    RequestPolicy, Url,
};
use rhai_runtime::{PanicHookGuard, RhaiContext, RhaiEngine};

/// The manifest of network bindings and their required capabilities.
///
/// Used for capability sweeps (C-06) and fault injection matrices (C-09).
pub const NETWORK_BINDINGS: &[(&str, Capability)] = &[
    ("fetch", Capability::NETWORK_FETCH),
    ("allow", Capability::NETWORK_FETCH),
    ("deny", Capability::NETWORK_FETCH),
    ("rewrite", Capability::NETWORK_FETCH),
    ("header", Capability::NETWORK_FETCH),
];

fn network_error(operation: &str, error_message: impl Into<String>) -> EngineError {
    EngineError::subsystem(SubsystemName::Network, operation, error_message)
}

fn fetch_handler(arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
    let url_arg = arguments
        .first()
        .ok_or_else(|| network_error("fetch", "missing URL argument"))?;
    let url_text = match url_arg {
        EngineValue::Text(text) => text.as_str(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    let parsed_url = Url::parse(url_text)
        .map_err(|error| network_error("fetch", format!("invalid URL: {error}")))?;
    let mut response_map = BTreeMap::new();
    response_map.insert("url".to_owned(), EngineValue::Text(parsed_url.to_string()));
    response_map.insert("status".to_owned(), EngineValue::Int(200));
    response_map.insert("ok".to_owned(), EngineValue::Bool(true));
    response_map.insert("body".to_owned(), EngineValue::Text(String::new()));
    Ok(EngineValue::Map(response_map))
}

#[allow(clippy::unnecessary_wraps)]
fn allow_handler(arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
    let _ = arguments;
    let mut verdict_map = BTreeMap::new();
    verdict_map.insert("verdict".to_owned(), EngineValue::Text("allow".to_owned()));
    Ok(EngineValue::Map(verdict_map))
}

fn deny_handler(arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
    let reason_arg = arguments
        .first()
        .ok_or_else(|| network_error("deny", "missing reason argument"))?;
    let reason_text = match reason_arg {
        EngineValue::Text(text) => text.clone(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    let mut verdict_map = BTreeMap::new();
    verdict_map.insert("verdict".to_owned(), EngineValue::Text("deny".to_owned()));
    verdict_map.insert("reason".to_owned(), EngineValue::Text(reason_text));
    Ok(EngineValue::Map(verdict_map))
}

fn rewrite_handler(arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
    let target_arg = arguments
        .first()
        .ok_or_else(|| network_error("rewrite", "missing target URL argument"))?;
    let target_url = match target_arg {
        EngineValue::Text(text) => text.as_str(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    let parsed_url = Url::parse(target_url)
        .map_err(|error| network_error("rewrite", format!("invalid rewrite URL: {error}")))?;
    let mut verdict_map = BTreeMap::new();
    verdict_map.insert(
        "verdict".to_owned(),
        EngineValue::Text("rewrite".to_owned()),
    );
    verdict_map.insert("url".to_owned(), EngineValue::Text(parsed_url.to_string()));
    Ok(EngineValue::Map(verdict_map))
}

fn header_handler(arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
    let name_arg = arguments
        .first()
        .ok_or_else(|| network_error("header", "missing header name argument"))?;
    let value_arg = arguments
        .get(1)
        .ok_or_else(|| network_error("header", "missing header value argument"))?;
    let name_text = match name_arg {
        EngineValue::Text(text) => text.as_str(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    let value_text = match value_arg {
        EngineValue::Text(text) => text.as_str(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    let header_name = HeaderName::new(name_text)
        .map_err(|error| network_error("header", format!("invalid header name: {error}")))?;
    let header_value = HeaderValue::from_text(value_text)
        .map_err(|error| network_error("header", format!("invalid header value: {error}")))?;
    let mut header_map = BTreeMap::new();
    header_map.insert(
        "name".to_owned(),
        EngineValue::Text(header_name.as_str().to_owned()),
    );
    header_map.insert(
        "value".to_owned(),
        EngineValue::Text(header_value.to_string()),
    );
    Ok(EngineValue::Map(header_map))
}

/// Register network bindings on a Rhai context under capability guards.
pub fn register_net_bindings(context: &mut RhaiContext) -> Result<(), EngineError> {
    let fetch_name = FunctionName::parse("fetch")?;
    let allow_name = FunctionName::parse("allow")?;
    let deny_name = FunctionName::parse("deny")?;
    let rewrite_name = FunctionName::parse("rewrite")?;
    let header_name = FunctionName::parse("header")?;

    let fetch_fn: NativeFn = Arc::new(fetch_handler);
    let allow_fn: NativeFn = Arc::new(allow_handler);
    let deny_fn: NativeFn = Arc::new(deny_handler);
    let rewrite_fn: NativeFn = Arc::new(rewrite_handler);
    let header_fn: NativeFn = Arc::new(header_handler);

    context.register_guarded_binding(
        &fetch_name,
        Arity::exact(1),
        Capability::NETWORK_FETCH,
        fetch_fn,
    )?;
    context.register_guarded_binding(
        &allow_name,
        Arity::exact(1),
        Capability::NETWORK_FETCH,
        allow_fn,
    )?;
    context.register_guarded_binding(
        &deny_name,
        Arity::exact(1),
        Capability::NETWORK_FETCH,
        deny_fn,
    )?;
    context.register_guarded_binding(
        &rewrite_name,
        Arity::exact(1),
        Capability::NETWORK_FETCH,
        rewrite_fn,
    )?;
    context.register_guarded_binding(
        &header_name,
        Arity::exact(2),
        Capability::NETWORK_FETCH,
        header_fn,
    )?;

    Ok(())
}

/// A scriptable request policy running `.rhai` under [`profiles::network_interceptor`].
///
/// Falls back safely to [`AllowAllPolicy`] if the script fails, errors or panics.
pub struct ScriptRequestPolicy {
    engine: RhaiEngine,
    script: String,
    fallback: AllowAllPolicy,
}

impl ScriptRequestPolicy {
    /// Create a new policy with the given Rhai engine and script source.
    #[must_use]
    pub fn new(engine: RhaiEngine, script: impl Into<String>) -> Self {
        Self {
            engine,
            script: script.into(),
            fallback: AllowAllPolicy,
        }
    }

    fn evaluate_script(&self, request: &HttpRequest) -> Result<PolicyVerdict, EngineError> {
        let mut context = self
            .engine
            .create_context(profiles::network_interceptor())?;
        register_net_bindings(&mut context)?;

        let request_var = VariableName::parse("request")?;
        let request_url = EngineValue::Text(request.url().to_string());
        context.set_variable(&request_var, request_url)?;

        let outcome = {
            let _quiet = PanicHookGuard::install();
            self.engine.eval_value(&mut context, &self.script)?
        };

        parse_verdict(outcome, request)
    }
}

fn parse_verdict(value: EngineValue, request: &HttpRequest) -> Result<PolicyVerdict, EngineError> {
    match value {
        EngineValue::Map(map) => parse_verdict_map(&map, request),
        EngineValue::Text(verdict_str) => parse_verdict_string(&verdict_str, request),
        _ => Ok(PolicyVerdict::Allow),
    }
}

fn parse_verdict_map(
    map: &BTreeMap<String, EngineValue>,
    request: &HttpRequest,
) -> Result<PolicyVerdict, EngineError> {
    let verdict = map
        .get("verdict")
        .and_then(|val| match val {
            EngineValue::Text(text) => Some(text.as_str()),
            _ => None,
        })
        .unwrap_or("allow");

    match verdict {
        "deny" => {
            let reason = map
                .get("reason")
                .and_then(|val| match val {
                    EngineValue::Text(text) => Some(text.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "denied by policy".to_owned());
            Ok(PolicyVerdict::Deny { reason })
        }
        "rewrite" => {
            let target = map.get("url").and_then(|val| match val {
                EngineValue::Text(text) => Some(text.as_str()),
                _ => None,
            });
            let Some(new_url_text) = target else {
                return Ok(PolicyVerdict::Allow);
            };
            let parsed = Url::parse(new_url_text)
                .map_err(|error| network_error("rewrite", format!("{error}")))?;
            Ok(PolicyVerdict::Rewrite(request.clone().with_url(parsed)))
        }
        _ => Ok(PolicyVerdict::Allow),
    }
}

fn parse_verdict_string(
    verdict: &str,
    request: &HttpRequest,
) -> Result<PolicyVerdict, EngineError> {
    if verdict == "allow" {
        return Ok(PolicyVerdict::Allow);
    }
    if let Some(reason) = verdict.strip_prefix("deny:") {
        return Ok(PolicyVerdict::Deny {
            reason: reason.to_owned(),
        });
    }
    if let Some(target) = verdict.strip_prefix("rewrite:") {
        let parsed =
            Url::parse(target).map_err(|error| network_error("rewrite", format!("{error}")))?;
        return Ok(PolicyVerdict::Rewrite(request.clone().with_url(parsed)));
    }
    Ok(PolicyVerdict::Allow)
}

impl RequestPolicy for ScriptRequestPolicy {
    fn decide(&self, request: &HttpRequest) -> Result<PolicyVerdict, NetworkError> {
        match self.evaluate_script(request) {
            Ok(verdict) => Ok(verdict),
            Err(error) => {
                tracing::warn!("script request policy error: {error}; using fallback");
                self.fallback.decide(request)
            }
        }
    }
}
