use super::instructions;
use super::program::Program;

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
	Expression(Expression),
	Special(instructions::Special),
	UserCall(instructions::UserCommand, Vec<Expression>),
	User(instructions::UserCommand),
	Statements(Vec<Node>),
	Loop(Vec<Node>),
	If(Expression, Vec<Node>),
	IfElse(Expression, Vec<Node>, Vec<Node>),
	Assignment(String, Expression),
	For(String, Expression, Vec<Node>),
}

#[derive(Debug)]
pub struct Scope<'a> {
	variables: Vec<String>,
	level: u32,
	parent: Option<&'a Scope<'a>>
}

impl<'a> Scope<'a> {
	pub fn new() -> Scope<'a> {
		Scope {
			variables: vec![],
			level: 0,
			parent: None
		}
	}

	pub fn nest(&'a self) -> Scope<'a> {
		Scope {
			parent: Some(&self),
			level: 0,
			variables: vec![]
		}
	}

	pub fn unnest(&mut self, program: &mut Program) {
		match self.parent {
			Some(_) => {
				self.assemble_teardown(program);
				self.parent = None
			},
			None => panic!("cannot unnest scope without parent")
		}
	}

	pub fn index_of(&self, variable_name: &String) -> Option<u32> {
		if let Some(i) = self.variables.iter().position(|r| r == variable_name) {
			Some(self.level - 1 - (i as u32))
		}
		else {
			if let Some(p) = self.parent {
				match p.index_of(variable_name) {
					Some(p_index) => {
						Some(p_index + self.level)
					},
					None => None
				}
			}
			else {
				None
			}
		}
	}

	pub fn define_variable(&mut self, variable_name: &String) {
		if self.variables.iter().any(|r| r == variable_name) {
			panic!("variable already defined")
		}

		self.variables.push(variable_name.clone());
		// A variable was already pushed, but we are now counting it througn variables.len()
	}

	pub fn undefine_variable(&mut self, variable_name: &String) {
		if let Some(p) = self.variables.iter().position(|r| r == variable_name) {
			self.variables.remove(p);
		} else {
			panic!("variable was not defined")
		}
	}

	pub(crate) fn assemble_teardown(&self, program: &mut Program) {
		if !self.variables.is_empty() {
			program.pop(self.variables.len() as u8);
		}
	}
}

