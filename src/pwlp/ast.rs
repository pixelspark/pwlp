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
	Assignment(String, Expression),
	For(String, Expression, Vec<Node>),
}

pub struct Scope {
	variables: Vec<String>,
	level: u32,
}

impl Scope {
	pub fn new() -> Scope {
		Scope {
			variables: vec![],
			level: 0,
		}
	}

	pub(crate) fn assemble_teardown(&self, program: &mut Program) {
		if self.variables.is_empty() {
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
				scope.level -= 1;
			}
			Node::Statements(stmts) => {
				for i in stmts.iter() {
					i.assemble(program, scope);
				}
			}
			Node::Loop(stmts) => {
				program.repeat_forever(move |q| {
					for i in stmts.iter() {
						i.assemble(q, scope);
					}
				});
			}
			Node::For(variable_name, expression, stmts) => {
				if scope.variables.iter().any(|r| r == variable_name) {
					panic!("variable already defined")
				}

				expression.assemble(program, scope);
				scope.variables.push(variable_name.clone());
				scope.level -= 1;
				program.repeat(|q| {
					for i in stmts.iter() {
						i.assemble(q, scope);
					}
				});

				// Undefine variable
				if let Some(p) = scope.variables.iter().position(|r| r == variable_name) {
					scope.variables.remove(p);
				} else {
					panic!("variable already defined")
				}
				scope.level += 1;
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
			Node::Assignment(variable_name, expression) => {
				if scope.variables.iter().any(|r| r == variable_name) {
					panic!("variable already defined")
				}
				expression.assemble(program, scope);
				scope.variables.push(variable_name.clone());
				scope.level -= 1;
				// Left on the stack but cleaned up later by Scope::assemble_teardown
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
	Load(String),
}

impl Expression {
	fn assemble(&self, program: &mut Program, scope: &mut Scope) {
		match self {
			Expression::Literal(u) => {
				program.push(*u);
				scope.level += 1;
			}
			Expression::User(s) => {
				program.user(*s);
				scope.level += 1;
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
				if let Some(index) = scope.variables.iter().position(|r| r == variable_name) {
					let relative = (scope.level + (scope.variables.len() - index - 1) as u32) as u8;
					program.peek(relative);
					scope.level += 1;
				} else {
					panic!("variable not found")
				}
			}
		}
	}
}
