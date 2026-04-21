use crate::ast::{AssertKind, BinaryOp, Expr, MatchPattern, Program, Stmt, UnaryOp};
use crate::runtime::RuntimeState;
use crate::standard_lib;
use crate::type_system::{NativeCallback, ProcedureValue, Value};
use crate::{CorvoError, CorvoResult};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum ControlFlow {
    Continue,
    Break,
    Terminate,
}

pub struct Evaluator {
    terminate_requested: bool,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            terminate_requested: false,
        }
    }

    pub fn run(&mut self, program: &Program, state: &mut RuntimeState) -> CorvoResult<()> {
        for stmt in &program.statements {
            self.exec_stmt(stmt, state)?;
            if self.terminate_requested {
                break;
            }
        }
        Ok(())
    }

    fn exec_stmt(&mut self, stmt: &Stmt, state: &mut RuntimeState) -> CorvoResult<()> {
        match stmt {
            Stmt::PrepBlock { body } => {
                // If every static that this prep block would set is already
                // present in state (baked in at compile time), skip the entire
                // block.  This prevents re-running side effects such as
                // `fs.read` calls when the compiled binary is executed after
                // the source files have been removed.
                let mut has_any_static = false;
                let mut all_statics_preset = true;
                for s in body {
                    if let Stmt::StaticSet { name, .. } = s {
                        has_any_static = true;
                        if !state.has_static(name) {
                            all_statics_preset = false;
                            break;
                        }
                    }
                }
                if has_any_static && all_statics_preset {
                    return Ok(());
                }

                // Execute the prep block body to set static variables, then discard
                // any runtime vars created in it. Vars in a prep block are scoped
                // to the block and are not available in the rest of the program.
                self.execute_block(body, state)?;
                state.clear_vars();
                Ok(())
            }
            Stmt::StaticSet { name, value } => {
                // Skip if already set (baked in from compilation)
                if state.has_static(name) {
                    return Ok(());
                }
                let val = self.eval_expr(value, state)?;
                state.static_set(name.clone(), val);
                Ok(())
            }
            Stmt::VarSet { name, value } => {
                let val = self.eval_expr(value, state)?;
                state.var_set(name.clone(), val);
                Ok(())
            }
            Stmt::VarIndexSet { name, index, value } => {
                let current = state.var_get(name)?;
                let index_val = self.eval_expr(index, state)?;
                let new_val = self.eval_expr(value, state)?;
                let updated = match (&current, &index_val) {
                    (Value::Map(map), Value::String(key)) => {
                        let mut new_map = map.clone();
                        new_map.insert(key.clone(), new_val);
                        Value::Map(new_map)
                    }
                    (Value::List(list), Value::Number(idx)) => {
                        let idx = *idx as usize;
                        if idx >= list.len() {
                            return Err(CorvoError::runtime(format!(
                                "Index {} out of bounds",
                                idx
                            )));
                        }
                        let mut new_list = list.clone();
                        new_list[idx] = new_val;
                        Value::List(new_list)
                    }
                    _ => {
                        return Err(CorvoError::r#type(
                            "Index assignment requires a map with a string key or a list with a number index",
                        ))
                    }
                };
                state.var_set(name.clone(), updated);
                Ok(())
            }
            Stmt::VarAddAssign { name, value } => {
                let current = state.var_get(name)?;
                let rhs = self.eval_expr(value, state)?;
                let updated = match (current, rhs) {
                    (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
                    (Value::String(a), Value::String(b)) => Value::String(format!("{}{}", a, b)),
                    _ => return Err(CorvoError::r#type("+= requires two numbers or two strings")),
                };
                state.var_set(name.clone(), updated);
                Ok(())
            }
            Stmt::VarSubAssign { name, value } => {
                let current = state.var_get(name)?;
                let rhs = self.eval_expr(value, state)?;
                let updated = match (current, rhs) {
                    (Value::Number(a), Value::Number(b)) => Value::Number(a - b),
                    (Value::String(a), Value::String(b)) => {
                        Value::String(a.replace(b.as_str(), ""))
                    }
                    _ => return Err(CorvoError::r#type("-= requires two numbers or two strings")),
                };
                state.var_set(name.clone(), updated);
                Ok(())
            }
            Stmt::VarOrAssign { name, candidates } => {
                for candidate in candidates {
                    if let Ok(val) = self.eval_expr(candidate, state) {
                        if val.is_truthy() {
                            state.var_set(name.clone(), val);
                            return Ok(());
                        }
                    }
                }
                Err(CorvoError::runtime(format!(
                    "No truthy value found in or= candidates for variable '{}'",
                    name
                )))
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.eval_expr(condition, state)?;
                if cond_val.as_bool().unwrap_or(false) {
                    self.execute_block(then_branch, state)
                } else {
                    self.execute_block(else_branch, state)
                }
            }
            Stmt::ExprStmt { expr } => {
                // Intercept special method calls that need mutable state access.
                if let Expr::MethodCall {
                    target,
                    method,
                    args,
                    ..
                } = expr
                {
                    // procedure.call(...) runs the procedure body with &mut state.
                    if method == "call" {
                        let target_val = self.eval_expr(target, state)?;
                        if let Value::Procedure(proc) = target_val {
                            return self.exec_procedure_call(&proc, args, state);
                        }
                        if let Value::NativeProcedure { callback: proc, .. } = target_val {
                            let evaluated_args = args
                                .iter()
                                .map(|a| self.eval_expr(a, state))
                                .collect::<Result<Vec<_>, _>>()?;
                            return (proc)(&evaluated_args, state).map(|_| ());
                        }
                    }
                    // @map_var.set(key, value) → mutate the variable in place,
                    // equivalent to @map_var["key"] = value.
                    if method == "set" {
                        if let Expr::VarGet { name } = target.as_ref() {
                            let target_val = self.eval_expr(target, state)?;
                            if matches!(target_val, Value::Map(_)) {
                                let evaluated_args = args
                                    .iter()
                                    .map(|a| self.eval_expr(a, state))
                                    .collect::<CorvoResult<Vec<_>>>()?;
                                let mut all_args = vec![target_val];
                                all_args.extend(evaluated_args);
                                let new_map = standard_lib::call(
                                    "map.set",
                                    &all_args,
                                    &std::collections::HashMap::new(),
                                    state,
                                )?;
                                state.var_set(name.clone(), new_map);
                                return Ok(());
                            }
                        }
                    }
                }
                self.eval_expr(expr, state)?;
                Ok(())
            }
            Stmt::TryBlock { body, fallbacks } => {
                let result = self.execute_block(body, state);

                if result.is_err() {
                    for fallback in fallbacks {
                        if self.execute_block(&fallback.body, state).is_ok() {
                            return Ok(());
                        }
                    }
                }
                result
            }
            Stmt::Loop { body } => {
                while !self.terminate_requested {
                    if let Err(e) = self.execute_block(body, state) {
                        match e {
                            CorvoError::Runtime { .. } => continue,
                            _ => return Err(e),
                        }
                    }
                }
                self.terminate_requested = false;
                Ok(())
            }
            Stmt::Browse {
                iterable,
                key,
                value,
                body,
            } => {
                let collection = self.eval_expr(iterable, state)?;
                match collection {
                    Value::List(list) => {
                        for (i, item) in list.iter().enumerate() {
                            state.var_set(key.clone(), Value::Number(i as f64));
                            state.var_set(value.clone(), item.clone());
                            self.execute_block(body, state)?;
                            if self.terminate_requested {
                                break;
                            }
                        }
                    }
                    Value::Map(map) => {
                        let mut entries: Vec<(String, Value)> = map.into_iter().collect();
                        entries.sort_by(|a, b| a.0.cmp(&b.0));
                        for (k, v) in entries {
                            state.var_set(key.clone(), Value::String(k));
                            state.var_set(value.clone(), v);
                            self.execute_block(body, state)?;
                            if self.terminate_requested {
                                break;
                            }
                        }
                    }
                    _ => return Err(CorvoError::r#type("browse only works on lists and maps")),
                }
                self.terminate_requested = false;
                Ok(())
            }
            Stmt::Terminate => {
                self.terminate_requested = true;
                Ok(())
            }
            Stmt::Assert { kind, args } => self.eval_assertion(kind, args, state),
            Stmt::DontPanic { body } => {
                // Intentionally suppress all runtime errors from the block body.
                // This includes VariableNotFound, DivisionByZero, Assertion failures,
                // and any other execution error that would normally propagate.
                let _ = self.execute_block(body, state);
                Ok(())
            }
            Stmt::AsyncBrowse {
                list,
                proc_name,
                item_param,
                shared_vars,
            } => self.exec_async_browse(list, proc_name, item_param, shared_vars, state),
        }
    }

    fn execute_block(&mut self, body: &[Stmt], state: &mut RuntimeState) -> Result<(), CorvoError> {
        for stmt in body {
            self.exec_stmt(stmt, state)?;
            if self.terminate_requested {
                return Ok(());
            }
        }
        Ok(())
    }

    /// Execute a procedure call with copy-in / copy-out pass-by-reference semantics.
    ///
    /// For each argument that is a plain `@variable` reference the corresponding
    /// outer variable is updated with the (possibly modified) parameter value
    /// after the body has run.  Non-variable arguments are copied in but never
    /// written back.  Parameter names are restored to their pre-call values (or
    /// removed if they did not exist before the call) once execution finishes.
    fn exec_procedure_call(
        &mut self,
        proc: &ProcedureValue,
        call_args: &[Expr],
        state: &mut RuntimeState,
    ) -> CorvoResult<()> {
        if call_args.len() != proc.params.len() {
            return Err(CorvoError::runtime(format!(
                "procedure expected {} argument(s), got {}",
                proc.params.len(),
                call_args.len()
            )));
        }

        // Evaluate all arguments and note which are plain variable references.
        let mut arg_values: Vec<Value> = Vec::with_capacity(call_args.len());
        let mut outer_names: Vec<Option<String>> = Vec::with_capacity(call_args.len());
        for arg_expr in call_args {
            let val = self.eval_expr(arg_expr, state)?;
            arg_values.push(val);
            if let Expr::VarGet { name } = arg_expr {
                outer_names.push(Some(name.clone()));
            } else {
                outer_names.push(None);
            }
        }

        // Save any pre-existing values for the parameter names so we can restore
        // them after the call (prevents param names from leaking into outer scope).
        let saved: Vec<Option<Value>> = proc.params.iter().map(|p| state.var_remove(p)).collect();

        // Bind parameters.
        for (param, val) in proc.params.iter().zip(arg_values) {
            state.var_set(param.to_string(), val);
        }

        // Execute the body.
        let body = proc.body.clone();
        let result = self.execute_block(&body, state);

        // Copy-back: write updated param values back to the caller's variables.
        for (i, param) in proc.params.iter().enumerate() {
            if let Some(outer_name) = &outer_names[i] {
                let updated = state.var_get(param.as_str()).unwrap_or(Value::Null);
                state.var_set(outer_name.clone(), updated);
            }
            // Restore param var to its pre-call state.
            state.var_remove(param.as_str());
            if let Some(prev) = saved[i].clone() {
                state.var_set(param.to_string(), prev);
            }
        }

        result
    }

    /// Execute an `async_browse` statement: iterate a list in parallel, running
    /// the given procedure for each item on its own thread.
    ///
    /// # Concurrency model
    ///
    /// * The **item binding** (`item_param`) is unique per thread — each thread
    ///   receives its own clone of the list element with no sharing.
    /// * Each **shared variable** is wrapped in an `Arc<Mutex<Value>>`.  Before
    ///   running the procedure body a thread briefly locks the mutex to take a
    ///   snapshot of the current value.  The procedure body runs **without** any
    ///   lock held, so I/O-bound work runs in parallel.  When the body finishes,
    ///   the thread locks the mutex and performs a **delta-merge** write-back:
    ///   for list values the items appended during the body are appended to
    ///   whatever the mutex currently holds (serializing write-backs from
    ///   concurrent threads correctly).  For all other types the thread's final
    ///   value replaces the current mutex value.
    /// * All other state variables are cloned into each thread and are
    ///   independent — mutations inside one thread are not visible to others.
    ///
    /// After all threads finish the final value of each shared variable is
    /// written back to the outer `RuntimeState`.
    fn exec_async_browse(
        &mut self,
        list_expr: &Expr,
        proc_name: &str,
        item_param: &str,
        shared_vars: &[String],
        state: &mut RuntimeState,
    ) -> CorvoResult<()> {
        // 1. Evaluate the list expression.
        let list_val = self.eval_expr(list_expr, state)?;
        let items = match list_val {
            Value::List(v) => v,
            other => {
                return Err(CorvoError::r#type(format!(
                    "async_browse requires a list, got {}",
                    other.r#type()
                )))
            }
        };

        if items.is_empty() {
            return Ok(());
        }

        let proc_val = state.var_get(proc_name)?;
        match proc_val {
            Value::Procedure(p) => {
                let expected_params = 1 + shared_vars.len();
                if p.params.len() != expected_params {
                    return Err(CorvoError::runtime(format!(
                        "async_browse: procedure '{}' expects {} parameter(s) (1 item + {} shared), got {}",
                        proc_name, expected_params, shared_vars.len(), p.params.len()
                    )));
                }
                self.exec_async_browse_ast(items, *p, item_param, shared_vars, state)
            }
            Value::NativeProcedure {
                params: p,
                callback: cb,
            } => self.exec_async_browse_native(items, p, cb, item_param, shared_vars, state),
            other => Err(CorvoError::r#type(format!(
                "async_browse: '{}' is not a procedure (got {})",
                proc_name,
                other.r#type()
            ))),
        }
    }

    fn exec_async_browse_ast(
        &mut self,
        items: Vec<Value>,
        proc: ProcedureValue,
        item_param: &str,
        shared_vars: &[String],
        state: &mut RuntimeState,
    ) -> CorvoResult<()> {
        let shared_arcs: Vec<Arc<Mutex<Value>>> = shared_vars
            .iter()
            .map(|name| {
                let val = state.var_get(name).unwrap_or(Value::Null);
                Arc::new(Mutex::new(val))
            })
            .collect::<Vec<_>>();

        let mut handles = Vec::with_capacity(items.len());
        for item in items {
            let proc_clone: ProcedureValue = proc.clone();
            let item_clone = item.clone();
            let item_param_name = item_param.to_string();
            let arcs: Vec<Arc<Mutex<Value>>> = shared_arcs.iter().map(Arc::clone).collect();
            let state_clone = state.clone();

            let handle = std::thread::spawn(move || -> CorvoResult<()> {
                let mut thread_state = state_clone;
                thread_state.var_set(item_param_name.clone(), item_clone);

                let mut snapshots: Vec<Value> = Vec::with_capacity(arcs.len());
                for (i, arc) in arcs.iter().enumerate() {
                    let param_name = &proc_clone.params[i + 1];
                    let snapshot = arc.lock().unwrap().clone();
                    snapshots.push(snapshot.clone());
                    thread_state.var_set(param_name.clone(), snapshot);
                }

                let body = proc_clone.body.clone();
                let mut evaluator = Evaluator::new();
                let result = evaluator.execute_block(&body, &mut thread_state);

                for (i, arc) in arcs.iter().enumerate() {
                    let param_name = &proc_clone.params[i + 1];
                    let thread_final = thread_state
                        .var_get(param_name.as_str())
                        .unwrap_or(Value::Null);

                    let mut guard = arc.lock().unwrap();
                    let current = guard.clone();
                    *guard = merge_shared_writeback(&snapshots[i], &thread_final, &current);
                }

                result
            });

            handles.push(handle);
        }

        let mut first_err: Option<CorvoError> = None;
        for handle in handles {
            match handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    if first_err.is_none() {
                        first_err = Some(e);
                    }
                }
                Err(_) => {
                    if first_err.is_none() {
                        first_err = Some(CorvoError::runtime("a thread panicked"));
                    }
                }
            }
        }

        for (i, arc) in shared_arcs.iter().enumerate() {
            let final_val = arc.lock().unwrap().clone();
            state.var_set(shared_vars[i].clone(), final_val);
        }

        match first_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    pub fn exec_async_browse_native(
        &mut self,
        items: Vec<Value>,
        params: Vec<String>,
        proc: NativeCallback,
        _item_param: &str,
        shared_vars: &[String],
        state: &mut RuntimeState,
    ) -> CorvoResult<()> {
        let shared_arcs: Vec<Arc<Mutex<Value>>> = shared_vars
            .iter()
            .map(|name| {
                let val = state.var_get(name).unwrap_or(Value::Null);
                Arc::new(Mutex::new(val))
            })
            .collect::<Vec<_>>();

        let mut handles = Vec::with_capacity(items.len());
        for item in items {
            let proc_clone = Arc::clone(&proc);
            let item_clone = item.clone();
            let arcs: Vec<Arc<Mutex<Value>>> = shared_arcs.iter().map(Arc::clone).collect();
            let state_clone = state.clone();
            let params_clone = params.clone();

            let handle = std::thread::spawn(move || -> CorvoResult<()> {
                let mut thread_state = state_clone;

                let mut snapshots: Vec<Value> = Vec::with_capacity(arcs.len());
                let mut call_args = vec![item_clone];
                for arc in &arcs {
                    let snapshot = arc.lock().unwrap().clone();
                    snapshots.push(snapshot.clone());
                    call_args.push(snapshot);
                }

                let thread_result = (proc_clone)(&call_args, &mut thread_state);

                for (i, arc) in arcs.iter().enumerate() {
                    let param_name = &params_clone[i + 1];
                    let thread_final = thread_state.var_get(param_name).unwrap_or(Value::Null);
                    let mut guard = arc.lock().unwrap();
                    let current = guard.clone();
                    *guard = merge_shared_writeback(&snapshots[i], &thread_final, &current);
                }

                thread_result.map(|_| ())
            });

            handles.push(handle);
        }

        let mut first_err: Option<CorvoError> = None;
        for handle in handles {
            match handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    if first_err.is_none() {
                        first_err = Some(e);
                    }
                }
                Err(_) => {
                    if first_err.is_none() {
                        first_err = Some(CorvoError::runtime("a thread panicked"));
                    }
                }
            }
        }

        for (i, arc) in shared_arcs.iter().enumerate() {
            let final_val = arc.lock().unwrap().clone();
            state.var_set(shared_vars[i].clone(), final_val);
        }

        match first_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    fn eval_expr(&self, expr: &Expr, state: &RuntimeState) -> CorvoResult<Value> {
        match expr {
            Expr::Literal { value } => Ok(value.clone()),
            Expr::VarGet { name } => state.var_get(name),
            Expr::StaticGet { name } => state.static_get(name),
            Expr::StringInterpolation { parts } => {
                let mut result = String::new();
                for part in parts {
                    let val = self.eval_expr(part, state)?;
                    result.push_str(&val.to_string());
                }
                Ok(Value::String(result))
            }
            Expr::FunctionCall {
                name,
                args,
                named_args,
            } => self.call_function(name, args, named_args, state),
            Expr::IndexAccess { target, index } => {
                let target_val = self.eval_expr(target, state)?;
                let index_val = self.eval_expr(index, state)?;
                self.index_access(&target_val, &index_val)
            }
            Expr::SliceAccess { target, start, end } => {
                let target_val = self.eval_expr(target, state)?;
                let start_val = match start {
                    Some(s) => Some(self.eval_expr(s, state)?),
                    None => None,
                };
                let end_val = match end {
                    Some(e) => Some(self.eval_expr(e, state)?),
                    None => None,
                };
                self.slice_access(&target_val, start_val.as_ref(), end_val.as_ref())
            }
            Expr::Match { value, arms } => {
                let matched = self.eval_expr(value, state)?;
                for arm in arms {
                    let is_match = match &arm.pattern {
                        MatchPattern::Literal(lit) => matched == *lit,
                        MatchPattern::Regex(pattern, flags) => {
                            if let Value::String(text) = &matched {
                                crate::standard_lib::re::build_regex(pattern, flags)
                                    .map(|re| re.is_match(text))
                                    .unwrap_or(false)
                            } else {
                                false
                            }
                        }
                        MatchPattern::Wildcard => true,
                    };
                    if is_match {
                        return self.eval_expr(&arm.body, state);
                    }
                }
                Err(CorvoError::runtime(format!(
                    "No match arm matched the value: {}",
                    matched
                )))
            }
            Expr::ProcedureLiteral { params, body } => {
                Ok(Value::Procedure(Box::new(ProcedureValue {
                    params: params.clone(),
                    body: body.clone(),
                })))
            }
            Expr::SharedArg { .. } => Err(CorvoError::runtime(
                "shared @var is only valid inside async_browse arguments",
            )),
            Expr::Unary { op, operand } => {
                let v = self.eval_expr(operand, state)?;
                match op {
                    UnaryOp::Neg => match v {
                        Value::Number(n) => Ok(Value::Number(-n)),
                        other => Err(CorvoError::r#type(format!(
                            "Unary '-' expects a number, got {}",
                            other.r#type()
                        ))),
                    },
                    UnaryOp::Not => Ok(Value::Boolean(!v.as_bool().unwrap_or(false))),
                }
            }
            Expr::Binary { op, left, right } => {
                let l = self.eval_expr(left, state)?;
                
                // Short-circuiting for logical operators
                if *op == BinaryOp::And {
                    if !l.as_bool().unwrap_or(false) {
                        return Ok(Value::Boolean(false));
                    }
                    let r = self.eval_expr(right, state)?;
                    return Ok(Value::Boolean(r.as_bool().unwrap_or(false)));
                }
                if *op == BinaryOp::Or {
                    if l.as_bool().unwrap_or(false) {
                        return Ok(Value::Boolean(true));
                    }
                    let r = self.eval_expr(right, state)?;
                    return Ok(Value::Boolean(r.as_bool().unwrap_or(false)));
                }

                let r = self.eval_expr(right, state)?;
                match op {
                    BinaryOp::Eq => Ok(Value::Boolean(l == r)),
                    BinaryOp::Neq => Ok(Value::Boolean(l != r)),
                    BinaryOp::Lt => Ok(Value::Boolean(l < r)),
                    BinaryOp::Le => Ok(Value::Boolean(l <= r)),
                    BinaryOp::Gt => Ok(Value::Boolean(l > r)),
                    BinaryOp::Ge => Ok(Value::Boolean(l >= r)),
                    _ => {
                        let (ln, rn) = match (l, r) {
                            (Value::Number(a), Value::Number(b)) => (a, b),
                            (a, b) => {
                                return Err(CorvoError::r#type(format!(
                                    "Arithmetic expects numbers, got {} and {}",
                                    a.r#type(),
                                    b.r#type()
                                )));
                            }
                        };
                        let out = match op {
                            BinaryOp::Add => ln + rn,
                            BinaryOp::Sub => ln - rn,
                            BinaryOp::Mul => ln * rn,
                            BinaryOp::Div => ln / rn,
                            BinaryOp::Mod => ln % rn,
                            _ => unreachable!(),
                        };
                        Ok(Value::Number(out))
                    }
                }
            }
            Expr::MethodCall {
                target,
                method,
                args,
                named_args,
            } => {
                let target_val = self.eval_expr(target, state)?;
                let ns = match &target_val {
                    Value::Regex(_, _) => "re",
                    Value::String(_) => "string",
                    Value::Number(_) => "number",
                    Value::List(_) => "list",
                    Value::Map(_) => "map",
                    Value::Procedure(_) | Value::NativeProcedure { .. } => return Err(CorvoError::runtime(
                        "procedure.call must be used as a statement, not in an expression context",
                    )),
                    other => {
                        return Err(CorvoError::r#type(format!(
                            "Cannot call method '{}' on type {}",
                            method,
                            other.r#type()
                        )))
                    }
                };
                let func_name = format!("{}.{}", ns, method);
                let evaluated_args: Vec<Value> = args
                    .iter()
                    .map(|arg| self.eval_expr(arg, state))
                    .collect::<CorvoResult<Vec<_>>>()?;
                let evaluated_named: std::collections::HashMap<String, Value> = named_args
                    .iter()
                    .map(|(k, v)| Ok((k.clone(), self.eval_expr(v, state)?)))
                    .collect::<CorvoResult<_>>()?;
                let mut all_args = vec![target_val];
                all_args.extend(evaluated_args);
                standard_lib::call(&func_name, &all_args, &evaluated_named, state)
            }
        }
    }

    fn call_function(
        &self,
        name: &str,
        args: &[Expr],
        named_args: &std::collections::HashMap<String, Expr>,
        state: &RuntimeState,
    ) -> CorvoResult<Value> {
        let evaluated_args: Vec<Value> = args
            .iter()
            .map(|arg| self.eval_expr(arg, state))
            .collect::<CorvoResult<Vec<_>>>()?;

        let evaluated_named: std::collections::HashMap<String, Value> = named_args
            .iter()
            .map(|(k, v)| Ok((k.clone(), self.eval_expr(v, state)?)))
            .collect::<CorvoResult<_>>()?;

        if name == "__list__" {
            return Ok(Value::List(evaluated_args));
        }

        if name == "__map__" {
            let mut map = std::collections::HashMap::new();
            let mut i = 0;
            while i + 1 < evaluated_args.len() {
                let key = evaluated_args[i].to_string();
                let value = evaluated_args[i + 1].clone();
                map.insert(key, value);
                i += 2;
            }
            return Ok(Value::Map(map));
        }

        standard_lib::call(name, &evaluated_args, &evaluated_named, state)
    }

    fn index_access(&self, target: &Value, index: &Value) -> CorvoResult<Value> {
        match (target, index) {
            (Value::List(list), Value::Number(idx)) => {
                let idx = *idx as usize;
                list.get(idx)
                    .cloned()
                    .ok_or_else(|| CorvoError::runtime(format!("Index {} out of bounds", idx)))
            }
            (Value::Map(map), Value::String(key)) => map
                .get(key)
                .cloned()
                .ok_or_else(|| CorvoError::runtime(format!("Key '{}' not found", key))),
            _ => Err(CorvoError::r#type("Cannot index into this type")),
        }
    }

    fn resolve_slice_index(index: f64, length: usize) -> usize {
        if index < 0.0 {
            let offset = (-index) as usize;
            length.saturating_sub(offset)
        } else {
            (index as usize).min(length)
        }
    }

    fn slice_access(
        &self,
        target: &Value,
        start: Option<&Value>,
        end: Option<&Value>,
    ) -> CorvoResult<Value> {
        match target {
            Value::List(list) => {
                let len = list.len();
                let start_idx = match start {
                    Some(Value::Number(n)) => Self::resolve_slice_index(*n, len),
                    None => 0,
                    _ => return Err(CorvoError::r#type("List slice index must be a number")),
                };
                let end_idx = match end {
                    Some(Value::Number(n)) => Self::resolve_slice_index(*n, len),
                    None => len,
                    _ => return Err(CorvoError::r#type("List slice index must be a number")),
                };
                let start_idx = start_idx.min(end_idx);
                Ok(Value::List(list[start_idx..end_idx].to_vec()))
            }
            Value::String(s) => {
                let chars: Vec<char> = s.chars().collect();
                let len = chars.len();
                let start_idx = match start {
                    Some(Value::Number(n)) => Self::resolve_slice_index(*n, len),
                    None => 0,
                    _ => return Err(CorvoError::r#type("String slice index must be a number")),
                };
                let end_idx = match end {
                    Some(Value::Number(n)) => Self::resolve_slice_index(*n, len),
                    None => len,
                    _ => return Err(CorvoError::r#type("String slice index must be a number")),
                };
                let start_idx = start_idx.min(end_idx);
                Ok(Value::String(chars[start_idx..end_idx].iter().collect()))
            }
            _ => Err(CorvoError::r#type("Cannot slice this type")),
        }
    }

    fn eval_assertion(
        &self,
        kind: &AssertKind,
        args: &[Expr],
        state: &RuntimeState,
    ) -> CorvoResult<()> {
        if args.is_empty() {
            return Err(CorvoError::parsing(
                "Assertion requires at least one argument",
            ));
        }

        let values: Vec<Value> = args
            .iter()
            .map(|arg| self.eval_expr(arg, state))
            .collect::<CorvoResult<Vec<_>>>()?;

        match kind {
            AssertKind::Eq => {
                if values.len() != 2 {
                    return Err(CorvoError::parsing(
                        "assert_eq requires exactly 2 arguments",
                    ));
                }
                if values[0] != values[1] {
                    return Err(CorvoError::assertion(format!(
                        "{} != {}",
                        values[0], values[1]
                    )));
                }
            }
            AssertKind::Neq => {
                if values.len() != 2 {
                    return Err(CorvoError::parsing(
                        "assert_neq requires exactly 2 arguments",
                    ));
                }
                if values[0] == values[1] {
                    return Err(CorvoError::assertion(format!(
                        "{} == {}",
                        values[0], values[1]
                    )));
                }
            }
            AssertKind::Gt => {
                if values.len() != 2 {
                    return Err(CorvoError::parsing(
                        "assert_gt requires exactly 2 arguments",
                    ));
                }
                let a = values[0]
                    .as_number()
                    .ok_or_else(|| CorvoError::r#type("assert_gt requires numbers"))?;
                let b = values[1]
                    .as_number()
                    .ok_or_else(|| CorvoError::r#type("assert_gt requires numbers"))?;
                if a <= b {
                    return Err(CorvoError::assertion(format!("{} !> {}", a, b)));
                }
            }
            AssertKind::Ge => {
                if values.len() != 2 {
                    return Err(CorvoError::parsing(
                        "assert_ge requires exactly 2 arguments",
                    ));
                }
                let a = values[0]
                    .as_number()
                    .ok_or_else(|| CorvoError::r#type("assert_ge requires numbers"))?;
                let b = values[1]
                    .as_number()
                    .ok_or_else(|| CorvoError::r#type("assert_ge requires numbers"))?;
                if a < b {
                    return Err(CorvoError::assertion(format!("{} !>= {}", a, b)));
                }
            }
            AssertKind::Lt => {
                if values.len() != 2 {
                    return Err(CorvoError::parsing(
                        "assert_lt requires exactly 2 arguments",
                    ));
                }
                let a = values[0]
                    .as_number()
                    .ok_or_else(|| CorvoError::r#type("assert_lt requires numbers"))?;
                let b = values[1]
                    .as_number()
                    .ok_or_else(|| CorvoError::r#type("assert_lt requires numbers"))?;
                if a >= b {
                    return Err(CorvoError::assertion(format!("{} !< {}", a, b)));
                }
            }
            AssertKind::Le => {
                if values.len() != 2 {
                    return Err(CorvoError::parsing(
                        "assert_le requires exactly 2 arguments",
                    ));
                }
                let a = values[0]
                    .as_number()
                    .ok_or_else(|| CorvoError::r#type("assert_le requires numbers"))?;
                let b = values[1]
                    .as_number()
                    .ok_or_else(|| CorvoError::r#type("assert_le requires numbers"))?;
                if a > b {
                    return Err(CorvoError::assertion(format!("{} !<= {}", a, b)));
                }
            }
            AssertKind::Match => {
                if values.len() != 2 {
                    return Err(CorvoError::parsing(
                        "assert_match requires exactly 2 arguments",
                    ));
                }
                let pattern = values[0]
                    .as_string()
                    .ok_or_else(|| CorvoError::r#type("assert_match requires strings"))?;
                let target = values[1]
                    .as_string()
                    .ok_or_else(|| CorvoError::r#type("assert_match requires strings"))?;
                let re =
                    regex::Regex::new(pattern).map_err(|e| CorvoError::parsing(e.to_string()))?;
                if !re.is_match(target) {
                    return Err(CorvoError::assertion(format!(
                        "'{}' does not match '{}'",
                        target, pattern
                    )));
                }
            }
        }
        Ok(())
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// Merge a thread's shared-variable write-back with the current mutex value.
///
/// For **list** values this implements an append-delta merge: items that the
/// thread **appended** beyond its starting snapshot (i.e. elements at indices
/// `snap.len()..fin.len()`) are appended to whatever the mutex currently holds.
/// This preserves all contributions from concurrent threads when the procedure
/// body exclusively uses append operations such as `@acc = list.push(@acc, item)`.
///
/// **Limitation**: the merge assumes items are only ever appended to the end
/// of the list, not inserted at arbitrary positions or replaced.  If the
/// procedure body uses `list.filter`, `list.map`, `list.set`, or any operation
/// that changes existing elements, the slice `fin[snap.len()..]` may extract
/// incorrect items.  In those cases — or whenever `fin.len() < snap.len()` —
/// the thread's final value is used directly (last-writer-wins).
///
/// For all other value types the thread's final value replaces the current
/// mutex value (last-writer-wins semantics).
pub fn merge_shared_writeback(snapshot: &Value, thread_final: &Value, current: &Value) -> Value {
    match (snapshot, thread_final, current) {
        (Value::List(snap), Value::List(fin), Value::List(cur)) if fin.len() >= snap.len() => {
            // Append only the items the thread added beyond its snapshot.
            let new_items = &fin[snap.len()..];
            let mut result = cur.clone();
            result.extend_from_slice(new_items);
            Value::List(result)
        }
        (Value::Number(snap), Value::Number(fin), Value::Number(cur)) => {
            // Add the delta that the thread contributed.
            Value::Number(cur + (fin - snap))
        }
        (Value::String(snap), Value::String(fin), Value::String(cur)) if fin.starts_with(snap) => {
            // Append only the suffix the thread added.
            let suffix = &fin[snap.len()..];
            let mut result = cur.clone();
            result.push_str(suffix);
            Value::String(result)
        }
        _ => thread_final.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn eval_source(source: &str) -> CorvoResult<RuntimeState> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let program = parser.parse()?;

        let mut state = RuntimeState::new();
        let mut evaluator = Evaluator::new();
        evaluator.run(&program, &mut state)?;
        Ok(state)
    }

    fn eval_expect_err(source: &str) -> CorvoError {
        eval_source(source).expect_err(&format!("Expected error for: {}", source))
    }

    // --- Basic Literals ---

    #[test]
    fn test_eval_var_set_and_get() {
        let state = eval_source(r#"var.set("x", 42)"#).unwrap();
        assert_eq!(state.var_get("x").unwrap(), Value::Number(42.0));
    }

    #[test]
    fn test_eval_static_set_and_get() {
        let state = eval_source(r#"prep { static.set("pi", 2.5) }"#).unwrap();
        assert_eq!(state.static_get("pi").unwrap(), Value::Number(2.5));
    }

    #[test]
    fn test_eval_string_literal() {
        let state = eval_source(r#"var.set("msg", "hello")"#).unwrap();
        assert_eq!(
            state.var_get("msg").unwrap(),
            Value::String("hello".to_string())
        );
    }

    #[test]
    fn test_eval_boolean_literal() {
        let state = eval_source(r#"var.set("flag", true)"#).unwrap();
        assert_eq!(state.var_get("flag").unwrap(), Value::Boolean(true));
    }

    // --- Math Operations ---

    #[test]
    fn test_eval_math_add() {
        let state = eval_source(r#"var.set("result", math.add(1, 2))"#).unwrap();
        assert_eq!(state.var_get("result").unwrap(), Value::Number(3.0));
    }

    #[test]
    fn test_eval_math_sub() {
        let state = eval_source(r#"var.set("result", math.sub(10, 3))"#).unwrap();
        assert_eq!(state.var_get("result").unwrap(), Value::Number(7.0));
    }

    #[test]
    fn test_eval_math_mul() {
        let state = eval_source(r#"var.set("result", math.mul(4, 5))"#).unwrap();
        assert_eq!(state.var_get("result").unwrap(), Value::Number(20.0));
    }

    #[test]
    fn test_eval_math_div() {
        let state = eval_source(r#"var.set("result", math.div(10, 2))"#).unwrap();
        assert_eq!(state.var_get("result").unwrap(), Value::Number(5.0));
    }

    #[test]
    fn test_eval_division_by_zero() {
        let result = eval_source(r#"var.set("result", math.div(1, 0))"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_math_modulo() {
        let state = eval_source(r#"var.set("result", math.mod(10, 3))"#).unwrap();
        assert_eq!(state.var_get("result").unwrap(), Value::Number(1.0));
    }

    // --- String Operations ---

    #[test]
    fn test_eval_string_concat() {
        let state = eval_source(r#"var.set("result", string.concat("hello", " world"))"#).unwrap();
        assert_eq!(
            state.var_get("result").unwrap(),
            Value::String("hello world".to_string())
        );
    }

    #[test]
    fn test_eval_string_interpolation() {
        let state = eval_source(
            r#"
            var.set("name", "world")
            var.set("msg", "Hello ${var.get("name")}")
            "#,
        )
        .unwrap();
        assert_eq!(
            state.var_get("msg").unwrap(),
            Value::String("Hello world".to_string())
        );
    }

    #[test]
    fn test_eval_string_interpolation_number() {
        let state = eval_source(
            r#"
            var.set("count", 42)
            var.set("msg", "Count: ${var.get("count")}")
            "#,
        )
        .unwrap();
        assert_eq!(
            state.var_get("msg").unwrap(),
            Value::String("Count: 42".to_string())
        );
    }

    #[test]
    fn test_eval_string_interpolation_expr() {
        let state = eval_source(
            r#"
            var.set("a", 10)
            var.set("b", 20)
            var.set("msg", "Sum: ${math.add(var.get("a"), var.get("b"))}")
            "#,
        )
        .unwrap();
        assert_eq!(
            state.var_get("msg").unwrap(),
            Value::String("Sum: 30".to_string())
        );
    }

    #[test]
    fn test_eval_string_interpolation_multiple() {
        let state = eval_source(
            r#"
            var.set("first", "John")
            var.set("last", "Doe")
            var.set("msg", "${var.get("first")} ${var.get("last")}")
            "#,
        )
        .unwrap();
        assert_eq!(
            state.var_get("msg").unwrap(),
            Value::String("John Doe".to_string())
        );
    }

    // --- List Operations ---

    #[test]
    fn test_eval_list_push() {
        let state = eval_source(
            r#"
            var.set("a", 1)
            var.set("b", 2)
            var.set("items", list.push(list.push([], var.get("a")), var.get("b")))
            "#,
        )
        .unwrap();
        match state.var_get("items").unwrap() {
            Value::List(items) => assert_eq!(items.len(), 2),
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_eval_index_access_list() {
        let state = eval_source(
            r#"
            var.set("items", list.push(list.push([], "a"), "b"))
            var.set("item", list.get(var.get("items"), 1))
            "#,
        )
        .unwrap();
        assert_eq!(
            state.var_get("item").unwrap(),
            Value::String("b".to_string())
        );
    }

    #[test]
    fn test_eval_list_literal() {
        let state = eval_source(r#"var.set("items", [1, 2, 3])"#).unwrap();
        match state.var_get("items").unwrap() {
            Value::List(items) => assert_eq!(items.len(), 3),
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_eval_empty_list_literal() {
        let state = eval_source(r#"var.set("items", [])"#).unwrap();
        match state.var_get("items").unwrap() {
            Value::List(items) => assert!(items.is_empty()),
            _ => panic!("Expected List"),
        }
    }

    // --- Map Operations ---

    #[test]
    fn test_eval_map_literal() {
        let state = eval_source(r#"var.set("m", {"a": 1, "b": 2})"#).unwrap();
        match state.var_get("m").unwrap() {
            Value::Map(m) => assert_eq!(m.len(), 2),
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_eval_empty_map_literal() {
        let state = eval_source(r#"var.set("m", {})"#).unwrap();
        match state.var_get("m").unwrap() {
            Value::Map(m) => assert!(m.is_empty()),
            _ => panic!("Expected Map"),
        }
    }

    // --- Control Flow ---

    #[test]
    fn test_eval_multiple_statements() {
        let state = eval_source(
            r#"
            var.set("x", 1)
            var.set("y", 2)
            var.set("sum", math.add(var.get("x"), var.get("y")))
            "#,
        )
        .unwrap();
        assert_eq!(state.var_get("sum").unwrap(), Value::Number(3.0));
    }

    #[test]
    fn test_eval_try_success() {
        let state = eval_source(
            r#"
            var.set("result", "not run")
            try {
                assert_eq(1, 1)
                var.set("result", "success")
            } fallback {
                var.set("result", "fallback")
            }
            "#,
        )
        .unwrap();
        assert_eq!(
            state.var_get("result").unwrap(),
            Value::String("success".to_string())
        );
    }

    #[test]
    fn test_eval_try_fallback() {
        let state = eval_source(
            r#"
            var.set("result", "not run")
            try {
                assert_eq(1, 2)
                var.set("result", "success")
            } fallback {
                var.set("result", "fallback")
            }
            "#,
        )
        .unwrap();
        assert_eq!(
            state.var_get("result").unwrap(),
            Value::String("fallback".to_string())
        );
    }

    #[test]
    fn test_eval_try_multiple_fallbacks() {
        let state = eval_source(
            r#"
            var.set("result", "init")
            try {
                assert_eq(1, 2)
            } fallback {
                assert_eq(3, 4)
            } fallback {
                var.set("result", "second fallback")
            }
            "#,
        )
        .unwrap();
        assert_eq!(
            state.var_get("result").unwrap(),
            Value::String("second fallback".to_string())
        );
    }

    #[test]
    fn test_eval_nested_try_blocks() {
        let state = eval_source(
            r#"
            var.set("result", "init")
            try {
                try {
                    assert_eq(1, 2)
                } fallback {
                    var.set("result", "inner fallback ran")
                }
            } fallback {
                var.set("result", "outer fallback")
            }
            "#,
        )
        .unwrap();
        assert_eq!(
            state.var_get("result").unwrap(),
            Value::String("inner fallback ran".to_string())
        );
    }

    #[test]
    fn test_eval_loop_with_terminate() {
        let state = eval_source(
            r#"
            var.set("count", 0)
            loop {
                var.set("count", math.add(var.get("count"), 1))
                try {
                    assert_eq(var.get("count"), 3)
                    terminate
                } fallback {
                }
            }
            "#,
        )
        .unwrap();
        assert_eq!(state.var_get("count").unwrap(), Value::Number(3.0));
    }

    #[test]
    fn test_eval_terminate() {
        let result = eval_source(
            r#"
            var.set("before", true)
            terminate
            var.set("after", true)
            "#,
        );
        assert!(result.is_ok());
        let state = result.unwrap();
        assert_eq!(state.var_get("before").unwrap(), Value::Boolean(true));
        assert!(state.var_get("after").is_err());
    }

    // --- Assertion Tests ---

    #[test]
    fn test_eval_assert_eq_pass() {
        assert!(eval_source("assert_eq(1, 1)").is_ok());
    }

    #[test]
    fn test_eval_assert_eq_fail() {
        let err = eval_expect_err("assert_eq(1, 2)");
        assert!(format!("{}", err).contains("1 != 2"));
    }

    #[test]
    fn test_eval_assert_neq_pass() {
        assert!(eval_source("assert_neq(1, 2)").is_ok());
    }

    #[test]
    fn test_eval_assert_neq_fail() {
        let err = eval_expect_err("assert_neq(1, 1)");
        assert!(format!("{}", err).contains("=="));
    }

    #[test]
    fn test_eval_assert_gt_pass() {
        assert!(eval_source("assert_gt(2, 1)").is_ok());
    }

    #[test]
    fn test_eval_assert_gt_fail() {
        let err = eval_expect_err("assert_gt(1, 2)");
        assert!(format!("{}", err).contains("!>"));
    }

    #[test]
    fn test_eval_assert_lt_pass() {
        assert!(eval_source("assert_lt(1, 2)").is_ok());
    }

    #[test]
    fn test_eval_assert_lt_fail() {
        let err = eval_expect_err("assert_lt(2, 1)");
        assert!(format!("{}", err).contains("!<"));
    }

    #[test]
    fn test_eval_assert_match_pass() {
        assert!(eval_source(r#"assert_match("hello.*", "hello world")"#).is_ok());
    }

    #[test]
    fn test_eval_assert_match_fail() {
        let err = eval_expect_err(r#"assert_match("hello.*", "goodbye")"#);
        assert!(format!("{}", err).contains("does not match"));
    }

    // --- Error Cases ---

    #[test]
    fn test_eval_var_not_found() {
        let err = eval_expect_err("var.set(\"x\", var.get(\"nonexistent\"))");
        assert!(format!("{}", err).contains("nonexistent"));
    }

    #[test]
    fn test_eval_static_not_found() {
        let err = eval_expect_err("var.set(\"x\", static.get(\"nonexistent\"))");
        assert!(format!("{}", err).contains("nonexistent"));
    }

    #[test]
    fn test_eval_unknown_function() {
        let err = eval_expect_err("nonexistent_func()");
        assert!(format!("{}", err).contains("nonexistent_func"));
    }

    #[test]
    fn test_eval_index_out_of_bounds() {
        let err = eval_expect_err(r#"list.get([], 0)"#);
        assert!(format!("{}", err).contains("out of bounds"));
    }

    #[test]
    fn test_eval_division_by_zero_mod() {
        assert!(eval_source(r#"math.mod(1, 0)"#).is_err());
    }

    // --- Complex Programs ---

    #[test]
    fn test_eval_comprehensive_program() {
        let state = eval_source(
            r#"
            var.set("counter", 0)
            var.set("results", [])
            loop {
                var.set("counter", math.add(var.get("counter"), 1))
                var.set("results", list.push(var.get("results"), var.get("counter")))
                try {
                    assert_eq(var.get("counter"), 5)
                    terminate
                } fallback {
                }
            }
            "#,
        )
        .unwrap();
        assert_eq!(state.var_get("counter").unwrap(), Value::Number(5.0));
        match state.var_get("results").unwrap() {
            Value::List(items) => assert_eq!(items.len(), 5),
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_eval_var_overwrite() {
        let state = eval_source(
            r#"
            var.set("x", 1)
            var.set("x", 2)
            "#,
        )
        .unwrap();
        assert_eq!(state.var_get("x").unwrap(), Value::Number(2.0));
    }

    #[test]
    fn test_eval_static_var_independent() {
        let state = eval_source(
            r#"
            prep {
                static.set("x", 2)
            }
            var.set("x", 1)
            "#,
        )
        .unwrap();
        assert_eq!(state.var_get("x").unwrap(), Value::Number(1.0));
        assert_eq!(state.static_get("x").unwrap(), Value::Number(2.0));
    }

    #[test]
    fn test_eval_nested_function_calls() {
        let state = eval_source(
            r#"
            var.set("result", math.add(math.mul(2, 3), math.div(10, 2)))
            "#,
        )
        .unwrap();
        assert_eq!(state.var_get("result").unwrap(), Value::Number(11.0));
    }

    #[test]
    fn test_eval_string_methods_in_expr() {
        let state = eval_source(
            r#"
            var.set("msg", string.concat(string.to_upper("hello"), " WORLD"))
            "#,
        )
        .unwrap();
        assert_eq!(
            state.var_get("msg").unwrap(),
            Value::String("HELLO WORLD".to_string())
        );
    }
}
