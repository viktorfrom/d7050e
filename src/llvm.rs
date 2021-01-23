use crate::ast::*;
use crate::memory::*;

extern crate inkwell;

use self::inkwell::{
    builder::Builder,
    context::Context,
    execution_engine::{ExecutionEngine, JitFunction},
    module::Module,
    passes::PassManager,
    types::BasicTypeEnum,
    values::{
        BasicValue, BasicValueEnum, FloatValue, FunctionValue, InstructionValue, IntValue,
        PointerValue,
    },
    FloatPredicate, IntPredicate, OptimizationLevel,
};

use core::panic;
use std::{
    borrow::Borrow,
    collections::HashMap,
    error::Error,
    io::{self, Write},
    iter::Peekable,
    ops::DerefMut,
    str::Chars,
};

type ExprFunc = unsafe extern "C" fn() -> i32;

// // ======================================================================================
// // COMPILER =============================================================================
// // ======================================================================================

// /// Defines the `Expr` compiler.
pub struct Compiler<'a, 'ctx> {
    pub context: &'ctx Context,
    pub builder: &'a Builder<'ctx>,
    pub module: &'a Module<'ctx>,
    pub execution_engine: &'a ExecutionEngine<'ctx>,
    pub fn_value_opt: Option<FunctionValue<'ctx>>,

