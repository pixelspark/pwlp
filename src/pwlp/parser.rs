use nom::{
	branch::alt,
	bytes::complete::{tag, take_while, take_while1},
	combinator::{map, map_res, opt},
	multi::{fold_many0, separated_list},
	sequence::{pair, preceded, terminated, tuple},
	IResult,
};

use super::ast::{Expression, Node, Scope};
use super::instructions;
use super::program::Program;

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

fn variable_name(input: &str) -> IResult<&str, &str> {
	take_while1(|c: char| c.is_alphabetic())(input)
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

fn load_expression(input: &str) -> IResult<&str, Expression> {
	map(variable_name, |v| Expression::Load(v.to_string()))(input)
}

fn bracketed_expression(input: &str) -> IResult<&str, Expression> {
	preceded(tag("("), terminated(expression, tag(")")))(input)
}

fn term(input: &str) -> IResult<&str, Expression> {
	alt((literal, user_expression, load_expression, bracketed_expression))(input)
}

fn comparison(input: &str) -> IResult<&str, Expression> {
	let (input, init) = unaries(input)?;

	fold_many0(
		pair(
			preceded(
				sp,
				terminated(
					alt((
						tag(">="),
						tag("<="),
						tag(">"),
						tag("<"),
						tag("=="),
						tag("!="),
					)),
					sp,
				),
			),
			unaries,
		),
		init,
		|acc, (op, val): (&str, Expression)| match op {
			">=" => Expression::Binary(Box::new(acc), instructions::Binary::GTE, Box::new(val)),
			"<=" => Expression::Binary(Box::new(acc), instructions::Binary::LTE, Box::new(val)),
			">" => Expression::Binary(Box::new(acc), instructions::Binary::GT, Box::new(val)),
			"<" => Expression::Binary(Box::new(acc), instructions::Binary::LT, Box::new(val)),
			"==" => Expression::Binary(Box::new(acc), instructions::Binary::EQ, Box::new(val)),
			"!=" => Expression::Binary(Box::new(acc), instructions::Binary::NEQ, Box::new(val)),
			_ => unreachable!(),
		},
	)(input)
}

fn unaries(input: &str) -> IResult<&str, Expression> {
	alt((
		map(pair(alt((tag("-"), tag("!"))), unaries), |t| match t.0 {
			"-" => Expression::Unary(instructions::Unary::NEG, Box::new(t.1)),
			"!" => Expression::Unary(instructions::Unary::NOT, Box::new(t.1)),
			_ => unreachable!(),
		}),
		binaries,
	))(input)
}

fn binaries(input: &str) -> IResult<&str, Expression> {
	let (input, init) = addition(input)?;

	fold_many0(
		pair(alt((tag("|"), tag("^"), tag("&"))), addition),
		init,
		|acc, (op, val): (&str, Expression)| match op {
			"&" => Expression::Binary(Box::new(acc), instructions::Binary::AND, Box::new(val)),
			"|" => Expression::Binary(Box::new(acc), instructions::Binary::OR, Box::new(val)),
			"^" => Expression::Binary(Box::new(acc), instructions::Binary::XOR, Box::new(val)),
			_ => unreachable!(),
		},
	)(input)
}

fn addition(input: &str) -> IResult<&str, Expression> {
	let (input, init) = multiplication(input)?;

	fold_many0(
		pair(alt((tag("+"), tag("-"))), multiplication),
		init,
		|acc, (op, val): (&str, Expression)| {
			if op == "+" {
				Expression::Binary(Box::new(acc), instructions::Binary::ADD, Box::new(val))
			} else {
				Expression::Binary(Box::new(acc), instructions::Binary::SUB, Box::new(val))
			}
		},
	)(input)
}

fn multiplication(input: &str) -> IResult<&str, Expression> {
	let (input, init) = term(input)?;

	fold_many0(
		pair(
			terminated(
				preceded(
					sp,
					alt((tag("*"), tag("/"), tag("%"), tag("<<"), tag(">>"))),
				),
				sp,
			),
			term,
		),
		init,
		|acc, (op, val): (&str, Expression)| match op {
			"*" => Expression::Binary(Box::new(acc), instructions::Binary::MUL, Box::new(val)),
			"/" => Expression::Binary(Box::new(acc), instructions::Binary::DIV, Box::new(val)),
			"%" => Expression::Binary(Box::new(acc), instructions::Binary::MOD, Box::new(val)),
			"<<" | ">>" => {
				if let Expression::Literal(n) = val {
					let unary = match op {
						"<<" => instructions::Unary::SHL8,
						">>" => instructions::Unary::SHR8,
						_ => unreachable!(),
					};

					if (n % 8) == 0 {
						let times = n / 8;
						let mut expr = acc;
						for _ in 0..times {
							expr = Expression::Unary(unary, Box::new(expr))
						}
						expr
					} else {
						panic!("cannot shift by other quantities than multiples of 8")
					}
				} else {
					panic!("cannot shift by dynamic quantities")
				}
			}
			_ => unreachable!(),
		},
	)(input)
}

fn expression(input: &str) -> IResult<&str, Expression> {
	comparison(input)
}

fn expression_statement(input: &str) -> IResult<&str, Node> {
	map(expression, Node::Expression)(input)
}

fn special_statement(input: &str) -> IResult<&str, Node> {
	alt((
		map(tag("yield"), |_| {
			Node::Special(instructions::Special::YIELD)
		}),
		map(tag("dump"), |_| Node::Special(instructions::Special::DUMP)),
	))(input)
}

fn user_statement(input: &str) -> IResult<&str, Node> {
	alt((
		map(tag("blit"), |_| Node::User(instructions::UserCommand::BLIT)),
		map(tuple((tag("set_pixel("), expression, tag(")"))), |t| {
			Node::UserCall(instructions::UserCommand::SET_PIXEL, vec![t.1])
		}),
		// set_pixel(i, r, g, b)
		map(
			tuple((
				tag("set_pixel("),
				preceded(sp, terminated(expression, sp)),
				tag(","),
				preceded(sp, terminated(expression, sp)),
				tag(","),
				preceded(sp, terminated(expression, sp)),
				tag(","),
				preceded(sp, terminated(expression, sp)),
				tag(")"),
			)),
			|t| {
				Node::UserCall(
					instructions::UserCommand::SET_PIXEL,
					vec![t.1, t.3, t.5, t.7],
				)
			},
		),
	))(input)
}

fn user_expression(input: &str) -> IResult<&str, Expression> {
	alt((
		map(tuple((tag("random("), expression, tag(")"))), |t| {
			Expression::UserCall(instructions::UserCommand::RANDOM_INT, vec![t.1])
		}),
		map(tuple((tag("get_pixel("), expression, tag(")"))), |t| {
			Expression::UserCall(instructions::UserCommand::GET_PIXEL, vec![t.1])
		}),
		map(tag("get_length"), |_| {
			Expression::User(instructions::UserCommand::GET_LENGTH)
		}),
		map(tag("get_wall_time"), |_| {
			Expression::User(instructions::UserCommand::GET_WALL_TIME)
		}),
		map(tag("get_precise_time"), |_| {
			Expression::User(instructions::UserCommand::GET_PRECISE_TIME)
		}),
	))(input)
}

fn if_statement(input: &str) -> IResult<&str, Node> {
	map(
		tuple((
			tag("if("),
			preceded(sp, terminated(expression, sp)),
			tag(")"),
			sp,
			tag("{"),
			sp,
			program,
			sp,
			tag("}"),
			sp,
			opt(tuple((tag("else {"), sp, program, sp, tag("}"), sp))),
		)),
		|t| {
			if let Node::Statements(if_statements) = t.6 {
				if let Some(else_tuple) = t.10 {
					if let Node::Statements(else_statements) = else_tuple.2 {
						Node::IfElse(t.1, if_statements, else_statements)
					} else {
						unreachable!()
					}
				} else {
					Node::If(t.1, if_statements)
				}
			} else {
				unreachable!()
			}
		},
	)(input)
}

fn loop_statement(input: &str) -> IResult<&str, Node> {
	map(
		tuple((tag("loop"), sp, tag("{"), sp, program, tag("}"))),
		|t| {
			if let Node::Statements(ss) = t.4 {
				Node::Loop(ss)
			} else {
				unreachable!()
			}
		},
	)(input)
}

fn for_statement(input: &str) -> IResult<&str, Node> {
	map(
		tuple((
			tag("for("),
			preceded(sp, terminated(variable_name, sp)),
			tag("="),
			preceded(sp, terminated(expression, sp)),
			tag(")"),
			sp,
			tag("{"),
			sp,
			program,
			sp,
			tag("}"),
		)),
		|t| {
			if let Node::Statements(ss) = t.8 {
				Node::For(t.1.to_string(), t.3, ss)
			} else {
				unreachable!()
			}
		},
	)(input)
}

fn assigment_statement(input: &str) -> IResult<&str, Node> {
	map(
		tuple((
			variable_name,
			preceded(sp, terminated(tag("="), sp)),
			expression,
		)),
		|t| Node::Assignment(t.0.to_string(), t.2),
	)(input)
}

fn statement(input: &str) -> IResult<&str, Node> {
	alt((
		user_statement,
		special_statement,
		assigment_statement,
		if_statement,
		for_statement,
		loop_statement,
		expression_statement,
	))(input)
}

fn program(input: &str) -> IResult<&str, Node> {
	terminated(
		terminated(
			terminated(
				map(
					separated_list(preceded(sp, tag(";")), preceded(sp, statement)),
					Node::Statements,
				),
				sp,
			),
			opt(tag(";")),
		),
		sp,
	)(input)
}

pub fn parse(source: &str) -> Result<Program, String> {
	match program(source) {
		Ok((remainder, n)) => {
			if remainder != "" {
				let err_string = format!("Could not parse, remainder: {}", remainder);
				Err(err_string)
			} else {
				let mut p = Program::new();
				let mut scope = Scope::new();
				n.assemble(&mut p, &mut scope);
				scope.assemble_teardown(&mut p);
				Ok(p)
			}
		}
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
		assert_eq!(
			expression("1+2"),
			Ok((
				"",
				Expression::Binary(
					Box::new(Expression::Literal(1)),
					instructions::Binary::ADD,
					Box::new(Expression::Literal(2))
				)
			))
		);

		if let Ok((remainder, n)) = program("loop{if(1+2*3>4){yield};\ndump}") {
			assert_eq!(remainder, "");
			let mut program = Program::new();
			let mut scope = Scope::new();
			n.assemble(&mut program, &mut scope);
			scope.assemble_teardown(&mut program);
		}
	}
}
