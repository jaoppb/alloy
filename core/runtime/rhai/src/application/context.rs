use crate::domain::limits::ExecutionLimits;
use crate::domain::marshaling::{
    RhaiNativeHandle, RhaiSingleton, dynamic_to_engine_value, engine_value_to_dynamic,
};
use engine::{
    CapabilitySet, EngineError, EngineValue, ExecutionContext, HostObject, Identifier, NativeFn,
};
use rhai::{Dynamic, EvalAltResult};
use std::collections::HashMap;
use std::sync::Arc;

/// An isolated Rhai execution context holding its own `rhai::Scope`, its own `rhai::Engine`, and capability permissions.
pub struct RhaiContext {
    capabilities: CapabilitySet,
    scope: rhai::Scope<'static>,
    engine: rhai::Engine,
    functions: HashMap<String, NativeFn>,
    host_objects: HashMap<String, HostObject>,
}

impl RhaiContext {
    /// Creates a new Rhai context with default limits and given capability set.
    #[must_use]
    pub fn new(capabilities: CapabilitySet) -> Self {
        Self::with_limits(capabilities, ExecutionLimits::new())
    }

    /// Creates a new Rhai context with custom execution limits applied to its engine isolate.
    #[must_use]
    pub fn with_limits(capabilities: CapabilitySet, limits: ExecutionLimits) -> Self {
        let mut engine = rhai::Engine::new();
        limits.apply_to(&mut engine);
        engine.register_type_with_name::<RhaiNativeHandle>("NativeHandle");
        engine.register_type_with_name::<RhaiSingleton>("HostSingleton");

        Self {
            capabilities,
            scope: rhai::Scope::new(),
            engine,
            functions: HashMap::new(),
            host_objects: HashMap::new(),
        }
    }

    /// Accesses the underlying Rhai scope immutably.
    #[must_use]
    pub const fn scope(&self) -> &rhai::Scope<'static> {
        &self.scope
    }

    /// Accesses the underlying Rhai scope mutably.
    pub fn scope_mut(&mut self) -> &mut rhai::Scope<'static> {
        &mut self.scope
    }

    /// Accesses the isolated Rhai engine immutably.
    #[must_use]
    pub const fn engine(&self) -> &rhai::Engine {
        &self.engine
    }

    /// Accesses the isolated Rhai engine mutably.
    pub fn engine_mut(&mut self) -> &mut rhai::Engine {
        &mut self.engine
    }

    /// Returns a reference to the registered native host functions.
    #[must_use]
    pub const fn functions(&self) -> &HashMap<String, NativeFn> {
        &self.functions
    }

    /// Returns a reference to registered host objects.
    #[must_use]
    pub const fn host_objects(&self) -> &HashMap<String, HostObject> {
        &self.host_objects
    }

    /// Evaluates script source directly in this context's isolated engine and scope.
    pub fn eval_dynamic(&mut self, script: &str) -> Result<Dynamic, Box<EvalAltResult>> {
        self.engine.eval_with_scope(&mut self.scope, script)
    }

    /// Evaluates compiled AST in this context's isolated engine and scope.
    pub fn eval_ast<T: Clone + Send + Sync + 'static>(
        &mut self,
        ast: &rhai::AST,
    ) -> Result<T, Box<EvalAltResult>> {
        self.engine.eval_ast_with_scope(&mut self.scope, ast)
    }
}