    variables: HashMap<String, PointerValue<'ctx>>,
    statement: (InstructionValue<'ctx>, bool),
}

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    #[inline]
    fn get_function(&self, name: &str) -> Option<FunctionValue<'ctx>> {
        self.module.get_function(name)
    }

    #[inline]
    fn fn_value(&self) -> FunctionValue<'ctx> {
        self.fn_value_opt.unwrap()
    }

    #[inline]
    fn get_variable(&self, name: &str) -> &PointerValue<'ctx> {
        match self.variables.get(name) {
            Some(var) => var,
            None => panic!("ERROR: Can't find matching variable"),
        }
    }

    fn create_entry_block_alloca(&mut self, name: &str, var_type: bool) -> PointerValue<'ctx> {
        let builder = self.context.create_builder();
        let entry = self.fn_value().get_first_basic_block().unwrap();
        match entry.get_first_instruction() {
            Some(first_instr) => builder.position_before(&first_instr),
            None => builder.position_at_end(entry),
        }
        let alloca: PointerValue;

        if var_type {
            alloca = builder.build_alloca(self.context.bool_type(), name);
        } else {
            alloca = builder.build_alloca(self.context.i32_type(), name);
        }

        self.variables.insert(name.to_string(), alloca);
        alloca
    }

    fn compile_expr(&mut self, expr: &Expr) -> (InstructionValue<'ctx>, bool) {
        match expr.clone() {
            Expr::Let(left, var_type, expr) => (self.compile_let(*left, var_type, *expr), false),

            Expr::VarExpr(var, op, expr) => (self.compile_var_expr(*var, op, *expr), false),

            Expr::If(cond, block) => (self.compile_if(*cond, block), false),
            Expr::IfElse(cond, block1, block2) => {
                (self.compile_if_else(*cond, block1, block2), false)
            }
            Expr::While(cond, block) => (self.compile_while(*cond, block), false),

            Expr::Fn(fn_var, params, ret_type, block) => {
                (self.compile_fn(*fn_var, params, ret_type, block), false)
            }
            Expr::Return(expr) => {
                let var = self.compile_stmt(*expr);
                (self.builder.build_return(Some(&var)), true)
            }

            _ => panic!("Invalid compile expr"),
        }
    }

    fn compile_stmt(&mut self, expr: Expr) -> IntValue<'ctx> {
        match expr.clone() {
            Expr::Int(i) => self.compile_int(i),
            Expr::Bool(b) => self.compile_bool(b),
            Expr::Var(var) => {
                if var != "" {
                    let val = self.get_variable(&var);
                    self.builder.build_load(*val, &var).into_int_value()
                } else {
                    let alloca = self.create_entry_block_alloca("empty", false);
                    let val = self.compile_stmt(Expr::Int(0));
                    self.builder.build_store(alloca, val);

                    let ptr_val = self.get_variable("empty");
                    self.builder.build_load(*ptr_val, &var).into_int_value()
                }
            }

            Expr::BinExpr(l, op, r) => self.compile_bin_expr(*l, op, *r),

            Expr::FnCall(func_name, args) => {
                let name = match *func_name {
                    Expr::Var(v) => v,
                    _ => panic!("asdA"),
                };

                let function = self.module.get_function(&name).unwrap();

                let call = self.builder.build_call(function, &[], &name).try_as_basic_value().left().unwrap();
                let test = match call {
                    value => value.into_int_value(),
                };
                test
            }
            _ => panic!("Invalid compile stmt!"),
        }
    }

    fn compile_let(&mut self, var: Expr, var_type: Type, expr: Expr) -> InstructionValue<'ctx> {
        match var {
            Expr::Var(left) => {
                let ptr_val = match var_type {
                    Type::Int => self.create_entry_block_alloca(&left, false),
                    Type::Bool => self.create_entry_block_alloca(&left, true),
                    _ => panic!("Invalid Let expr type!"),
                };
                let val = self.compile_stmt(expr);
                let store = self.builder.build_store(ptr_val, val);

                store
            }
            _ => panic!("Invalid Expr!"),
        }
    }

    fn compile_var_expr(&mut self, var: Expr, op: Op, expr: Expr) -> InstructionValue<'ctx> {
        let val = self.compile_stmt(expr.clone());

        match (var, op, expr) {
            (Expr::Var(v), Op::AssOp(AssOp::Eq), Expr::Int(_)) => {
                let var_ptr = self.get_variable(&v);

                self.builder.build_store(*var_ptr, val)
            }
            (Expr::Var(v), Op::AssOp(AssOp::Eq), Expr::Bool(_)) => {
                let var_ptr = self.get_variable(&v);

                self.builder.build_store(*var_ptr, val)
            }

            _ => panic!("Invalid Var op!"),
        }
    }

    fn compile_bin_expr(&mut self, l: Expr, op: Op, r: Expr) -> IntValue<'ctx> {
        // println!("var = {:#?}, op = {:#?}, expr = {:#?}, ", l, op, r);
        match (l.clone(), r.clone()) {
            (Expr::Int(_), Expr::Int(_)) => {
                let left = self.compile_stmt(l);
                let right = self.compile_stmt(r);
                self.compile_int_expr(left, op, right)
            }
            (Expr::Var(v), Expr::Int(_)) => {
                if v != "" {
                    let left = self.compile_stmt(l);
                    let right = self.compile_stmt(r);
                    self.compile_int_expr(left, op, right)
                } else {
                    return self.compile_stmt(r);
                }
            }
            (Expr::Bool(_), Expr::Bool(_)) => {
                let left = self.compile_stmt(l);
                let right = self.compile_stmt(r);
                self.compile_bool_expr(left, op, right)
            }
            (Expr::Var(v), Expr::Bool(_)) => {
                if v != "" {
                    let left = self.compile_stmt(l);
                    let right = self.compile_stmt(r);
                    self.compile_bool_expr(left, op, right)
                } else {
                    let a = self.compile_stmt(r);
                    return a;
                }
            }
            (_, Expr::BinExpr(_, _, _)) => self.compile_stmt(r),
            _ => panic!("Invalid Bin expr!"),
        }
    }

    fn compile_bool_expr(
        &mut self,
        l: IntValue<'ctx>,
        op: Op,
        r: IntValue<'ctx>,
    ) -> IntValue<'ctx> {
        match op {
            Op::LogOp(LogOp::And) => self.builder.build_and(l, r, "and"),
            Op::LogOp(LogOp::Or) => self.builder.build_or(l, r, "or"),
            Op::RelOp(RelOp::Eq) => self.builder.build_int_compare(IntPredicate::EQ, l, r, "Eq"),
            Op::RelOp(RelOp::Neq) => self
                .builder
                .build_int_compare(IntPredicate::NE, l, r, "Neq"),
            Op::RelOp(RelOp::Leq) => self
                .builder
                .build_int_compare(IntPredicate::SLE, l, r, "Leq"),
            Op::RelOp(RelOp::Geq) => self
                .builder
                .build_int_compare(IntPredicate::SGE, l, r, "Geq"),
            Op::RelOp(RelOp::Les) => self
                .builder
                .build_int_compare(IntPredicate::SGT, l, r, "Les"),
            Op::RelOp(RelOp::Gre) => self
                .builder
                .build_int_compare(IntPredicate::SLT, l, r, "Gre"),
            _ => panic!("Invalid Bool expr!"),
        }
    }

    fn compile_int_expr(&self, l: IntValue<'ctx>, op: Op, r: IntValue<'ctx>) -> IntValue<'ctx> {
        match op {
            Op::AriOp(AriOp::Add) => self.builder.build_int_add(l, r, "add"),
            Op::AriOp(AriOp::Sub) => self.builder.build_int_sub(l, r, "sub"),
            Op::AriOp(AriOp::Div) => self.builder.build_int_unsigned_div(l, r, "div"),
            Op::AriOp(AriOp::Mul) => self.builder.build_int_mul(l, r, "mul"),
            Op::RelOp(RelOp::Eq) => self.builder.build_int_compare(IntPredicate::EQ, l, r, "Eq"),
            Op::RelOp(RelOp::Neq) => self
                .builder
                .build_int_compare(IntPredicate::NE, l, r, "Neq"),
            Op::RelOp(RelOp::Leq) => self
                .builder
                .build_int_compare(IntPredicate::SLE, l, r, "Leq"),
            Op::RelOp(RelOp::Geq) => self
                .builder
                .build_int_compare(IntPredicate::SGE, l, r, "Geq"),
            Op::RelOp(RelOp::Les) => self
                .builder
                .build_int_compare(IntPredicate::SGT, l, r, "Les"),
            Op::RelOp(RelOp::Gre) => self
                .builder
                .build_int_compare(IntPredicate::SLT, l, r, "Gre"),
            _ => panic!("Invalid Int expr!"),
        }
    }

    fn compile_if(&mut self, cond: Expr, block: Vec<Expr>) -> InstructionValue<'ctx> {
        let cond = self.compile_stmt(cond);
        let then_block = self.context.append_basic_block(self.fn_value(), "then");
        let cont_block = self.context.append_basic_block(self.fn_value(), "cont");

        self.builder
            .build_conditional_branch(cond, then_block, cont_block);
        self.builder.position_at_end(then_block);
        self.compile_block(block);

        self.builder.build_unconditional_branch(cont_block);
        self.builder.position_at_end(cont_block);

        let phi = self.builder.build_phi(self.context.i32_type(), "if");
        phi.add_incoming(&[
            (&self.compile_int(1), then_block),
            (&self.compile_int(0), cont_block),
        ]);
        phi.as_instruction()
    }

    fn compile_if_else(
        &mut self,
        cond: Expr,
        block1: Vec<Expr>,
        block2: Vec<Expr>,
    ) -> InstructionValue<'ctx> {
        let condition = self.compile_stmt(cond);

        let basic_block1 = self.context.append_basic_block(self.fn_value(), "block1");
        let basic_block2 = self.context.append_basic_block(self.fn_value(), "block2");
        let cont_block = self.context.append_basic_block(self.fn_value(), "cont");

        self.builder
            .build_conditional_branch(condition, basic_block1, basic_block2);

        self.builder.position_at_end(basic_block1);
        self.compile_block(block1);
        self.builder.build_unconditional_branch(cont_block);

        self.builder.position_at_end(basic_block2);
        self.compile_block(block2);

        self.builder.build_unconditional_branch(cont_block);

        self.builder.position_at_end(cont_block);
        let phi = self.builder.build_phi(self.context.i32_type(), "iftmp");

        phi.add_incoming(&[
            (&self.compile_int(1), basic_block1),
            (&self.compile_int(0), basic_block2),
        ]);

        phi.as_instruction()
    }

    fn compile_while(&mut self, cond: Expr, block: Vec<Expr>) -> InstructionValue<'ctx> {
        let do_block = self.context.append_basic_block(self.fn_value(), "do");
        let cont_block = self.context.append_basic_block(self.fn_value(), "cont");

        self.builder.build_conditional_branch(
            self.compile_stmt(cond.clone()),
            do_block,
            cont_block,
        );
        self.builder.position_at_end(do_block);
        self.compile_block(block);

        self.builder.build_conditional_branch(
            self.compile_stmt(cond.clone()),
            do_block,
            cont_block,
        );
        self.builder.position_at_end(cont_block);

        let phi = self.builder.build_phi(self.context.i32_type(), "while");
        phi.add_incoming(&[
            (&self.compile_int(0), do_block),
            (&self.compile_int(1), do_block),
        ]);
        phi.as_instruction()
    }

    fn compile_int(&self, int: i32) -> IntValue<'ctx> {
        self.context.i32_type().const_int(int as u64, false)
    }

    fn compile_bool(&self, b: bool) -> IntValue<'ctx> {
        match b {
            true => self.context.bool_type().const_int(1, false),
            false => self.context.bool_type().const_int(0, false),
        }
    }

    fn compile_block(&mut self, block: Vec<Expr>) -> InstructionValue<'ctx> {
        let mut last_cmd: Option<InstructionValue> = None;

        for expr in block.iter() {
            self.statement = self.compile_expr(expr);

            if self.statement.1 {
                return self.statement.0;
            }
            last_cmd = Some(self.statement.0);
        }

        match last_cmd {
            Some(instruction) => instruction,
            None => panic!(),
        }
    }

    fn compile_fn(
        &mut self,
        fn_var: Expr,
        _params: Vec<(Expr, Type)>,
        _ret_type: Type,
        block: Vec<Expr>,
    ) -> InstructionValue<'ctx> {
        let u32_type = self.context.i32_type();
        let fn_type = u32_type.fn_type(&[], false);
        let function = match fn_var {
            Expr::Var(v) => self.module.add_function(&v, fn_type, None),
            _ => panic!("Invalid fn var!"),
        };
        let basic_block = self.context.append_basic_block(function, "entry");

        self.fn_value_opt = Some(function);
        self.builder.position_at_end(basic_block);
        self.compile_block(block)
    }
}

