use crate::domain::marshaling::{
    RhaiNativeHandle, RhaiSingleton, dynamic_to_engine_value, engine_value_to_dynamic,
};
use engine::{Capability, CapabilitySet, EngineError, EngineValue, HostMethodFn, HostObject};
use rhai::{Dynamic, Engine, EvalAltResult, Position};
use std::sync::Arc;

/// Modular dispatcher registering host object methods and accessors into the Rhai Engine (C-56).
pub struct HostDispatcher;

impl HostDispatcher {
    /// Registers a host object into the engine, verifying capability constraints.
    pub fn register_host_object(
        engine: &mut Engine,
        object: &HostObject,
        capabilities: &CapabilitySet,
    ) -> Result<(), EngineError> {
        let cap = object.required_capability();
        let has_cap = cap.is_none_or(|c| capabilities.contains(c));
        let obj_name = object.name().as_str();

        if object.is_singleton() {
            Self::register_singleton(engine, obj_name, object, has_cap, cap);
        } else {
            Self::register_instance(engine, object, has_cap, cap);
        }

        Ok(())
    }

    fn register_singleton(
        engine: &mut Engine,
        obj_name: &str,
        object: &HostObject,
        has_cap: bool,
        required_cap: Option<Capability>,
    ) {
        for (method_id, method_fn) in object.methods() {
            let m_name = method_id.as_str();
            let exp = obj_name.to_string();
            let m = Arc::clone(method_fn);

            if !has_cap {
                let err_cap = required_cap.unwrap();
                let exp_clone = exp.clone();
                engine.register_fn(
                    m_name,
                    move |s: &mut RhaiSingleton| -> Result<Dynamic, Box<EvalAltResult>> {
                        Self::check_singleton_permission(s, &exp_clone, err_cap)
                    },
                );
                let exp_clone = exp.clone();
                engine.register_fn(
                    m_name,
                    move |s: &mut RhaiSingleton,
                          _a1: Dynamic|
                          -> Result<Dynamic, Box<EvalAltResult>> {
                        Self::check_singleton_permission(s, &exp_clone, err_cap)
                    },
                );
                let exp_clone = exp.clone();
                engine.register_fn(
                    m_name,
                    move |s: &mut RhaiSingleton,
                          _a1: Dynamic,
                          _a2: Dynamic|
                          -> Result<Dynamic, Box<EvalAltResult>> {
                        Self::check_singleton_permission(s, &exp_clone, err_cap)
                    },
                );
                let exp_clone = exp.clone();
                engine.register_fn(
                    m_name,
                    move |s: &mut RhaiSingleton,
                          _a1: Dynamic,
                          _a2: Dynamic,
                          _a3: Dynamic,
                          _a4: Dynamic,
                          _a5: Dynamic|
                          -> Result<Dynamic, Box<EvalAltResult>> {
                        Self::check_singleton_permission(s, &exp_clone, err_cap)
                    },
                );
                let exp_clone = exp.clone();
                engine.register_fn(
                    m_name,
                    move |s: &mut RhaiSingleton,
                          _a1: Dynamic,
                          _a2: Dynamic,
                          _a3: Dynamic,
                          _a4: Dynamic,
                          _a5: Dynamic,
                          _a6: Dynamic|
                          -> Result<Dynamic, Box<EvalAltResult>> {
                        Self::check_singleton_permission(s, &exp_clone, err_cap)
                    },
                );
                let exp_clone = exp.clone();
                engine.register_fn(
                    m_name,
                    move |s: &mut RhaiSingleton,
                          _a1: Dynamic,
                          _a2: Dynamic,
                          _a3: Dynamic,
                          _a4: Dynamic,
                          _a5: Dynamic,
                          _a6: Dynamic,
                          _a7: Dynamic|
                          -> Result<Dynamic, Box<EvalAltResult>> {
                        Self::check_singleton_permission(s, &exp_clone, err_cap)
                    },
                );
            } else {
                // Arity 0
                let exp_clone = exp.clone();
                let m_clone = Arc::clone(&m);
                engine.register_fn(
                    m_name,
                    move |s: &mut RhaiSingleton| -> Result<Dynamic, Box<EvalAltResult>> {
                        Self::check_receiver(s, &exp_clone)?;
                        Self::dispatch_call(None, &m_clone, &[])
                    },
                );

                // Arity 1
                let exp_clone = exp.clone();
                let m_clone = Arc::clone(&m);
                engine.register_fn(
                    m_name,
                    move |s: &mut RhaiSingleton,
                          a1: Dynamic|
                          -> Result<Dynamic, Box<EvalAltResult>> {
                        Self::check_receiver(s, &exp_clone)?;
                        let v1 = dynamic_to_engine_value(&a1).map_err(Self::to_eval_err)?;
                        Self::dispatch_call(None, &m_clone, &[v1])
                    },
                );

                // Arity 2
                let exp_clone = exp.clone();
                let m_clone = Arc::clone(&m);
                engine.register_fn(
                    m_name,
                    move |s: &mut RhaiSingleton,
                          a1: Dynamic,
                          a2: Dynamic|
                          -> Result<Dynamic, Box<EvalAltResult>> {
                        Self::check_receiver(s, &exp_clone)?;
                        let v1 = dynamic_to_engine_value(&a1).map_err(Self::to_eval_err)?;
                        let v2 = dynamic_to_engine_value(&a2).map_err(Self::to_eval_err)?;
                        Self::dispatch_call(None, &m_clone, &[v1, v2])
                    },
                );

                // Arity 5
                let exp_clone = exp.clone();
                let m_clone = Arc::clone(&m);
                engine.register_fn(
                    m_name,
                    move |s: &mut RhaiSingleton,
                          a1: Dynamic,
                          a2: Dynamic,
                          a3: Dynamic,
                          a4: Dynamic,
                          a5: Dynamic|
                          -> Result<Dynamic, Box<EvalAltResult>> {
                        Self::check_receiver(s, &exp_clone)?;
                        let v1 = dynamic_to_engine_value(&a1).map_err(Self::to_eval_err)?;
                        let v2 = dynamic_to_engine_value(&a2).map_err(Self::to_eval_err)?;
                        let v3 = dynamic_to_engine_value(&a3).map_err(Self::to_eval_err)?;
                        let v4 = dynamic_to_engine_value(&a4).map_err(Self::to_eval_err)?;
                        let v5 = dynamic_to_engine_value(&a5).map_err(Self::to_eval_err)?;
                        Self::dispatch_call(None, &m_clone, &[v1, v2, v3, v4, v5])
                    },
                );

                // Arity 6 (pushBorder: x, y, w, h, border_w, color)
                let exp_clone = exp.clone();
                let m_clone = Arc::clone(&m);
                engine.register_fn(
                    m_name,
                    move |s: &mut RhaiSingleton,
                          a1: Dynamic,
                          a2: Dynamic,
                          a3: Dynamic,
                          a4: Dynamic,
                          a5: Dynamic,
                          a6: Dynamic|
                          -> Result<Dynamic, Box<EvalAltResult>> {
                        Self::check_receiver(s, &exp_clone)?;
                        let v1 = dynamic_to_engine_value(&a1).map_err(Self::to_eval_err)?;
                        let v2 = dynamic_to_engine_value(&a2).map_err(Self::to_eval_err)?;
                        let v3 = dynamic_to_engine_value(&a3).map_err(Self::to_eval_err)?;
                        let v4 = dynamic_to_engine_value(&a4).map_err(Self::to_eval_err)?;
                        let v5 = dynamic_to_engine_value(&a5).map_err(Self::to_eval_err)?;
                        let v6 = dynamic_to_engine_value(&a6).map_err(Self::to_eval_err)?;
                        Self::dispatch_call(None, &m_clone, &[v1, v2, v3, v4, v5, v6])
                    },
                );

                // Arity 7 (pushText: text, x, y, w, h, font_size, color)
                let exp_clone = exp.clone();
                let m_clone = Arc::clone(&m);
                engine.register_fn(
                    m_name,
                    move |s: &mut RhaiSingleton,
                          a1: Dynamic,
                          a2: Dynamic,
                          a3: Dynamic,
                          a4: Dynamic,
                          a5: Dynamic,
                          a6: Dynamic,
                          a7: Dynamic|
                          -> Result<Dynamic, Box<EvalAltResult>> {
                        Self::check_receiver(s, &exp_clone)?;
                        let v1 = dynamic_to_engine_value(&a1).map_err(Self::to_eval_err)?;
                        let v2 = dynamic_to_engine_value(&a2).map_err(Self::to_eval_err)?;
                        let v3 = dynamic_to_engine_value(&a3).map_err(Self::to_eval_err)?;
                        let v4 = dynamic_to_engine_value(&a4).map_err(Self::to_eval_err)?;
                        let v5 = dynamic_to_engine_value(&a5).map_err(Self::to_eval_err)?;
                        let v6 = dynamic_to_engine_value(&a6).map_err(Self::to_eval_err)?;
                        let v7 = dynamic_to_engine_value(&a7).map_err(Self::to_eval_err)?;
                        Self::dispatch_call(None, &m_clone, &[v1, v2, v3, v4, v5, v6, v7])
                    },
                );
            }
        }
    }

