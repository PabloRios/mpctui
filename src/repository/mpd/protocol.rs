//! Sintaxis del protocolo de texto: armado de comandos, filtros y parseo de líneas.

use std::fmt;

use super::vocabulary::{Verb, tag_name};
use crate::domain::TagFilter;
use crate::repository::error::{Ack, Error, Result};

/// Par `clave: valor` de una respuesta.
pub type Pair = (String, String);

const OK_LINE: &str = "OK";
const ACK_PREFIX: &str = "ACK [";
const GREETING_PREFIX: &str = "OK MPD ";
const PAIR_SEPARATOR: &str = ": ";
const QUOTE: char = '"';
const ESCAPE: char = '\\';

/// Encierra un argumento entre comillas dobles escapando `\` y `"`.
pub fn quote(argument: &str) -> String {
    let mut quoted = String::with_capacity(argument.len() + 2);
    quoted.push(QUOTE);
    for character in argument.chars() {
        if character == QUOTE || character == ESCAPE {
            quoted.push(ESCAPE);
        }
        quoted.push(character);
    }
    quoted.push(QUOTE);
    quoted
}

/// Línea de comando lista para enviar (sin el `\n` final).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command(String);

impl Command {
    pub fn new(verb: Verb) -> Self {
        Self(verb.as_str().to_owned())
    }

    /// Agrega un argumento; siempre va entre comillas, MPD lo acepta igual para números.
    pub fn arg(mut self, argument: impl fmt::Display) -> Self {
        self.0.push(' ');
        self.0.push_str(&quote(&argument.to_string()));
        self
    }

    pub fn optional_arg(self, argument: Option<impl fmt::Display>) -> Self {
        match argument {
            Some(argument) => self.arg(argument),
            None => self,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Command {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOperator {
    Equals,
    Contains,
}

impl FilterOperator {
    fn as_str(self) -> &'static str {
        match self {
            FilterOperator::Equals => "==",
            FilterOperator::Contains => "contains",
        }
    }
}

/// Expresión de filtro (sintaxis 0.21+) sin el escape externo, que agrega `Command::arg`.
/// Devuelve `None` si no hay filtros.
pub fn filter_expression(filters: &[TagFilter], operator: FilterOperator) -> Option<String> {
    let conditions: Vec<String> = filters
        .iter()
        .map(|filter| condition(filter, operator))
        .collect();
    match conditions.as_slice() {
        [] => None,
        [single] => Some(single.clone()),
        many => Some(format!("({})", many.join(" AND "))),
    }
}

fn condition(filter: &TagFilter, operator: FilterOperator) -> String {
    format!(
        "({} {} {})",
        tag_name(filter.tag),
        operator.as_str(),
        quote(&filter.value)
    )
}

/// Verifica que el servidor salude como MPD.
pub fn check_greeting(line: &str) -> Result<()> {
    if line.starts_with(GREETING_PREFIX) {
        Ok(())
    } else {
        Err(Error::Protocol(format!("el servidor no es MPD: {line:?}")))
    }
}

/// Clasificación de una línea de respuesta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseLine {
    Ok,
    Ack(Ack),
    Pair(Pair),
}

pub fn parse_response_line(line: &str) -> Result<ResponseLine> {
    if line == OK_LINE {
        return Ok(ResponseLine::Ok);
    }
    if line.starts_with(ACK_PREFIX) {
        return parse_ack(line)
            .map(ResponseLine::Ack)
            .ok_or_else(|| Error::Protocol(format!("ACK mal formado: {line:?}")));
    }
    parse_pair(line)
        .map(ResponseLine::Pair)
        .ok_or_else(|| Error::Protocol(format!("línea inesperada: {line:?}")))
}

/// Parsea `ACK [50@0] {play} No such song`.
fn parse_ack(line: &str) -> Option<Ack> {
    let rest = line.strip_prefix(ACK_PREFIX)?;
    let (code_and_index, rest) = rest.split_once("] {")?;
    let (code, _command_list_index) = code_and_index.split_once('@')?;
    let (command, message) = rest.split_once('}')?;
    Some(Ack {
        code: code.parse().ok()?,
        command: command.to_owned(),
        message: message.trim_start().to_owned(),
    })
}

fn parse_pair(line: &str) -> Option<Pair> {
    let (key, value) = line.split_once(PAIR_SEPARATOR)?;
    Some((key.to_owned(), value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Tag;

    #[test]
    fn quote_escapes_quotes_and_backslashes() {
        assert_eq!(quote("simple"), r#""simple""#);
        assert_eq!(quote(r#"a "b" \c"#), r#""a \"b\" \\c""#);
    }

    #[test]
    fn command_quotes_every_argument() {
        let command = Command::new(Verb::Add).arg("Rock/Tema 1.mp3");
        assert_eq!(command.as_str(), r#"add "Rock/Tema 1.mp3""#);
        let command = Command::new(Verb::Play).optional_arg(None::<u32>);
        assert_eq!(command.as_str(), "play");
    }

    #[test]
    fn filter_is_escaped_twice_when_sent() {
        let filters = [
            TagFilter::new(Tag::Artist, r#"AC"DC"#),
            TagFilter::new(Tag::Album, "Back"),
        ];
        let expression = filter_expression(&filters, FilterOperator::Equals);
        assert_eq!(
            expression.as_deref(),
            Some(r#"((Artist == "AC\"DC") AND (Album == "Back"))"#)
        );
        assert_eq!(
            Command::new(Verb::Find).optional_arg(expression).as_str(),
            r#"find "((Artist == \"AC\\\"DC\") AND (Album == \"Back\"))""#
        );
    }

    #[test]
    fn filter_expression_shapes() {
        assert_eq!(filter_expression(&[], FilterOperator::Equals), None);
        assert_eq!(
            filter_expression(&[TagFilter::new(Tag::Any, "x")], FilterOperator::Contains)
                .as_deref(),
            Some(r#"(any contains "x")"#)
        );
    }

    #[test]
    fn greeting_must_come_from_mpd() {
        assert!(check_greeting("OK MPD 0.23.5").is_ok());
        assert!(check_greeting("SSH-2.0-OpenSSH").is_err());
    }

    #[test]
    fn response_lines_are_classified() {
        assert_eq!(parse_response_line("OK").unwrap(), ResponseLine::Ok);
        assert_eq!(
            parse_response_line("Title: Uno: Dos").unwrap(),
            ResponseLine::Pair(("Title".into(), "Uno: Dos".into()))
        );
        assert_eq!(
            parse_response_line("ACK [50@0] {play} No such song").unwrap(),
            ResponseLine::Ack(Ack {
                code: 50,
                command: "play".into(),
                message: "No such song".into(),
            })
        );
        assert!(parse_response_line("ACK [roto").is_err());
        assert!(parse_response_line("sin separador").is_err());
    }
}
