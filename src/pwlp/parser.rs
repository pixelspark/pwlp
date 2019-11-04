use nom::{
	IResult,
	bytes::complete::{tag, take_while1, take_while},
	branch::alt,
	combinator::{map, map_res},
	multi::{fold_many0, separated_list},
	sequence::{pair, preceded, tuple, terminated}
};

use super::instructions;
use super::program::{Program};

#[derive(Clone, Debug, PartialEq)]
enum Node {
	Expression(Expression),
	Special(instructions::Special),
	UserCall(instructions::UserCommand, Expression),
	User(instructions::UserCommand),
	Statements(Vec<Node>),
	Loop(Vec<Node>),
	If(Expression, Vec<Node>)
}

impl Node {
	fn assemble(&self, program: &mut Program) {
		match self {
			Node::Expression(e) => {
				e.assemble(program);
				program.pop(1);
			},
			Node::Special(s) => {
				program.special(*s);
			},
			Node::User(s) => {
				program.user(*s);
			},
			Node::UserCall(s, e) => {
				e.assemble(program);
				program.user(*s);
				program.pop(1);
			},
			Node::Statements(stmts) => {
				for i in stmts.iter() {
					i.assemble(program);
				}
			},
			Node::Loop(stmts) => {
				program.repeat_forever(move |q| {
					for i in stmts.iter() {
						i.assemble(q);
					};
				});
			},
			Node::If(e, ss) => {
				e.assemble(program);
				program.if_not_zero(move |q| {
					for i in ss.iter() {
						i.assemble(q);
					};
				});
				program.pop(1);
			}
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
enum Expression {
	Literal(u32),
	Unary(instructions::Unary, Box<Expression>),
	Binary(Box<Expression>, instructions::Binary, Box<Expression>),
	User(instructions::UserCommand)
}

impl Expression {
	fn assemble(&self, program: &mut Program) {
		match self {
			Expression::Literal(u) => {
				program.push(*u);
			},
			Expression::User(s) => {
				program.user(*s);
			},
			Expression::Unary(op, rhs) => {
				rhs.assemble(program);
				program.unary(*op);
			},
			Expression::Binary(lhs, op, rhs) => {
				lhs.assemble(program);
				rhs.assemble(program);
				program.binary(*op);
			}
		}
	}
}

fn from_hex(input: &str) -> Result<u32, std::num::ParseIntError> {
	u32::from_str_radix(input, 16)
}

fn from_dec(input: &str) -> Result<u32, std::num::ParseIntError> {
	u32::from_str_radix(input, 10)
}

fn is_hex_digit(c: char) -> bool {
	c.is_digit(16)
}

fn is_dec_digit(c: char) -> bool {
	c.is_digit(10)
}

fn sp(input: &str) -> IResult<&str, &str> {
	let chars = " \t\r\n ";
	take_while(move |c| chars.contains(c))(input)
}

fn hex_number(input: &str) -> IResult<&str, u32> {
	map_res(take_while1(is_hex_digit), from_hex)(input)
}

fn dec_number(input: &str) -> IResult<&str, u32> {
	map_res(take_while1(is_dec_digit), from_dec)(input)
}

fn hex_literal(input: &str) -> IResult<&str, u32> {
	let (input, _) = tag("0x")(input)?;
	let (input, num) = hex_number(input)?;
	Ok((input, num))
}

fn literal(input: &str) -> IResult<&str, Expression> {
	let (input, res) = alt((hex_literal, dec_number))(input)?;
	Ok((input, Expression::Literal(res)))
}

fn term(input: &str) -> IResult<&str, Expression> {
	alt((literal, user_expression))(input)
}

fn comparison(input: &str) -> IResult<&str, Expression> {
	let (input, init) = unaries(input)?;

	fold_many0(
		pair(alt((tag(">="), tag("<="), tag(">"), tag("<"))), unaries),
		init,
		| acc, (op, val): (&str, Expression) | {
			match op {
				">=" => Expression::Binary(Box::new(acc), instructions::Binary::GTE, Box::new(val)),
				"<=" => Expression::Binary(Box::new(acc), instructions::Binary::LTE, Box::new(val)),
				">" => Expression::Binary(Box::new(acc), instructions::Binary::GT, Box::new(val)),
				"<" => Expression::Binary(Box::new(acc), instructions::Binary::LT, Box::new(val)),
				_ => unreachable!()
			}
		}
	)(input)
}

fn unaries(input: &str) -> IResult<&str, Expression> {
	alt((
		map(pair(
			alt((tag("-"), tag("!"))),
			unaries
		), |t| {
			match t.0 {
				"-" => Expression::Unary(instructions::Unary::NEG, Box::new(t.1)),
				"!" => Expression::Unary(instructions::Unary::NOT, Box::new(t.1)),
				_ => unreachable!()
			}
		}), 
	binaries))(input)
}

fn binaries(input: &str) -> IResult<&str, Expression> {
	let (input, init) = addition(input)?;

	fold_many0(
		pair(alt((tag("|"), tag("^"), tag("&"))), addition),
		init,
		| acc, (op, val): (&str, Expression) | {
			match op {
				"&" => Expression::Binary(Box::new(acc), instructions::Binary::AND, Box::new(val)),
				"|" => Expression::Binary(Box::new(acc), instructions::Binary::OR, Box::new(val)),
				"^" => Expression::Binary(Box::new(acc), instructions::Binary::XOR, Box::new(val)),
				_ => unreachable!()
			}
		}
	)(input)
}

fn addition(input: &str) -> IResult<&str, Expression> {
	let (input, init) = multiplication(input)?;

	fold_many0(
		pair(alt((tag("+"), tag("-"))), multiplication),
		init,
		| acc, (op, val): (&str, Expression) | {
			if op  == "+" {
				Expression::Binary(Box::new(acc), instructions::Binary::ADD, Box::new(val))
			}
			else {
				Expression::Binary(Box::new(acc), instructions::Binary::SUB, Box::new(val))
			}
		}
	)(input)
}

fn multiplication(input: &str) -> IResult<&str, Expression> {
	let (input, init) = term(input)?;

	fold_many0(
		pair(alt((tag("*"), tag("/"), tag("%"))), term),
		init,
		| acc, (op, val): (&str, Expression) | {
			match op {
				"*" => Expression::Binary(Box::new(acc), instructions::Binary::MUL, Box::new(val)),
				"/" => Expression::Binary(Box::new(acc), instructions::Binary::DIV, Box::new(val)),
				"%" => Expression::Binary(Box::new(acc), instructions::Binary::MOD, Box::new(val)),
				_ => unreachable!()
			}
		}
	)(input)
}

fn expression(input: &str) -> IResult<&str, Expression> {
	comparison(input)
}

fn expression_statement(input: &str) -> IResult<&str, Node> {
	map(expression, |e| Node::Expression(e))(input)
}

fn special_statement(input: &str) -> IResult<&str, Node> {
	alt((
		map(tag("yield"), |_| Node::Special(instructions::Special::YIELD)),
		map(tag("dump"), |_| Node::Special(instructions::Special::DUMP))
	))(input)
}

fn user_statement(input: &str) -> IResult<&str, Node> {
	alt((
		map(tag("blit"), |_| Node::User(instructions::UserCommand::BLIT)),
		map(tuple((tag("set_pixel("), expression, tag(")"))), |t| {
			Node::UserCall(instructions::UserCommand::SET_PIXEL, t.1)
		})
	))(input)
}

fn user_expression(input: &str) -> IResult<&str, Expression> {
	alt((
		map(tag("get_length"), |_| Expression::User(instructions::UserCommand::GET_LENGTH)),
		map(tag("get_wall_time"), |_| Expression::User(instructions::UserCommand::GET_WALL_TIME)),
		map(tag("get_precise_time"), |_| Expression::User(instructions::UserCommand::GET_PRECISE_TIME))
	))(input)
}

fn if_statement(input: &str) -> IResult<&str, Node> {
	map(tuple((tag("if("), expression, tag(")"), sp, tag("{"), program, tag("}"))), |t| {
		if let Node::Statements(ss) = t.5 {
			Node::If(t.1, ss)
		}
		else {
			unreachable!()
		}
	})(input)
}


fn loop_statement(input: &str) -> IResult<&str, Node> {
	map(tuple((tag("loop"), sp, tag("{"), sp, program, tag("}"))), |t| {
		if let Node::Statements(ss) = t.4 {
			Node::Loop(ss)
		}
		else {
			unreachable!()
		}
	})(input)
}

fn statement(input: &str) -> IResult<&str, Node> {
	alt((if_statement, loop_statement, expression_statement, user_statement, special_statement))(input)
}

fn program(input: &str) -> IResult<&str, Node> {
	terminated(map(separated_list(preceded(sp, tag(";")), preceded(sp, statement)), |statements| {
		Node::Statements(statements)
	}), sp)(input)
}

pub fn parse(source: &str) -> Result<Program, String> {
	match program(source) {
		Ok((remainder, n)) => {
			if remainder != "" {
				let err_string = format!("Could not parse, remainder: {}", remainder);
				Err(err_string)
			}
			else {
				let mut p = Program::new();
				n.assemble(&mut p);
				Ok(p)
			}
		},
		Err(x) => {
			let err_string = format!("Parser error: {:?}", x);
			Err(err_string)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn main() {
		assert_eq!(expression("0x0000CC"), Ok(("", Expression::Literal(204))));
		assert_eq!(expression("1337"), Ok(("", Expression::Literal(1337))));
		assert_eq!(expression("1+2"), Ok(("", Expression::Binary(Box::new(Expression::Literal(1)), instructions::Binary::ADD, Box::new(Expression::Literal(2))))));
		
		if let Ok((remainder, n)) = program("loop{if(1+2*3>4){yield};\ndump}") {
			assert_eq!(remainder, "");
			let mut program = Program::new();
			n.assemble(&mut program);
			println!("Program:\n{:?}", &mut program);
		}
	}
}