    fn register_instance(
        engine: &mut Engine,
        object: &HostObject,
        has_cap: bool,
        required_cap: Option<Capability>,
    ) {
        for (method_id, method_fn) in object.methods() {
            let m_name = method_id.as_str();
            let m = Arc::clone(method_fn);

            if !has_cap {
                let err_cap = required_cap.unwrap();
                engine.register_fn(
                    m_name,
                    move |_h: &mut RhaiNativeHandle| -> Result<Dynamic, Box<EvalAltResult>> {
                        Err(EvalAltResult::ErrorRuntime(
                            format!("PermissionDenied: {err_cap:?}").into(),
                            Position::NONE,
                        )
                        .into())
                    },
                );
                engine.register_fn(
                    m_name,
                    move |_h: &mut RhaiNativeHandle,
                          _a1: Dynamic|
                          -> Result<Dynamic, Box<EvalAltResult>> {
                        Err(EvalAltResult::ErrorRuntime(
                            format!("PermissionDenied: {err_cap:?}").into(),
                            Position::NONE,
                        )
                        .into())
                    },
                );
            } else {
                let m_clone = Arc::clone(&m);
                engine.register_fn(
                    m_name,
                    move |h: &mut RhaiNativeHandle| -> Result<Dynamic, Box<EvalAltResult>> {
                        let target = EngineValue::Handle(Arc::clone(h.inner()));
                        Self::dispatch_call(Some(&target), &m_clone, &[])
                    },
                );

                let m_clone = Arc::clone(&m);
                engine.register_fn(
                    m_name,
                    move |h: &mut RhaiNativeHandle,
                          a1: Dynamic|
                          -> Result<Dynamic, Box<EvalAltResult>> {
                        let target = EngineValue::Handle(Arc::clone(h.inner()));
                        let v1 = dynamic_to_engine_value(&a1).map_err(Self::to_eval_err)?;
                        Self::dispatch_call(Some(&target), &m_clone, &[v1])
                    },
                );
            }
        }

        for (prop_id, getter_fn, setter_fn) in object.properties() {
            let p_name = prop_id.as_str();
            let g = Arc::clone(getter_fn);
            if let Some(s) = setter_fn {
                let s = Arc::clone(s);
                engine.register_get_set(
                    p_name,
                    move |h: &mut RhaiNativeHandle| -> Dynamic {
                        let target = EngineValue::Handle(Arc::clone(h.inner()));
                        g(Some(&target))
                            .map(engine_value_to_dynamic)
                            .unwrap_or(Dynamic::UNIT)
                    },
                    move |h: &mut RhaiNativeHandle, val: Dynamic| {
                        let target = EngineValue::Handle(Arc::clone(h.inner()));
                        if let Ok(v) = dynamic_to_engine_value(&val) {
                            let _ = s(Some(&target), v);
                        }
                    },
                );
            } else {
                engine.register_get(p_name, move |h: &mut RhaiNativeHandle| -> Dynamic {
                    let target = EngineValue::Handle(Arc::clone(h.inner()));
                    g(Some(&target))
                        .map(engine_value_to_dynamic)
                        .unwrap_or(Dynamic::UNIT)
                });
            }
        }
    }

    fn check_singleton_permission(
        s: &RhaiSingleton,
        expected: &str,
        cap: Capability,
    ) -> Result<Dynamic, Box<EvalAltResult>> {
        if s.name() == expected {
            Err(EvalAltResult::ErrorRuntime(
                format!("PermissionDenied: {cap:?}").into(),
                Position::NONE,
            )
            .into())
        } else {
            Err("Receiver mismatch".into())
        }
    }

    fn check_receiver(s: &RhaiSingleton, expected: &str) -> Result<(), Box<EvalAltResult>> {
        if s.name() == expected {
            Ok(())
        } else {
            Err("Receiver mismatch".into())
        }
    }

    fn dispatch_call(
        target: Option<&EngineValue>,
        method: &HostMethodFn,
        args: &[EngineValue],
    ) -> Result<Dynamic, Box<EvalAltResult>> {
        let res = method(target, args).map_err(Self::to_eval_err)?;
        Ok(engine_value_to_dynamic(res))
    }

    fn to_eval_err(e: impl ToString) -> Box<EvalAltResult> {
        Box::new(EvalAltResult::ErrorRuntime(
            e.to_string().into(),
            Position::NONE,
        ))
    }
}