pub fn llvm(ast: Vec<Expr>) -> Result<(), Box<dyn Error>> {
    let context = Context::create();
    let module = context.create_module("llvm-program");
    let builder = context.create_builder();
    let execution_engine = module
        .create_jit_execution_engine(OptimizationLevel::None)
        .unwrap();

    let mut compiler = Compiler {
        context: &context,
        builder: &builder,
        module: &module,
        execution_engine: &execution_engine,
        fn_value_opt: None,
        variables: HashMap::new(),

        statement: (builder.build_return(None), false),
    };

    for expr in ast {
        match expr {
            Expr::Fn(n, p, t, b) => {
                compiler.compile_fn(*n, p, t, b);
            }
            // TODO: add let-statements etc. here? 
            _ => continue,
        }
    }

    compiler.module.print_to_stderr();
    let compiled_program: JitFunction<ExprFunc> =
        unsafe { compiler.execution_engine.get_function("main").ok().unwrap() };

    unsafe {
        println!("llvm-result: {} ", compiled_program.call());
    }

    Ok(())
}

#[cfg(test)]
mod parse_tests {
    use super::*;
    use crate::parser::*;
    use crate::type_checker::*;

    #[test]
    fn test_llvm_return() {
        let p = parser("fn main() -> i32 { return 1 }").unwrap().1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> i32 { return 1 + 1 }").unwrap().1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> bool { return true }").unwrap().1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }
    }

    #[test]
    fn test_llvm_let_bin_expr() {
        let p = parser("fn main() -> i32 { let a: i32 = 1; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> bool { let a: bool = true; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> i32 { let a: i32 = 1 + 1; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> i32 { let a: i32 = 1 - 1; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> i32 { let a: i32 = 1 / 1; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> i32 { let a: i32 = 1 * 1; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> bool { let a: bool = 1 == 1; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> bool { let a: bool = 1 != 1; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> bool { let a: bool = 1 <= 3; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> bool { let a: bool = 4 >= 3; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> bool { let a: bool = 1 < 3; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> bool { let a: bool = 4 > 3; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> i32 { let a: i32 = 1; let b: i32 = a + 2; return b }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> bool { let a: bool = true && true; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> bool { let a: bool = true == true; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> bool { let a: bool = true != false; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> bool { let a: bool = true <= false; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> bool { let a: bool = true >= false; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> bool { let a: bool = true < false; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser("fn main() -> bool { let a: bool = true > false; return a }")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p =
            parser("fn main() -> bool { let a: bool = true; let b: bool = a > false return b }")
                .unwrap()
                .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }
    }

    #[test]
    fn test_llvm_if() {
        let p = parser(" fn main() -> i32 { if true { return 1 }; return 2 } ")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser(" fn main() -> bool { if true { return false }; return true } ")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }
    }

    #[test]
    fn test_llvm_if_else() {
        let p = parser(" fn main() -> i32 { if false { return 1 } else { return 3 }; return 2 } ")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser(
            " fn main() -> bool { if false { return true } else { return false }; return true } ",
        )
        .unwrap()
        .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }
    }

    #[test]
    fn test_llvm_when() {
        let p = parser(" fn main() -> i32 { while true { return 1 }; return 2 } ")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }

        let p = parser(" fn main() -> bool { while true { return false }; return true } ")
            .unwrap()
            .1;
        let t = type_checker(p.clone());

        if t {
            assert!(llvm(p).is_ok());
        }
    }
}