impl ExecutionContext for RhaiContext {
    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    fn register_host_object(&mut self, object: HostObject) -> Result<(), EngineError> {
        let cap = object.required_capability();
        let has_cap = cap.is_none_or(|c| self.capabilities.contains(c));
        let obj_name = object.name().as_str().to_string();

        if object.is_singleton() {
            self.scope
                .set_value(object.name().as_str(), RhaiSingleton(obj_name.clone()));

            for (method_id, method_fn) in object.methods() {
                let m_name = method_id.as_str();
                let expected = obj_name.clone();
                let m = Arc::clone(method_fn);

                if !has_cap {
                    let err_cap = cap.unwrap();
                    let exp = expected.clone();
                    self.engine.register_fn(
                        m_name,
                        move |s: &mut RhaiSingleton| -> Result<Dynamic, Box<EvalAltResult>> {
                            if s.0 == exp {
                                Err(EvalAltResult::ErrorRuntime(
                                    format!("PermissionDenied: {err_cap:?}").into(),
                                    rhai::Position::NONE,
                                )
                                .into())
                            } else {
                                Err("Receiver mismatch".into())
                            }
                        },
                    );
                    let exp = expected.clone();
                    self.engine.register_fn(
                        m_name,
                        move |s: &mut RhaiSingleton,
                              _a1: Dynamic|
                              -> Result<Dynamic, Box<EvalAltResult>> {
                            if s.0 == exp {
                                Err(EvalAltResult::ErrorRuntime(
                                    format!("PermissionDenied: {err_cap:?}").into(),
                                    rhai::Position::NONE,
                                )
                                .into())
                            } else {
                                Err("Receiver mismatch".into())
                            }
                        },
                    );
                    let exp = expected.clone();
                    self.engine.register_fn(
                        m_name,
                        move |s: &mut RhaiSingleton,
                              _a1: Dynamic,
                              _a2: Dynamic|
                              -> Result<Dynamic, Box<EvalAltResult>> {
                            if s.0 == exp {
                                Err(EvalAltResult::ErrorRuntime(
                                    format!("PermissionDenied: {err_cap:?}").into(),
                                    rhai::Position::NONE,
                                )
                                .into())
                            } else {
                                Err("Receiver mismatch".into())
                            }
                        },
                    );
                    let exp = expected.clone();
                    self.engine.register_fn(
                        m_name,
                        move |s: &mut RhaiSingleton,
                              _a1: Dynamic,
                              _a2: Dynamic,
                              _a3: Dynamic,
                              _a4: Dynamic,
                              _a5: Dynamic|
                              -> Result<Dynamic, Box<EvalAltResult>> {
                            if s.0 == exp {
                                Err(EvalAltResult::ErrorRuntime(
                                    format!("PermissionDenied: {err_cap:?}").into(),
                                    rhai::Position::NONE,
                                )
                                .into())
                            } else {
                                Err("Receiver mismatch".into())
                            }
                        },
                    );
                } else {
                    // Arity 0
                    let exp = expected.clone();
                    let m_clone = Arc::clone(&m);
                    self.engine.register_fn(
                        m_name,
                        move |s: &mut RhaiSingleton| -> Result<Dynamic, Box<EvalAltResult>> {
                            if s.0 != exp {
                                return Err("Receiver mismatch".into());
                            }
                            let res = m_clone(None, &[]).map_err(|e| {
                                Box::new(EvalAltResult::ErrorRuntime(
                                    e.to_string().into(),
                                    rhai::Position::NONE,
                                ))
                            })?;
                            Ok(engine_value_to_dynamic(res))
                        },
                    );

                    // Arity 1
                    let exp = expected.clone();
                    let m_clone = Arc::clone(&m);
                    self.engine.register_fn(
                        m_name,
                        move |s: &mut RhaiSingleton,
                              a1: Dynamic|
                              -> Result<Dynamic, Box<EvalAltResult>> {
                            if s.0 != exp {
                                return Err("Receiver mismatch".into());
                            }
                            let v1 = dynamic_to_engine_value(&a1).map_err(|e| {
                                Box::new(EvalAltResult::ErrorRuntime(
                                    e.to_string().into(),
                                    rhai::Position::NONE,
                                ))
                            })?;
                            let res = m_clone(None, &[v1]).map_err(|e| {
                                Box::new(EvalAltResult::ErrorRuntime(
                                    e.to_string().into(),
                                    rhai::Position::NONE,
                                ))
                            })?;
                            Ok(engine_value_to_dynamic(res))
                        },
                    );

                    // Arity 2
                    let exp = expected.clone();
                    let m_clone = Arc::clone(&m);
                    self.engine.register_fn(
                        m_name,
                        move |s: &mut RhaiSingleton,
                              a1: Dynamic,
                              a2: Dynamic|
                              -> Result<Dynamic, Box<EvalAltResult>> {
                            if s.0 != exp {
                                return Err("Receiver mismatch".into());
                            }
                            let v1 = dynamic_to_engine_value(&a1).map_err(|e| {
                                Box::new(EvalAltResult::ErrorRuntime(
                                    e.to_string().into(),
                                    rhai::Position::NONE,
                                ))
                            })?;
                            let v2 = dynamic_to_engine_value(&a2).map_err(|e| {
                                Box::new(EvalAltResult::ErrorRuntime(
                                    e.to_string().into(),
                                    rhai::Position::NONE,
                                ))
                            })?;
                            let res = m_clone(None, &[v1, v2]).map_err(|e| {
                                Box::new(EvalAltResult::ErrorRuntime(
                                    e.to_string().into(),
                                    rhai::Position::NONE,
                                ))
                            })?;
                            Ok(engine_value_to_dynamic(res))
                        },
                    );

                    // Arity 5 (for renderer.pushRect)
                    let exp = expected.clone();
                    let m_clone = Arc::clone(&m);
                    self.engine.register_fn(
                        m_name,
                        move |s: &mut RhaiSingleton,
                              a1: Dynamic,
                              a2: Dynamic,
                              a3: Dynamic,
                              a4: Dynamic,
                              a5: Dynamic|
                              -> Result<Dynamic, Box<EvalAltResult>> {
                            if s.0 != exp {
                                return Err("Receiver mismatch".into());
                            }
                            let v1 = dynamic_to_engine_value(&a1).map_err(|e| {
                                Box::new(EvalAltResult::ErrorRuntime(
                                    e.to_string().into(),
                                    rhai::Position::NONE,
                                ))
                            })?;
                            let v2 = dynamic_to_engine_value(&a2).map_err(|e| {
                                Box::new(EvalAltResult::ErrorRuntime(
                                    e.to_string().into(),
                                    rhai::Position::NONE,
                                ))
                            })?;
                            let v3 = dynamic_to_engine_value(&a3).map_err(|e| {
                                Box::new(EvalAltResult::ErrorRuntime(
                                    e.to_string().into(),
                                    rhai::Position::NONE,
                                ))
                            })?;
                            let v4 = dynamic_to_engine_value(&a4).map_err(|e| {
                                Box::new(EvalAltResult::ErrorRuntime(
                                    e.to_string().into(),
                                    rhai::Position::NONE,
                                ))
                            })?;
                            let v5 = dynamic_to_engine_value(&a5).map_err(|e| {
                                Box::new(EvalAltResult::ErrorRuntime(
                                    e.to_string().into(),
                                    rhai::Position::NONE,
                                ))
                            })?;
                            let res = m_clone(None, &[v1, v2, v3, v4, v5]).map_err(|e| {
                                Box::new(EvalAltResult::ErrorRuntime(
                                    e.to_string().into(),
                                    rhai::Position::NONE,
                                ))
                            })?;
                            Ok(engine_value_to_dynamic(res))
                        },
                    );
                }
            }
        } else {
            // Instance methods on RhaiNativeHandle (e.g. Node)
            for (method_id, method_fn) in object.methods() {
                let m_name = method_id.as_str();
                let m = Arc::clone(method_fn);

                if !has_cap {
                    let err_cap = cap.unwrap();
                    self.engine.register_fn(
                        m_name,
                        move |_h: &mut RhaiNativeHandle| -> Result<Dynamic, Box<EvalAltResult>> {
                            Err(EvalAltResult::ErrorRuntime(
                                format!("PermissionDenied: {err_cap:?}").into(),
                                rhai::Position::NONE,
                            )
                            .into())
                        },
                    );
                    self.engine.register_fn(
                        m_name,
                        move |_h: &mut RhaiNativeHandle,
                              _a1: Dynamic|
                              -> Result<Dynamic, Box<EvalAltResult>> {
                            Err(EvalAltResult::ErrorRuntime(
                                format!("PermissionDenied: {err_cap:?}").into(),
                                rhai::Position::NONE,
                            )
                            .into())
                        },
                    );
                } else {
                    let m_clone = Arc::clone(&m);
                    self.engine.register_fn(
                        m_name,
                        move |h: &mut RhaiNativeHandle| -> Result<Dynamic, Box<EvalAltResult>> {
                            let target = EngineValue::Handle(Arc::clone(&h.0));
                            let res = m_clone(Some(&target), &[]).map_err(|e| {
                                Box::new(EvalAltResult::ErrorRuntime(
                                    e.to_string().into(),
                                    rhai::Position::NONE,
                                ))
                            })?;
                            Ok(engine_value_to_dynamic(res))
                        },
                    );

                    let m_clone = Arc::clone(&m);
                    self.engine.register_fn(
                        m_name,
                        move |h: &mut RhaiNativeHandle,
                              a1: Dynamic|
                              -> Result<Dynamic, Box<EvalAltResult>> {
                            let target = EngineValue::Handle(Arc::clone(&h.0));
                            let v1 = dynamic_to_engine_value(&a1).map_err(|e| {
                                Box::new(EvalAltResult::ErrorRuntime(
                                    e.to_string().into(),
                                    rhai::Position::NONE,
                                ))
                            })?;
                            let res = m_clone(Some(&target), &[v1]).map_err(|e| {
                                Box::new(EvalAltResult::ErrorRuntime(
                                    e.to_string().into(),
                                    rhai::Position::NONE,
                                ))
                            })?;
                            Ok(engine_value_to_dynamic(res))
                        },
                    );
                }
            }

            for (prop_id, getter_fn, setter_fn) in object.properties() {
                let p_name = prop_id.as_str();
                let g = Arc::clone(getter_fn);
                if let Some(s) = setter_fn {
                    let s = Arc::clone(s);
                    self.engine.register_get_set(
                        p_name,
                        move |h: &mut RhaiNativeHandle| -> Dynamic {
                            let target = EngineValue::Handle(Arc::clone(&h.0));
                            g(Some(&target))
                                .map(engine_value_to_dynamic)
                                .unwrap_or(Dynamic::UNIT)
                        },
                        move |h: &mut RhaiNativeHandle, val: Dynamic| {
                            let target = EngineValue::Handle(Arc::clone(&h.0));
                            if let Ok(v) = dynamic_to_engine_value(&val) {
                                let _ = s(Some(&target), v);
                            }
                        },
                    );
                } else {
                    self.engine
                        .register_get(p_name, move |h: &mut RhaiNativeHandle| -> Dynamic {
                            let target = EngineValue::Handle(Arc::clone(&h.0));
                            g(Some(&target))
                                .map(engine_value_to_dynamic)
                                .unwrap_or(Dynamic::UNIT)
                        });
                }
            }
        }

        self.host_objects.insert(obj_name, object);
        Ok(())
    }