impl Node {
	pub fn assemble(&self, program: &mut Program, scope: &mut Scope) {
		match self {
			Node::Expression(e) => {
				e.assemble(program, scope);
				program.pop(1);
				scope.level -= 1;
			}
			Node::Special(s) => {
				program.special(*s);
			}
			Node::User(s) => {
				program.user(*s);
			}
			Node::UserCall(s, e) => {
				match s {
					instructions::UserCommand::SET_PIXEL => {
						for (n, param) in e.iter().enumerate() {
							param.assemble(program, scope);
							for _ in 0..n {
								program.unary(instructions::Unary::SHL8);
							}

							if n > 0 {
								program.or();
							}
						}
					}
					_ => {
						for param in e.iter() {
							param.assemble(program, scope);
						}
					}
				}
				program.user(*s);
				program.pop(1);
				scope.level -= (e.len()) as u32;
			}
			Node::Statements(stmts) => {
				for i in stmts.iter() {
					i.assemble(program, scope);
				}
			}
			Node::Loop(stmts) => {
				program.repeat_forever(|q| {
					let mut child_scope = scope.nest();
					for i in stmts.iter() {
						i.assemble(q, &mut child_scope);
					}
					child_scope.unnest(q);
				});
			}
			Node::For(variable_name, expression, stmts) => {
				expression.assemble(program, scope);
				scope.define_variable(variable_name);
				program.repeat(|q| {
					let mut child_scope = scope.nest();
					for i in stmts.iter() {
						i.assemble(q, &mut child_scope);
					}
					child_scope.unnest(q);
				});

				// Undefine variable
				scope.undefine_variable(variable_name);
				scope.level -= 1;
				program.pop(1);
			}
			Node::If(e, ss) => {
				let old_level = scope.level;
				e.assemble(program, scope);
				program.if_not_zero(|q| {
					for i in ss.iter() {
						i.assemble(q, scope);
					}
				});
				program.pop(1);
				scope.level = old_level;
			}
			Node::IfElse(e, if_statements, else_statements) => {	
				let old_level = scope.level;
				e.assemble(program, scope);
				program.if_not_zero(|q| {
					for i in if_statements.iter() {
						i.assemble(q, scope);
					}
				});
				program.if_zero(|q| {
					for i in else_statements.iter() {
						i.assemble(q, scope);
					}
				});
				program.pop(1);
				scope.level = old_level;
			}
			Node::Assignment(variable_name, expression) => {
				expression.assemble(program, scope);
				scope.define_variable(variable_name); // Value left on the stack but cleaned up later by Scope::assemble_teardown
			}
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
	Literal(u32),
	Unary(instructions::Unary, Box<Expression>),
	Binary(Box<Expression>, instructions::Binary, Box<Expression>),
	User(instructions::UserCommand),
	UserCall(instructions::UserCommand, Vec<Expression>),
	Load(String),
}

impl Expression {
	fn assemble(&self, program: &mut Program, scope: &mut Scope) {
		// If we can be simplified to a constant expression, do that!
		if let Some(c) = self.const_value() {
			program.push(c);
			scope.level += 1;
			return;
		}

		match self {
			Expression::Literal(u) => {
				program.push(*u);
				scope.level += 1;
			}
			Expression::User(s) => {
				program.user(*s);
				scope.level += 1;
			}
			Expression::UserCall(s, e) => {
				let old_level = scope.level;
				for param in e.iter() {
					param.assemble(program, scope);
				}
				program.user(*s);
				scope.level = old_level + 1;
			}
			Expression::Unary(op, rhs) => {
				rhs.assemble(program, scope);
				program.unary(*op);
			}
			Expression::Binary(lhs, op, rhs) => {
				lhs.assemble(program, scope);
				rhs.assemble(program, scope);
				program.binary(*op);
				scope.level -= 1;
			}
			Expression::Load(variable_name) => {
				if let Some(relative) = scope.index_of(variable_name) {
					println!("Index of {} is {}", variable_name, relative);
					program.peek(relative as u8);
					scope.level += 1;
				} else {
					panic!("variable not found: {}", variable_name)
				}
			}
		}
	}

	fn const_value(&self) -> Option<u32> {
		match &self {
			Expression::Literal(u) => Some(*u),
			Expression::UserCall(_, _) | Expression::User(_) => None,
			Expression::Load(_var_name) => None,
			Expression::Binary(lhs, op, rhs) => {
				if let (Some(lhc), Some(rhc)) = (lhs.const_value(), rhs.const_value()) {
					match op {
						instructions::Binary::ADD => Some(lhc.overflowing_add(rhc).0),
						instructions::Binary::SUB => Some(lhc.overflowing_sub(rhc).0),
						instructions::Binary::DIV => Some(lhc.overflowing_div(rhc).0),
						instructions::Binary::MUL => Some(lhc.overflowing_mul(rhc).0),
						instructions::Binary::MOD => Some(lhc % rhc),
						instructions::Binary::EQ => Some(if lhc == rhc { 1 } else { 0 }),
						instructions::Binary::NEQ => Some(if lhc != rhc { 1 } else { 0 }),
						instructions::Binary::LT => Some(if lhc < rhc { 1 } else { 0 }),
						instructions::Binary::LTE => Some(if lhc <= rhc { 1 } else { 0 }),
						instructions::Binary::GT => Some(if lhc > rhc { 1 } else { 0 }),
						instructions::Binary::GTE => Some(if lhc >= rhc { 1 } else { 0 }),
						instructions::Binary::OR => Some(lhc | rhc),
						instructions::Binary::XOR => Some(lhc ^ rhc),
						instructions::Binary::AND => Some(lhc & rhc),
					}
				} else {
					None
				}
			}

			Expression::Unary(op, rhs) => {
				if let Some(c) = rhs.const_value() {
					match op {
						instructions::Unary::INC => Some(c.overflowing_add(1).0),
						instructions::Unary::DEC => Some(c.overflowing_sub(1).0),
						instructions::Unary::NOT => Some(!c),
						instructions::Unary::NEG => None,  // TODO
						instructions::Unary::SHL8 => None, // TODO
						instructions::Unary::SHR8 => None, // TODO
					}
				} else {
					None
				}
			}
		}
	}
}
