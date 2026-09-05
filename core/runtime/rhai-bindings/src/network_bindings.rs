//! Network bindings for Rhai scripts (Fase M, PRD-009, ADR-0003, ADR-0010, ADR-0011).
//!
//! Provides the [`NETWORK_BINDINGS`] manifest and registers capability-guarded
//! native functions for network interception and request policy.
//! Every binding requires [`Capability::NETWORK_FETCH`].

use std::collections::BTreeMap;
use std::sync::Arc;

use engine::{
    Arity, Capability, EngineError, EngineValue, ExecutionContext, NativeFn, RuntimeEngine,
    SubsystemName, VariableName, profiles,
};
use network::{
    AllowAllPolicy, HeaderName, HeaderValue, HttpRequest, NetworkError, PolicyVerdict,
    RequestPolicy, Url,
};
use rhai_runtime::{
    GuardedBinding, PanicHookGuard, RhaiCompiledScript, RhaiContext, RhaiEngine,
    install_guarded_table, run_with_fallback,
};

use crate::DEFAULT_NETWORK_SCRIPT;

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
    let url_argument = arguments
        .first()
        .ok_or_else(|| network_error("fetch", "missing URL argument"))?;
    let url_text = match url_argument {
        EngineValue::Text(text) => text.as_str(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    let parsed_url = Url::parse(url_text)
        .map_err(|error| network_error("fetch", format!("invalid URL: {error}")))?;
    let mut response_map = BTreeMap::new();
    let url_value = EngineValue::Text(parsed_url.to_string());
    response_map.insert("url".to_owned(), url_value);
    response_map.insert("status".to_owned(), EngineValue::Int(200));
    response_map.insert("ok".to_owned(), EngineValue::Bool(true));
    response_map.insert("body".to_owned(), EngineValue::Text(String::new()));
    Ok(EngineValue::Map(response_map))
}

#[allow(clippy::unnecessary_wraps)]
fn allow_handler(_arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
    let mut verdict_map = BTreeMap::new();
    verdict_map.insert("verdict".to_owned(), EngineValue::Text("allow".to_owned()));
    Ok(EngineValue::Map(verdict_map))
}

fn deny_handler(arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
    let reason_argument = arguments
        .first()
        .ok_or_else(|| network_error("deny", "missing reason argument"))?;
    let reason_text = match reason_argument {
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
    let target_argument = arguments
        .first()
        .ok_or_else(|| network_error("rewrite", "missing target URL argument"))?;
    let target_url = match target_argument {
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
    let parsed_url_text = parsed_url.to_string();
    verdict_map.insert("url".to_owned(), EngineValue::Text(parsed_url_text));
    Ok(EngineValue::Map(verdict_map))
}

fn header_handler(arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
    let name_argument = arguments
        .first()
        .ok_or_else(|| network_error("header", "missing header name argument"))?;
    let value_argument = arguments
        .get(1)
        .ok_or_else(|| network_error("header", "missing header value argument"))?;
    let name_text = match name_argument {
        EngineValue::Text(text) => text.as_str(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    let value_text = match value_argument {
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
    let header_name_text = header_name.as_str().to_owned();
    header_map.insert("name".to_owned(), EngineValue::Text(header_name_text));
    let header_value_text = header_value.to_string();
    header_map.insert("value".to_owned(), EngineValue::Text(header_value_text));
    Ok(EngineValue::Map(header_map))
}

/// Builds the table of guarded network bindings for [`install_guarded_table`].
#[must_use]
pub fn network_guarded_bindings() -> [GuardedBinding; 5] {
    let fetch_handler_fn: NativeFn = Arc::new(fetch_handler);
    let allow_handler_fn: NativeFn = Arc::new(allow_handler);
    let deny_handler_fn: NativeFn = Arc::new(deny_handler);
    let rewrite_handler_fn: NativeFn = Arc::new(rewrite_handler);
    let header_handler_fn: NativeFn = Arc::new(header_handler);

    [
        GuardedBinding::new(
            "fetch",
            Arity::exact(1),
            Capability::NETWORK_FETCH,
            fetch_handler_fn,
        ),
        GuardedBinding::new(
            "allow",
            Arity::exact(1),
            Capability::NETWORK_FETCH,
            allow_handler_fn,
        ),
        GuardedBinding::new(
            "deny",
            Arity::exact(1),
            Capability::NETWORK_FETCH,
            deny_handler_fn,
        ),
        GuardedBinding::new(
            "rewrite",
            Arity::exact(1),
            Capability::NETWORK_FETCH,
            rewrite_handler_fn,
        ),
        GuardedBinding::new(
            "header",
            Arity::exact(2),
            Capability::NETWORK_FETCH,
            header_handler_fn,
        ),
    ]
}

/// Register network bindings on a Rhai context under capability guards.
pub fn register_network_bindings(context: &mut RhaiContext) -> Result<(), EngineError> {
    let bindings = network_guarded_bindings();
    install_guarded_table(context, &bindings)
}

/// Deprecated alias for [`register_network_bindings`].
#[deprecated(
    since = "0.5.0",
    note = "use register_network_bindings to avoid abbreviation"
)]
pub fn register_net_bindings(context: &mut RhaiContext) -> Result<(), EngineError> {
    register_network_bindings(context)
}

/// A scriptable request policy running `.rhai` under [`profiles::network_interceptor`].
///
/// Falls back safely via 3-tier fallback to [`DEFAULT_NETWORK_SCRIPT`] and [`AllowAllPolicy`]
/// whenever a script compilation, evaluation, limit, or panic occurs (C-09).
pub struct ScriptRequestPolicy {
    engine: RhaiEngine,
    primary_script: Option<RhaiCompiledScript>,
    default_script: Option<RhaiCompiledScript>,
    fallback: AllowAllPolicy,
}

impl ScriptRequestPolicy {
    /// Create a new policy with the given Rhai engine and script source.
    #[must_use]
    pub fn new(engine: RhaiEngine, script_source: impl Into<String>) -> Self {
        let script_text = script_source.into();
        let primary_script = engine.compile(&script_text).ok();
        let default_script = engine.compile(DEFAULT_NETWORK_SCRIPT).ok();
        Self {
            engine,
            primary_script,
            default_script,
            fallback: AllowAllPolicy,
        }
    }

    fn evaluate_request(
        &self,
        request: &HttpRequest,
        compiled: Option<&RhaiCompiledScript>,
    ) -> Result<(PolicyVerdict, EngineValue), EngineError> {
        let Some(script) = compiled else {
            return Err(network_error(
                "evaluate_request",
                "script compilation failed",
            ));
        };
        let mut context = self
            .engine
            .create_context(profiles::network_interceptor())?;
        register_network_bindings(&mut context)?;

        let request_variable = VariableName::parse("request")?;
        let request_url = EngineValue::Text(request.url().to_string());
        context.set_variable(&request_variable, request_url)?;

        let outcome = {
            let _quiet = PanicHookGuard::install();
            self.engine.eval_compiled_value(&mut context, script)?
        };

        let verdict = parse_verdict(outcome.clone(), request)?;
        Ok((verdict, outcome))
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
            let rewritten_request = request.clone().with_url(parsed);
            Ok(PolicyVerdict::Rewrite(rewritten_request))
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
        let rewritten_request = request.clone().with_url(parsed);
        return Ok(PolicyVerdict::Rewrite(rewritten_request));
    }
    Ok(PolicyVerdict::Allow)
}

impl RequestPolicy for ScriptRequestPolicy {
    fn decide(&self, request: &HttpRequest) -> Result<PolicyVerdict, NetworkError> {
        let (verdict, _) = run_with_fallback(
            None,
            || self.evaluate_request(request, self.primary_script.as_ref()),
            || {
                self.evaluate_request(request, self.default_script.as_ref())
                    .map(|(verdict, _)| verdict)
            },
            || {
                self.fallback
                    .decide(request)
                    .unwrap_or(PolicyVerdict::Allow)
            },
        );
        Ok(verdict)
    }
}
