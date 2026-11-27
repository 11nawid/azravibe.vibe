//! Bytecode compiler and stack VM for Azra.
//!
//! `azra run --vm script.vibe` compiles the AST into a flat instruction stream
//! and executes it without recursive AST walking. The tree-walking interpreter
//! remains the default; unsupported constructs produce a clear compile error.

use crate::ast::{BinOp, Expr, OutputLevel, Stmt, Target};
use crate::error::AzraError;
use crate::token::Num;
use crate::value::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Op {
    Push(Value),
    Load(String),
    Store(String),
    Pop,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
    Neg,
    Jump(usize),
    JumpIfFalse(usize),
    Call { name: String, argc: usize },
    Return,
    Output { level: OutputLevel, argc: usize },
    Halt,
}

#[derive(Debug, Clone, Default)]
pub struct Chunk {
    pub code: Vec<Op>,
}

impl Chunk {
    fn emit(&mut self, op: Op) -> usize {
        self.code.push(op);
        self.code.len() - 1
    }
    fn patch(&mut self, at: usize, target: usize) {
        match &mut self.code[at] {
            Op::Jump(addr) | Op::JumpIfFalse(addr) => *addr = target,
            other => panic!("cannot patch {:?}", other),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Program {
    pub main: Chunk,
    pub functions: HashMap<String, Chunk>,
}

struct Compiler<'a> {
    chunk: Chunk,
    functions: &'a mut HashMap<String, Chunk>,
}

impl<'a> Compiler<'a> {
    fn new(functions: &'a mut HashMap<String, Chunk>) -> Self {
        Compiler {
            chunk: Chunk::default(),
            functions,
        }
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), AzraError> {
        match expr {
            Expr::Number(n) => {
                self.chunk.emit(Op::Push(Value::Number(n.clone())));
            }
            Expr::Str(s) => {
                self.chunk.emit(Op::Push(Value::Str(s.clone())));
            }
            Expr::Bool(b) => {
                self.chunk.emit(Op::Push(Value::Bool(*b)));
            }
            Expr::Null => {
                self.chunk.emit(Op::Push(Value::Null));
            }
            Expr::Var { name, .. } => {
                self.chunk.emit(Op::Load(name.clone()));
            }
            Expr::Negate { expr, .. } => {
                self.compile_expr(expr)?;
                self.chunk.emit(Op::Neg);
            }
            Expr::Not { expr, .. } => {
                self.compile_expr(expr)?;
                self.chunk.emit(Op::Not);
            }
            Expr::Binary {
                op, left, right, ..
            } => {
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                let operation = match op {
                    BinOp::Add => Op::Add,
                    BinOp::Sub => Op::Sub,
                    BinOp::Mul => Op::Mul,
                    BinOp::Div => Op::Div,
                    BinOp::Mod => Op::Mod,
                    BinOp::Pow => Op::Pow,
                    BinOp::Eq => Op::Eq,
                    BinOp::Ne => Op::Ne,
                    BinOp::Lt => Op::Lt,
                    BinOp::Le => Op::Le,
                    BinOp::Gt => Op::Gt,
                    BinOp::Ge => Op::Ge,
                    BinOp::And => Op::And,
                    BinOp::Or => Op::Or,
                    _ => {
                        return Err(AzraError::runtime(
                            "operator cannot be compiled to bytecode",
                        ));
                    }
                };
                self.chunk.emit(operation);
            }
            Expr::Call { path, args, .. } if path.len() == 1 => {
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.chunk.emit(Op::Call {
                    name: path[0].clone(),
                    argc: args.len(),
                });
            }
            _ => {
                return Err(AzraError::runtime(
                    "expression cannot be compiled to bytecode",
                ));
            }
        }
        Ok(())
    }

    fn compile_block(&mut self, statements: &[Stmt]) -> Result<(), AzraError> {
        for statement in statements {
            match statement {
                Stmt::Set {
                    target: Target::Path { path, .. },
                    value,
                    ..
                } if path.len() == 1 && value.len() == 1 => {
                    self.compile_expr(&value[0])?;
                    self.chunk.emit(Op::Store(path[0].clone()));
                }
                Stmt::Output { level, args, .. } => {
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    self.chunk.emit(Op::Output {
                        level: *level,
                        argc: args.len(),
                    });
                }
                Stmt::ExprStmt { expr, .. } => {
                    self.compile_expr(expr)?;
                    self.chunk.emit(Op::Pop);
                }
                Stmt::Call { path, args, .. } if path.len() == 1 => {
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    self.chunk.emit(Op::Call {
                        name: path[0].clone(),
                        argc: args.len(),
                    });
                }
                Stmt::If {
                    branches,
                    otherwise,
                    ..
                } => {
                    let mut exits = Vec::new();
                    for branch in branches {
                        self.compile_expr(&branch.cond)?;
                        let skip = self.chunk.emit(Op::JumpIfFalse(0));
                        self.compile_block(&branch.body)?;
                        exits.push(self.chunk.emit(Op::Jump(0)));
                        self.chunk.patch(skip, self.chunk.code.len());
                    }
                    if let Some(body) = otherwise {
                        self.compile_block(body)?;
                    }
                    let end = self.chunk.code.len();
                    for exit in exits {
                        self.chunk.patch(exit, end);
                    }
                }
                Stmt::While { cond, body, .. } => {
                    let start = self.chunk.code.len();
                    self.compile_expr(cond)?;
                    let exit = self.chunk.emit(Op::JumpIfFalse(0));
                    self.compile_block(body)?;
                    self.chunk.emit(Op::Jump(start));
                    self.chunk.patch(exit, self.chunk.code.len());
                }
                Stmt::Return { value, .. } => {
                    if let Some(expr) = value {
                        self.compile_expr(expr)?;
                    } else {
                        self.chunk.emit(Op::Push(Value::Null));
                    }
                    self.chunk.emit(Op::Return);
                }
                Stmt::Define {
                    name, params, body, ..
                } => {
                    let mut functions = std::mem::take(self.functions);
                    let mut function = Compiler::new(&mut functions);
                    for param in params.iter().rev() {
                        function.chunk.emit(Op::Store(param.clone()));
                    }
                    function.compile_block(body)?;
                    function.chunk.emit(Op::Push(Value::Null));
                    function.chunk.emit(Op::Return);
                    let compiled_chunk = function.chunk.clone();
                    drop(function);
                    functions.insert(name.clone(), compiled_chunk);
                    *self.functions = functions;
                }
                _ => {
                    return Err(AzraError::runtime(
                        "statement cannot be compiled to bytecode",
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Compile a parsed program into bytecode.
pub fn compile(statements: &[Stmt]) -> Result<Program, AzraError> {
    let mut functions = HashMap::new();
    let mut compiler = Compiler::new(&mut functions);
    compiler.compile_block(statements)?;
    compiler.chunk.emit(Op::Halt);
    Ok(Program {
        main: compiler.chunk,
        functions,
    })
}

#[cfg(test)]
mod vm_tests {
    use super::*;

    #[test]
    fn compiles_and_runs_a_function() {
        let source = "define add with a, b:\n    give back a + b\nset result to add with 20, 22\nsay result\n";
        let tokens = crate::lexer::tokenize(source).unwrap();
        let ast = crate::parser::parse(&tokens).unwrap();
        let program = compile(&ast).unwrap();
        let output = run(&program).unwrap();
        assert_eq!(output, vec!["42"]);
    }
}

struct Frame {
    chunk: Chunk,
    ip: usize,
    locals: HashMap<String, Value>,
}

fn as_number(value: &Value) -> Result<Num, AzraError> {
    match value {
        Value::Number(n) => Ok(n.clone()),
        other => Err(AzraError::runtime(format!(
            "operator expects a number, got {}",
            other.type_name()
        ))),
    }
}

fn as_f64(value: &Value) -> Result<f64, AzraError> {
    Ok(as_number(value)?.as_f64())
}

fn binary(op: &Op, left: Value, right: Value) -> Result<Value, AzraError> {
    let result = match op {
        Op::Add => match (&left, &right) {
            (Value::Number(a), Value::Number(b)) => Value::Number(num_binary(a, b, '+')?),
            _ => Value::Str(format!("{}{}", left.render(), right.render())),
        },
        Op::Sub => Value::Number(num_binary(&as_number(&left)?, &as_number(&right)?, '-')?),
        Op::Mul => Value::Number(num_binary(&as_number(&left)?, &as_number(&right)?, '*')?),
        Op::Div => {
            let divisor = as_f64(&right)?;
            if divisor == 0.0 {
                return Err(AzraError::runtime("division by zero"));
            }
            Value::Number(Num::Float(as_f64(&left)? / divisor))
        }
        Op::Mod => {
            let divisor = as_f64(&right)?;
            if divisor == 0.0 {
                return Err(AzraError::runtime("modulo by zero"));
            }
            Value::Number(Num::Float(as_f64(&left)?.rem_euclid(divisor)))
        }
        Op::Pow => Value::Number(Num::Float(as_f64(&left)?.powf(as_f64(&right)?))),
        Op::Eq => Value::Bool(left.equals(&right)),
        Op::Ne => Value::Bool(!left.equals(&right)),
        Op::Lt => Value::Bool(compare(&left, &right)? < 0),
        Op::Le => Value::Bool(compare(&left, &right)? <= 0),
        Op::Gt => Value::Bool(compare(&left, &right)? > 0),
        Op::Ge => Value::Bool(compare(&left, &right)? >= 0),
        Op::And => Value::Bool(left.is_truthy() && right.is_truthy()),
        Op::Or => Value::Bool(left.is_truthy() || right.is_truthy()),
        _ => unreachable!(),
    };
    Ok(result)
}

fn num_binary(a: &Num, b: &Num, op: char) -> Result<Num, AzraError> {
    let (x, y) = (a.as_f64(), b.as_f64());
    let value = match op {
        '+' => x + y,
        '-' => x - y,
        _ => x * y,
    };
    if matches!((a, b), (Num::Int(_), Num::Int(_))) && value.fract() == 0.0 {
        Ok(Num::Int(value as i64))
    } else {
        Ok(Num::Float(value))
    }
}

fn compare(left: &Value, right: &Value) -> Result<i32, AzraError> {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => Ok(if a.as_f64() < b.as_f64() {
            -1
        } else if a.as_f64() > b.as_f64() {
            1
        } else {
            0
        }),
        (Value::Str(a), Value::Str(b)) => Ok(a.cmp(b) as i32),
        _ => Err(AzraError::runtime("cannot compare these values")),
    }
}

/// Execute a compiled program and return its output lines.
pub fn run(program: &Program) -> Result<Vec<String>, AzraError> {
    let mut globals: HashMap<String, Value> = HashMap::new();
    let mut stack: Vec<Value> = Vec::new();
    let mut output = Vec::new();
    let mut frames = vec![Frame {
        chunk: program.main.clone(),
        ip: 0,
        locals: HashMap::new(),
    }];

    loop {
        let frame = frames.last_mut().expect("a frame is always present");
        let Some(op) = frame.chunk.code.get(frame.ip).cloned() else {
            return Err(AzraError::runtime(" bytecode program ran past its end"));
        };
        frame.ip += 1;
        match op {
            Op::Push(value) => stack.push(value),
            Op::Load(name) => {
                let value = frames
                    .last()
                    .and_then(|frame| frame.locals.get(&name))
                    .or_else(|| globals.get(&name))
                    .cloned()
                    .ok_or_else(|| AzraError::runtime(format!("undefined variable `{name}`")))?;
                stack.push(value);
            }
            Op::Store(name) => {
                let value = stack
                    .pop()
                    .ok_or_else(|| AzraError::runtime("stack underflow"))?;
                if frames.len() == 1 {
                    globals.insert(name, value);
                } else {
                    frames.last_mut().unwrap().locals.insert(name, value);
                }
            }
            Op::Pop => {
                stack.pop();
            }
            Op::Neg => {
                let value = stack
                    .pop()
                    .ok_or_else(|| AzraError::runtime("stack underflow"))?;
                let number = as_number(&value)?;
                stack.push(Value::Number(Num::Float(-number.as_f64())));
            }
            Op::Not => {
                let value = stack
                    .pop()
                    .ok_or_else(|| AzraError::runtime("stack underflow"))?;
                stack.push(Value::Bool(!value.is_truthy()));
            }
            Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::Mod
            | Op::Pow
            | Op::Eq
            | Op::Ne
            | Op::Lt
            | Op::Le
            | Op::Gt
            | Op::Ge
            | Op::And
            | Op::Or => {
                let right = stack
                    .pop()
                    .ok_or_else(|| AzraError::runtime("stack underflow"))?;
                let left = stack
                    .pop()
                    .ok_or_else(|| AzraError::runtime("stack underflow"))?;
                stack.push(binary(&op, left, right)?);
            }
            Op::Jump(target) => frames.last_mut().unwrap().ip = target,
            Op::JumpIfFalse(target) => {
                let value = stack
                    .pop()
                    .ok_or_else(|| AzraError::runtime("stack underflow"))?;
                if !value.is_truthy() {
                    frames.last_mut().unwrap().ip = target;
                }
            }
            Op::Output { level, argc } => {
                let start = stack.len().saturating_sub(argc);
                let parts: Vec<String> = stack[start..].iter().map(|v| v.render()).collect();
                stack.truncate(start);
                let text = parts.join(" ");
                let line = match level {
                    OutputLevel::Say => text,
                    OutputLevel::Warn => format!("[WARN] {text}"),
                    OutputLevel::Error => format!("[ERROR] {text}"),
                };
                output.push(line);
            }
            Op::Call { name, argc } => {
                let _ = argc;
                if let Some(chunk) = program.functions.get(&name).cloned() {
                    frames.push(Frame {
                        chunk: Chunk {
                            code: chunk.code.clone(),
                        },
                        ip: 0,
                        locals: HashMap::new(),
                    });
                } else {
                    return Err(AzraError::runtime(format!(
                        "undefined function `{name}` in bytecode VM"
                    )));
                }
            }
            Op::Return => {
                let value = stack.pop().unwrap_or(Value::Null);
                frames.pop();
                if frames.is_empty() {
                    return Ok(output);
                }
                stack.push(value);
            }
            Op::Halt => return Ok(output),
        }
    }
}