    fn register_fn(&mut self, name: Identifier, f: NativeFn) -> Result<(), EngineError> {
        self.functions
            .insert(name.as_str().to_string(), Arc::clone(&f));
        Ok(())
    }

    fn set_variable(&mut self, name: Identifier, value: EngineValue) -> Result<(), EngineError> {
        let dyn_val = engine_value_to_dynamic(value);
        self.scope.set_value(name.as_str(), dyn_val);
        Ok(())
    }

    fn get_variable(&self, name: &Identifier) -> Result<Option<EngineValue>, EngineError> {
        let Some(val) = self.scope.get_value::<Dynamic>(name.as_str()) else {
            return Ok(None);
        };
        dynamic_to_engine_value(&val).map(Some)
    }

    fn call_function(
        &mut self,
        name: &Identifier,
        args: &[EngineValue],
    ) -> Result<EngineValue, EngineError> {
        if let Some(f) = self.functions.get(name.as_str()).cloned() {
            return f(self, args);
        }

        if let Some((ns, method)) = name.as_str().split_once('.') {
            if let Some(host_obj) = self.host_objects.get(ns) {
                if let Some(cap) = host_obj.required_capability() {
                    if !self.capabilities.contains(cap) {
                        return Err(EngineError::PermissionDenied(format!("{cap:?}")));
                    }
                }
                for (m_name, m_fn) in host_obj.methods() {
                    if m_name.as_str() == method {
                        return m_fn(None, args);
                    }
                }
            }
        }

        Err(EngineError::FunctionNotFound(name.as_str().to_string()))
    }

    fn reset_scope(&mut self) -> Result<(), EngineError> {
        self.scope.clear();
        for (name, obj) in &self.host_objects {
            if obj.is_singleton() {
                self.scope
                    .set_value(name.as_str(), RhaiSingleton(name.clone()));
            }
        }
        Ok(())
    }
}
