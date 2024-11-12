use nom::{
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{multispace0, not_line_ending, space1},
    combinator::{map, opt},
    multi::many0,
    sequence::{preceded, terminated, tuple},
    IResult,
};
use std::{collections::HashMap, error::Error};
type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
// pub struct Directive {
//     pub name: String,
//     pub args: Vec<String>,
// }

pub enum Directive {
    Version,
    Endian,
    Protect,
    Encoding,
    Reference,
    Alias,
    Include,
}

impl From<&str> for Directive {
    fn from(value: &str) -> Self {
        match value {
            "VERSION" => Directive::Version,
            "ENDIAN" => Directive::Endian,
            "PROTECT" => Directive::Protect,
            "ENCODING" => Directive::Encoding,
            "REFERENCE" => Directive::Reference,
            "ALIAS" => Directive::Alias,
            "INCLUDE" => Directive::Include,
            _ => panic!("Unknown directive {}", value),
        }
    }
}

#[derive(Debug)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: String,
    pub args: Vec<String>,
}

#[derive(Debug)]
pub enum Line {
    Directive(Directive, Vec<String>),
    FieldDefinition(FieldDefinition),
}

fn is_not_space(c: char) -> bool {
    !c.is_whitespace()
}

fn parse_directive(input: &str) -> IResult<&str, (Directive, Vec<String>)> {
    let (input, _) = tag("/")(input)?;
    let (input, directive_name) = take_while1(is_not_space)(input)?;
    let (input, args) = many0(preceded(space1, take_while1(is_not_space)))(input)?;
    Ok((
        input,
        (
            Directive::from(directive_name),
            args.into_iter().map(String::from).collect(),
        ),
    ))
}
fn parse_field_definition(input: &str) -> IResult<&str, FieldDefinition> {
    if input.starts_with("/") {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }
    let (input, name) = take_while1(is_not_space)(input)?;
    let (input, _) = space1(input)?;
    let (input, field_type) = take_while1(is_not_space)(input)?;
    let (input, args) = many0(preceded(space1, take_while1(is_not_space)))(input)?;
    Ok((
        input,
        FieldDefinition {
            name: name.to_string(),
            field_type: field_type.to_string(),
            args: args.into_iter().map(String::from).collect(),
        },
    ))
}

fn parse_comment(input: &str) -> IResult<&str, &str> {
    let (input, _) = multispace0(input)?;
    preceded(tag("#"), not_line_ending)(input)
}

fn parse_line(input: &str) -> IResult<&str, Line> {
    if input == "" {
        //base case
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Eof,
        )));
    }
    let (input, _) = many0(parse_comment)(input)?;
    let (input, _) = multispace0(input)?;
    let (input, line) = alt((
        map(parse_field_definition, Line::FieldDefinition),
        map(parse_directive, |(x, y)| Line::Directive(x, y)),
    ))(input)?;
    let (input, _) = multispace0(input)?;
    
    Ok((input, line))
}

pub fn parse_format_file(input: &str) -> IResult<&str, Vec<Line>> {
    let (input, lines) = many0(parse_line)(input)?;
    Ok((input, lines))
}
