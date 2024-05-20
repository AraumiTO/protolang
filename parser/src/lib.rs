pub mod span;
pub mod hl;

use std::{iter, slice::Iter};
use std::collections::HashSet;
use std::sync::Mutex;

use itertools::{Itertools, MultiPeek, PeekingNext};
use once_cell::sync::Lazy;
use span::{Positioned, Span};
use tracing::{error, trace, warn};
use crate::hl::Meta;

pub static ENUM_TYPES: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

#[derive(Debug)]
pub struct SyntaxError {
  message: String,
}

impl SyntaxError {
  fn new(message: String) -> Self {
    SyntaxError {
      message
    }
  }
}

#[derive(Debug, Clone)]
pub enum Delimiter {
  BraceOpen,
  BraceClose,
  ParenOpen,
  ParenClose,
}

#[derive(Debug, Clone)]
pub enum Comment {
  LineDoc(String)
}

#[derive(Debug, Clone)]
pub enum Token {
  Meta,
  Model,
  Type,
  Enum,
  Entity,
  Constructor,
  Server,
  Client,

  Required,
  Optional,

  Ident(String),
  Number(i64),
  Delimiter(Delimiter),
  Eq,
  Colon,
  Semi,
  Comma,
  Gt,
  Lt,
  Question,
  Dot,
  String(String),

  Comment(Comment),
  // BlockCommentOpen,
  // BlockCommentLine(String),
  // BlockCommentClose,

  EOF,
}

trait PeekNum<'a, T: Iterator<Item = (usize, char)>> {
  fn peek_num(&mut self, n: usize) -> String;
  fn consume_num(&mut self, n: usize);
}

impl<'a, T: Iterator<Item = (usize, char)>> PeekNum<'a, T> for MultiPeek<T> {
  fn peek_num(&mut self, n: usize) -> String {
    let mut buffer = String::new();
    // let mut peekable_chars = self.peekable();

    for _ in 0..n {
      if let Some(&(_, c)) = self.peek() {
        buffer.push(c);
        // peekable_chars.next(); // Move the iterator
      } else {
        break; // Break if the iterator ends before peeking n characters
      }
    }
    self.reset_peek();

    buffer
  }

  fn consume_num(&mut self, n: usize) {
    for _ in 0..n {
      if let Some(_) = self.next() {} else {
        break; // Break if the iterator ends before peeking n characters
      }
    }
  }
}

pub fn tokenizer(input: &str) -> Result<Vec<Positioned<Token>>, SyntaxError> {
  let mut tokens: Vec<Positioned<Token>> = Vec::new();
  let mut iter = itertools::multipeek(input.chars().enumerate());

  let mut line: usize = 0;
  let mut column: usize = 0;

  let mut is_string = false;
  let mut string = String::new();
  // let mut is_comment = false;

  'char: while let Some((pos, ch)) = iter.next() {
    // if is_comment {
    //   trace!("comment '{}'", ch);
    //   if iter.peek_num("*/".len()) == "*/" {
    //     iter.consume_num("*/".len());
    //     tokens.push(Positioned::new(Token::BlockCommentClose, Span { start: pos, end: pos + "/".len(), line, column }));
    //     is_comment = false;
    //     continue;
    //   }
    //   continue;
    // }

    if is_string {
      if ch == '"' {
        is_string = false;

        let span = Span { start: pos - string.len(), end: pos, line, column };
        tokens.push(Positioned::new(Token::String(string.to_owned()), span));
        string.clear();
      } else {
        string.push(ch);
      }
      continue;
    }

    if ch == '/' && iter.peek_num("//".len()) == "//" {
      let mut comment = String::new();
      while let Some((_, ch)) = iter.next() {
        comment.push(ch);
        column += 1;
        if ch == '\n' {
          line += 0;
          column = 0;

          let len = comment.len();
          tokens.push(Positioned::new(Token::Comment(Comment::LineDoc(comment)), Span { start: pos, end: pos + len, line, column }));
          continue 'char;
        }
      }
    }

    if ch == '/' && iter.peek_num("/".len()) == "/" {
      while let Some((_, ch)) = iter.next() {
        column += 1;
        if ch == '\n' {
          line += 0;
          column = 0;
          continue 'char;
        }
      }
    }

    match ch {
      ch if ch.is_whitespace() => {}
      'm' if iter.peek_num("eta ".len()) == "eta " => {
        iter.consume_num("eta".len());
        let span = Span { start: pos, end: pos + "eta".len(), line, column };
        tokens.push(Positioned::new(Token::Meta, span))
      }
      'm' if iter.peek_num("odel ".len()) == "odel " => {
        iter.consume_num("odel".len());
        let span = Span { start: pos, end: pos + "odel".len(), line, column };
        tokens.push(Positioned::new(Token::Model, span))
      }
      't' if iter.peek_num("ype ".len()) == "ype " => {
        iter.consume_num("ype".len());
        let span = Span { start: pos, end: pos + "ype".len(), line, column };
        tokens.push(Positioned::new(Token::Type, span))
      }
      'e' if iter.peek_num("num ".len()) == "num " => {
        iter.consume_num("num".len());
        let span = Span { start: pos, end: pos + "ype".len(), line, column };
        tokens.push(Positioned::new(Token::Enum, span))
      }
      'e' if iter.peek_num("ntity ".len()) == "ntity " => {
        iter.consume_num("ntity".len());
        tokens.push(Positioned::new(Token::Entity, Span { start: pos, end: pos + "ntity".len(), line, column }));
      }
      'c' if iter.peek_num("onstructor ".len()) == "onstructor " => {
        iter.consume_num("onstructor".len());
        tokens.push(Positioned::new(Token::Constructor, Span { start: pos, end: pos + "onstructor".len(), line, column }));
      }
      'c' if iter.peek_num("lient ".len()) == "lient " => {
        iter.consume_num("lient".len());
        tokens.push(Positioned::new(Token::Client, Span { start: pos, end: pos + "lient".len(), line, column }));
      }
      's' if iter.peek_num("erver ".len()) == "erver " => {
        iter.consume_num("erver".len());
        tokens.push(Positioned::new(Token::Server, Span { start: pos, end: pos + "erver".len(), line, column }));
      }
      'r' if iter.peek_num("equired ".len()) == "equired " => {
        iter.consume_num("equired".len());
        tokens.push(Positioned::new(Token::Required, Span { start: pos, end: pos + "equired".len(), line, column }));
      }
      'o' if iter.peek_num("ptional ".len()) == "ptional " => {
        iter.consume_num("ptional".len());
        tokens.push(Positioned::new(Token::Optional, Span { start: pos, end: pos + "ptional".len(), line, column }));
      }
      '=' => tokens.push(Positioned::new(Token::Eq, Span { start: pos, end: pos, line, column })),
      '{' => tokens.push(Positioned::new(Token::Delimiter(Delimiter::BraceOpen), Span { start: pos, end: pos, line, column })),
      '}' => tokens.push(Positioned::new(Token::Delimiter(Delimiter::BraceClose), Span { start: pos, end: pos, line, column })),
      '(' => tokens.push(Positioned::new(Token::Delimiter(Delimiter::ParenOpen), Span { start: pos, end: pos, line, column })),
      ')' => tokens.push(Positioned::new(Token::Delimiter(Delimiter::ParenClose), Span { start: pos, end: pos, line, column })),
      ':' => tokens.push(Positioned::new(Token::Colon, Span { start: pos, end: pos, line, column })),
      ';' => tokens.push(Positioned::new(Token::Semi, Span { start: pos, end: pos, line, column })),
      ',' => tokens.push(Positioned::new(Token::Comma, Span { start: pos, end: pos, line, column })),
      '>' => tokens.push(Positioned::new(Token::Gt, Span { start: pos, end: pos, line, column })),
      '<' => tokens.push(Positioned::new(Token::Lt, Span { start: pos, end: pos, line, column })),
      '?' => tokens.push(Positioned::new(Token::Question, Span { start: pos, end: pos, line, column })),
      '.' => tokens.push(Positioned::new(Token::Dot, Span { start: pos, end: pos, line, column })),
      '"' => {
        is_string = true;
      }
      // '/' if iter.peek_num("**".len()) == "**" => {
      //   iter.consume_num("**".len());
      //   tokens.push(Positioned::new(Token::BlockCommentOpen, Span { start: pos, end: pos + "**".len(), line, column }));
      //   is_comment = true;
      // },
      // '*' if iter.peek_num("/".len()) == "/" => {
      //   iter.consume_num("/".len());
      //   tokens.push(Positioned::new(Token::BlockCommentClose, Span { start: pos, end: pos + "/".len(), line, column }));
      //   is_comment = false;
      // },
      '0'..='9' => {
        let s = iter::once(ch)
          .chain(iter::from_fn(|| {
            iter.by_ref().peeking_next(|(_, s)| s.is_ascii_digit()).map(|(_, c)| c)
          }))
          .collect::<String>();
        let n: i64 = s
          .parse()
          .unwrap();

        let span = Span { start: pos, end: pos + s.len() - 1, line, column };
        tokens.push(Positioned::new(Token::Number(n), span))
      }
      ch if ch.is_ascii_alphabetic() || ch == '_' => {
        let s = iter::once(ch)
          .chain(iter::from_fn(|| {
            iter.by_ref().peeking_next(|(_, s)| s.is_ascii_alphanumeric() || *s == '_').map(|(_, c)| c)
          }))
          .collect::<String>();

        let span = Span { start: pos, end: pos + s.len() - 1, line, column };
        tokens.push(Positioned::new(Token::Ident(s), span))
      }
      _ => return Err(SyntaxError::new(format!("unrecognized character {} (position: {}, {}:{})", ch, pos, line, column))),
    }

    if !ch.is_ascii_control() {
      column += 1;
    }
    if ch == '\n' {
      line += 1;
      column = 0;
    }
  }

  // tokens.push(Token::EOF);
  Ok(tokens)
}

#[derive(Debug)]
pub struct Program {
  pub body: Vec<ProgramItem>,
}

#[derive(Debug)]
pub enum ProgramItem {
  Meta(MetaDeclaration),
  Model(ModelDeclaration),
  Type(TypeDeclaration),
  Enum(EnumDeclaration),
}

#[derive(Debug, Clone)]
pub struct CommentLit(pub String);

#[derive(Debug, Clone)]
pub struct Identifier(pub String);

#[derive(Debug)]
pub struct StringLit(pub String);

#[derive(Debug)]
pub struct NumberLit(pub i64);

#[derive(Debug)]
pub struct BooleanLit(pub bool);

#[derive(Debug)]
pub struct MetaDeclaration {
  pub key: Positioned<Identifier>,
  pub value: Positioned<StringLit>,
}

#[derive(Debug)]
pub struct ModelDeclaration {
  pub name: Positioned<Identifier>,
  pub id: Positioned<NumberLit>,
  pub body: Vec<ModelItem>,
  pub meta: Vec<MetaDeclaration>,
  pub comments: Vec<CommentLit>,
}

#[derive(Debug)]
pub enum ModelItem {
  Entity(EntityDeclaration),
  Constructor(ConstructorDeclaration),
  ServerMethod(ServerMethodDeclaration),
  ClientMethod(ClientMethodDeclaration),
}

#[derive(Debug)]
pub struct TypeDeclaration {
  pub name: Positioned<Identifier>,
  pub fields: Vec<FieldDeclaration>,
  pub meta: Vec<MetaDeclaration>,
  pub comments: Vec<CommentLit>,
}

#[derive(Debug)]
pub struct EnumDeclaration {
  pub name: Positioned<Identifier>,
  pub repr: Positioned<Identifier>,
  pub variants: Vec<VariantDeclaration>,
  pub meta: Vec<MetaDeclaration>,
  pub comments: Vec<CommentLit>,
}

#[derive(Debug)]
pub struct EntityDeclaration {
  pub name: Positioned<Identifier>,
  pub required: Option<Positioned<Token>>,
  pub comments: Vec<CommentLit>,
}

#[derive(Debug)]
pub struct ConstructorDeclaration {
  pub fields: Vec<FieldDeclaration>,
  pub meta: Vec<MetaDeclaration>,
  pub comments: Vec<CommentLit>,
}

#[derive(Debug)]
pub struct ServerMethodDeclaration {
  pub name: Positioned<Identifier>,
  pub params: Vec<ParamDeclaration>,
  pub id: Positioned<NumberLit>,
  pub comments: Vec<CommentLit>,
}

#[derive(Debug)]
pub struct ClientMethodDeclaration {
  pub name: Positioned<Identifier>,
  pub params: Vec<ParamDeclaration>,
  pub id: Positioned<NumberLit>,
  pub comments: Vec<CommentLit>,
}

#[derive(Debug)]
pub struct FieldDeclaration {
  pub name: Positioned<Identifier>,
  pub kind: Type,
  pub position: Positioned<NumberLit>,
  pub comments: Vec<CommentLit>,
}

#[derive(Debug)]
pub struct VariantDeclaration {
  pub name: Positioned<Identifier>,
  pub value: Positioned<NumberLit>,
  pub comments: Vec<CommentLit>,
}

#[derive(Debug)]
pub struct ParamDeclaration {
  pub name: Positioned<Identifier>,
  pub kind: Type,
}

#[derive(Debug)]
pub enum Type {
  Ident {
    ty: Positioned<Identifier>,
    nullable: Option<Positioned<Token>>,
  },
  Generic {
    ty: Positioned<Identifier>,
    nullable: Option<Positioned<Token>>,
    params: Vec<Type>,
  },
  Nested {
    ty: Box<Type>,
    inner: Box<Type>,
  },
}

pub fn parse_program(input: &mut MultiPeek<Iter<Positioned<Token>>>) -> Result<Program, SyntaxError> {
  let mut body = Vec::new();
  let mut comments = Vec::new();
  while let Some(token) = input.peek() {
    match &token.value {
      Token::Comment(comment) => {
        trace!("comment {:?}", comment);
        match comment {
          Comment::LineDoc(body) => comments.push(CommentLit(body.to_owned())),
          _ => {}
        }
        input.next();
      }
      Token::Meta => {
        body.push(ProgramItem::Meta(parse_meta(input).unwrap()));
        comments.clear();
      }
      Token::Model => {
        body.push(ProgramItem::Model(parse_model(input, &comments).unwrap()));
        comments.clear();
      }
      Token::Type => {
        body.push(ProgramItem::Type(parse_type(input, &comments).unwrap()));
        comments.clear();
      }
      Token::Enum => {
        body.push(ProgramItem::Enum(parse_enum(input, &comments).unwrap()));
        comments.clear();
      }
      _ => return Err(SyntaxError::new(format!("unrecognized token {:?}", token))),
    }
  }

  Ok(Program {
    body
  })
}

pub fn parse_meta(input: &mut MultiPeek<Iter<Positioned<Token>>>) -> Result<MetaDeclaration, SyntaxError> {
  let token = input.next().unwrap();
  match &token.value {
    Token::Meta => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Meta", token)))
  };

  let token = input.next().unwrap();
  let key = match &token.value {
    Token::Ident(value) => token.span.wrap(Identifier(value.to_owned())),
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Ident", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Eq => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Eq", token)))
  };

  let token = input.next().unwrap();
  let value = match &token.value {
    Token::String(value) => token.span.wrap(StringLit(value.to_owned())),
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected String", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Semi => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Semi", token)))
  };

  Ok(MetaDeclaration {
    key,
    value,
  })
}

macro_rules! consume_token {
  ($input:expr, $token:pat) => {{
    let token = $input.next().unwrap();
    match &token.value {
      $token => token,
      _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected {}", token, stringify!($token))))
    }
  }};
}

macro_rules! consume_ident {
  ($input:expr) => {{
    let token = $input.next().unwrap();
    match &token.value {
      Token::Ident(value) => token.span.wrap(Identifier(value.to_owned())),
      _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected {}", token, stringify!(Token::Ident))))
    }
  }};
}

macro_rules! consume_number {
  ($input:expr) => {{
    let token = $input.next().unwrap();
    match &token.value {
      Token::Number(value) => token.span.wrap(NumberLit(value.to_owned())),
      _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected {}", token, stringify!(Token::Number))))
    }
  }};
}

pub fn parse_model(input: &mut MultiPeek<Iter<Positioned<Token>>>, comments: &[CommentLit]) -> Result<ModelDeclaration, SyntaxError> {
  consume_token!(input, Token::Model);
  let name = consume_ident!(input);
  consume_token!(input, Token::Eq);
  let id = consume_number!(input);
  consume_token!(input, Token::Delimiter(Delimiter::BraceOpen));

  let mut meta = Vec::new();
  let mut body = Vec::new();
  let mut item_comments = Vec::new();
  while let Some(token) = input.peek() {
    trace!("body: {:?}", token.value);
    match &token.value {
      Token::Comment(comment) => {
        trace!("comment {:?}", comment);
        match comment {
          Comment::LineDoc(body) => item_comments.push(CommentLit(body.to_owned())),
          _ => {}
        }
        input.next();
      }
      Token::Meta => {
        meta.push(parse_meta(input).unwrap());
        item_comments.clear();
      }
      Token::Required | Token::Entity => {
        body.push(ModelItem::Entity(parse_entity(input, &item_comments).unwrap()));
        item_comments.clear();
      }
      Token::Constructor => {
        body.push(ModelItem::Constructor(parse_constructor(input, &item_comments).unwrap()));
        item_comments.clear();
      }
      Token::Server => {
        body.push(ModelItem::ServerMethod(parse_server_method(input, &item_comments).unwrap()));
        item_comments.clear();
      }
      Token::Client => {
        body.push(ModelItem::ClientMethod(parse_client_method(input, &item_comments).unwrap()));
        item_comments.clear();
      }
      Token::Delimiter(Delimiter::BraceClose) => break,
      _ => return Err(SyntaxError::new(format!("unrecognized token {:?}", token))),
    }
  }

  consume_token!(input, Token::Delimiter(Delimiter::BraceClose));

  Ok(ModelDeclaration {
    name,
    id,
    body,
    meta,
    comments: comments.to_vec(),
  })
}

pub fn parse_entity(input: &mut MultiPeek<Iter<Positioned<Token>>>, comments: &[CommentLit]) -> Result<EntityDeclaration, SyntaxError> {
  let token = input.next().unwrap();
  let required = match &token.value {
    Token::Required => Some(token.to_owned()),
    Token::Entity => None,
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Required or Entity", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Entity => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Entity", token)))
  };

  let token = input.next().unwrap();
  let name = match &token.value {
    Token::Ident(value) => token.span.wrap(Identifier(value.to_owned())),
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Ident", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Semi => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Semi", token)))
  };

  Ok(EntityDeclaration {
    name,
    required,
    comments: comments.to_vec(),
  })
}

pub fn parse_constructor(input: &mut MultiPeek<Iter<Positioned<Token>>>, comments: &[CommentLit]) -> Result<ConstructorDeclaration, SyntaxError> {
  let token = input.next().unwrap();
  match &token.value {
    Token::Constructor => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Constructor", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Delimiter(Delimiter::BraceOpen) => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected BraceOpen", token)))
  };

  let mut meta = Vec::new();
  let mut fields = Vec::new();
  let mut field_comments = Vec::new();
  while let Some(token) = input.peek() {
    match &token.value {
      Token::Comment(comment) => {
        trace!("comment {:?}", comment);
        match comment {
          Comment::LineDoc(body) => field_comments.push(CommentLit(body.to_owned())),
          _ => {}
        }
        input.next();
      }
      Token::Meta => {
        meta.push(parse_meta(input).unwrap());
        field_comments.clear();
      }
      Token::Ident(_) => {
        fields.push(parse_field(input, &field_comments).unwrap());
        field_comments.clear();
      }
      Token::Delimiter(Delimiter::BraceClose) => break,
      _ => {}
      // _ => return Err(SyntaxError::new(format!("unrecognized token {:?}", token))),
    }
  }

  let token = input.next().unwrap();
  match &token.value {
    Token::Delimiter(Delimiter::BraceClose) => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected BraceClose", token)))
  };

  Ok(ConstructorDeclaration {
    fields,
    meta,
    comments: comments.to_vec(),
  })
}

pub fn parse_field(input: &mut MultiPeek<Iter<Positioned<Token>>>, comments: &[CommentLit]) -> Result<FieldDeclaration, SyntaxError> {
  let token = input.next().unwrap();
  let name = match &token.value {
    Token::Ident(value) => token.span.wrap(Identifier(value.to_owned())),
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Ident", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Colon => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Colon", token)))
  };

  let kind = parse_type_2(input).unwrap();

  let token = input.next().unwrap();
  match &token.value {
    Token::Eq => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Eq", token)))
  };

  let token = input.next().unwrap();
  let position = match &token.value {
    Token::Number(value) => token.span.wrap(NumberLit(value.to_owned())),
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Number", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Semi => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Semi", token)))
  };

  Ok(FieldDeclaration {
    name,
    kind,
    position,
    comments: comments.to_vec(),
  })
}

pub fn parse_variant(input: &mut MultiPeek<Iter<Positioned<Token>>>, comments: &[CommentLit]) -> Result<VariantDeclaration, SyntaxError> {
  let token = input.next().unwrap();
  let name = match &token.value {
    Token::Ident(value) => token.span.wrap(Identifier(value.to_owned())),
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Ident", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Eq => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Eq", token)))
  };

  let token = input.next().unwrap();
  let value = match &token.value {
    Token::Number(value) => token.span.wrap(NumberLit(value.to_owned())),
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Number", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Semi => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Semi", token)))
  };

  Ok(VariantDeclaration {
    name,
    value,
    comments: comments.to_vec(),
  })
}

pub fn parse_param(input: &mut MultiPeek<Iter<Positioned<Token>>>) -> Result<ParamDeclaration, SyntaxError> {
  let token = input.next().unwrap();
  let name = match &token.value {
    Token::Ident(value) => token.span.wrap(Identifier(value.to_owned())),
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Ident", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Colon => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Colon", token)))
  };

  let kind = parse_type_2(input).unwrap();

  Ok(ParamDeclaration {
    name,
    kind,
  })
}

pub fn parse_type(input: &mut MultiPeek<Iter<Positioned<Token>>>, comments: &[CommentLit]) -> Result<TypeDeclaration, SyntaxError> {
  let token = input.next().unwrap();
  match &token.value {
    Token::Type => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Type", token)))
  };

  let token = input.next().unwrap();
  let name = match &token.value {
    Token::Ident(value) => token.span.wrap(Identifier(value.to_owned())),
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Ident", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Delimiter(Delimiter::BraceOpen) => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected BraceOpen", token)))
  };

  let mut meta = Vec::new();
  let mut fields = Vec::new();
  let mut field_comments = Vec::new();
  while let Some(token) = input.peek() {
    match &token.value {
      Token::Comment(comment) => {
        trace!("comment {:?}", comment);
        match comment {
          Comment::LineDoc(body) => field_comments.push(CommentLit(body.to_owned())),
          _ => {}
        }
        input.next();
      }
      Token::Meta => {
        meta.push(parse_meta(input).unwrap());
        field_comments.clear();
      }
      Token::Ident(_) => {
        fields.push(parse_field(input, &field_comments).unwrap());
        field_comments.clear();
      }
      Token::Delimiter(Delimiter::BraceClose) => break,
      _ => {}
      // _ => return Err(SyntaxError::new(format!("unrecognized token {:?}", token))),
    }
  }

  let token = input.next().unwrap();
  match &token.value {
    Token::Delimiter(Delimiter::BraceClose) => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected BraceClose", token)))
  };

  Ok(TypeDeclaration {
    name,
    fields,
    meta,
    comments: comments.to_vec(),
  })
}

pub fn parse_enum(input: &mut MultiPeek<Iter<Positioned<Token>>>, comments: &[CommentLit]) -> Result<EnumDeclaration, SyntaxError> {
  let token = input.next().unwrap();
  match &token.value {
    Token::Enum => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Enum", token)))
  };

  let token = input.next().unwrap();
  let name = match &token.value {
    Token::Ident(value) => token.span.wrap(Identifier(value.to_owned())),
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Ident", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Colon => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Colon", token)))
  };

  let token = input.next().unwrap();
  let repr = match &token.value {
    Token::Ident(value) => token.span.wrap(Identifier(value.to_owned())),
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Ident", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Delimiter(Delimiter::BraceOpen) => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected BraceOpen", token)))
  };

  let mut meta = Vec::new();
  let mut variants = Vec::new();
  let mut field_comments = Vec::new();
  while let Some(token) = input.peek() {
    match &token.value {
      Token::Comment(comment) => {
        trace!("comment {:?}", comment);
        match comment {
          Comment::LineDoc(body) => field_comments.push(CommentLit(body.to_owned())),
          _ => {}
        }
        input.next();
      }
      Token::Meta => {
        meta.push(parse_meta(input).unwrap());
        field_comments.clear();
      }
      Token::Ident(_) => {
        variants.push(parse_variant(input, &field_comments).unwrap());
        field_comments.clear();
      }
      Token::Delimiter(Delimiter::BraceClose) => break,
      _ => {}
      // _ => return Err(SyntaxError::new(format!("unrecognized token {:?}", token))),
    }
  }

  let token = input.next().unwrap();
  match &token.value {
    Token::Delimiter(Delimiter::BraceClose) => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected BraceClose", token)))
  };

  Ok(EnumDeclaration {
    name,
    repr,
    variants,
    meta,
    comments: comments.to_vec(),
  })
}

pub fn parse_server_method(input: &mut MultiPeek<Iter<Positioned<Token>>>, comments: &[CommentLit]) -> Result<ServerMethodDeclaration, SyntaxError> {
  let token = input.next().unwrap();
  match &token.value {
    Token::Server => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Server", token)))
  };

  let token = input.next().unwrap();
  let name = match &token.value {
    Token::Ident(value) => token.span.wrap(Identifier(value.to_owned())),
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Ident", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Delimiter(Delimiter::ParenOpen) => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected ParenOpen", token)))
  };

  let mut params = Vec::new();
  while let Some(token) = input.peek() {
    match &token.value {
      Token::Ident(_) => {
        params.push(parse_param(input).unwrap());

        let token = input.peek().unwrap();
        match &token.value {
          Token::Comma => {
            input.next();

            let token = input.peek().unwrap();
            match &token.value {
              Token::Delimiter(Delimiter::ParenClose) => return Err(SyntaxError::new(format!("unexpected token {:?}", token))),
              _ => {}
            }
          }
          Token::Delimiter(Delimiter::ParenClose) => {}
          _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Comma or ParenClose", token))),
        }
        input.reset_peek();
      }
      Token::Delimiter(Delimiter::ParenClose) => break,
      _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Ident or ParenClose", token))),
    }
  }

  let token = input.next().unwrap();
  match &token.value {
    Token::Delimiter(Delimiter::ParenClose) => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected ParenClose", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Eq => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Eq", token)))
  };

  let token = input.next().unwrap();
  let id = match &token.value {
    Token::Number(value) => token.span.wrap(NumberLit(value.to_owned())),
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Number", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Semi => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Semi", token)))
  };

  Ok(ServerMethodDeclaration {
    name,
    params,
    id,
    comments: comments.to_vec(),
  })
}

pub fn parse_client_method(input: &mut MultiPeek<Iter<Positioned<Token>>>, comments: &[CommentLit]) -> Result<ClientMethodDeclaration, SyntaxError> {
  let token = input.next().unwrap();
  match &token.value {
    Token::Client => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Client", token)))
  };

  let token = input.next().unwrap();
  let name = match &token.value {
    Token::Ident(value) => token.span.wrap(Identifier(value.to_owned())),
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Ident", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Delimiter(Delimiter::ParenOpen) => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected ParenOpen", token)))
  };

  let mut params = Vec::new();
  while let Some(token) = input.peek() {
    match &token.value {
      Token::Ident(_) => {
        params.push(parse_param(input).unwrap());

        let token = input.peek().unwrap();
        match &token.value {
          Token::Comma => {
            input.next();

            let token = input.peek().unwrap();
            match &token.value {
              Token::Delimiter(Delimiter::ParenClose) => return Err(SyntaxError::new(format!("unexpected token {:?}", token))),
              _ => {}
            }
          }
          Token::Delimiter(Delimiter::ParenClose) => {}
          _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Comma or ParenClose", token))),
        }
        input.reset_peek();
      }
      Token::Delimiter(Delimiter::ParenClose) => break,
      _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Ident or ParenClose", token))),
    }
  }

  let token = input.next().unwrap();
  match &token.value {
    Token::Delimiter(Delimiter::ParenClose) => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected ParenClose", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Eq => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Eq", token)))
  };

  let token = input.next().unwrap();
  let id = match &token.value {
    Token::Number(value) => token.span.wrap(NumberLit(value.to_owned())),
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Number", token)))
  };

  let token = input.next().unwrap();
  match &token.value {
    Token::Semi => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Semi", token)))
  };

  Ok(ClientMethodDeclaration {
    name,
    params,
    id,
    comments: comments.to_vec(),
  })
}

pub fn parse_type_2(input: &mut MultiPeek<Iter<Positioned<Token>>>) -> Result<Type, SyntaxError> {
  let mut current_generic = None;
  let mut current_nested_type = None;
  let mut current_ident = None;
  let mut nullable_token = None;

  while let Some(token) = input.peek() {
    match &token.value {
      Token::Question => {
        if nullable_token.is_some() {
          return Err(SyntaxError::new(format!("unexpected token {:?}, nullable is already present", token)));
        }

        nullable_token = Some(token.wrap(token.value.to_owned()));
        trace!("PARSED NULLABLE: {:?}", nullable_token);
        input.next();
      }
      Token::Ident(ident) => {
        if current_ident.is_some() {
          return Err(SyntaxError::new(format!("unexpected token {:?}, identifier is already present", token)));
        }

        current_ident = Some(token.wrap(Identifier(ident.to_owned())));
        trace!("PARSED TYPE IDENT: {:?}", current_ident);
        input.next();
      }
      Token::Dot => {
        if current_nested_type.is_some() {
          return Err(SyntaxError::new(format!("unexpected token {:?}, nested type is already present", token)));
        }
        if current_ident.is_none() {
          return Err(SyntaxError::new(format!("unexpected token {:?}, expected type identifier", token)));
        }

        input.next();
        current_nested_type = Some(parse_type_2(input).unwrap());
        trace!("PARSED NESTED TYPE IDENT: {:?}", current_ident);
      }
      Token::Lt => {
        if current_generic.is_some() {
          return Err(SyntaxError::new(format!("unexpected token {:?}, generic is already present", token)));
        }
        if current_ident.is_none() {
          return Err(SyntaxError::new(format!("unexpected token {:?}, expected type identifier", token)));
        }

        trace!("PARSE GENERIC ENTER");
        let params = parse_type_2_generic_params(input).unwrap();
        trace!("PARSE GENERIC {:?}", params);

        current_generic = Some(params);
      }
      _ => {
        let token = token.to_owned();
        input.reset_peek();

        let get_type = || -> Option<Type> {
          if let Some(current_generic) = current_generic {
            return Some(Type::Generic {
              ty: current_ident.unwrap(),
              nullable: nullable_token,
              params: current_generic,
            });
          }
          if let Some(current_ident) = current_ident {
            return Some(Type::Ident {
              ty: current_ident,
              nullable: nullable_token,
            });
          }
          None
        };

        if let Some(current_nested_type) = current_nested_type {
          return Ok(Type::Nested {
            ty: Box::new(get_type().unwrap()),
            inner: Box::new(current_nested_type),
          });
        }

        if let Some(ty) = get_type() {
          return Ok(ty);
        }

        return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Question, Ident, or Lt", token)));
      }
    }
  }
  return Err(SyntaxError::new(format!("unexpected eof")));
}

pub fn parse_type_2_generic_params(input: &mut MultiPeek<Iter<Positioned<Token>>>) -> Result<Vec<Type>, SyntaxError> {
  let token = input.next().unwrap();
  match &token.value {
    Token::Lt => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Lt", token)))
  };

  let mut params = Vec::new();
  while let Some(token) = input.peek() {
    match &token.value {
      Token::Ident(_) => {
        input.reset_peek();
        params.push(parse_type_2(input).unwrap());

        let token = input.peek().unwrap();
        match &token.value {
          Token::Comma => {
            input.next();

            let token = input.peek().unwrap();
            match &token.value {
              Token::Gt => return Err(SyntaxError::new(format!("unexpected token {:?}", token))),
              _ => {}
            }
          }
          Token::Gt => {}
          _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Comma or Gt", token))),
        }
        input.reset_peek();
      }
      Token::Gt => break,
      _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Ident or Gt", token))),
    }
  }

  let token = input.next().unwrap();
  match &token.value {
    Token::Gt => {}
    _ => return Err(SyntaxError::new(format!("unrecognized token {:?}, expected Gt", token)))
  };

  Ok(params)
}

pub fn type_to_hl(kind: &Type) -> String {
  match kind {
    Type::Ident { ty, nullable } => {
      format!("{}{}", ty.value.0.to_owned(), if nullable.is_some() { "?" } else { "" })
    }
    Type::Generic { ty, nullable, params } => {
      let params = params.iter().map(|it| type_to_hl(it)).join(", ");
      format!("{}<{}>{}", ty.value.0.to_owned(), params, if nullable.is_some() { "?" } else { "" })
    }
    Type::Nested { ty, inner } => {
      format!("{}.{}", type_to_hl(ty), type_to_hl(inner))
    }
  }
}

pub fn type_to_hl_codec(kind: &Type) -> String {
  match kind {
    Type::Ident { ty, nullable } => {
      let name = ty.value.0.to_owned();
      let info = if ENUM_TYPES.lock().unwrap().contains(&name) { "EnumCodecInfo" } else { "TypeCodecInfo" };
      format!("new {}({},{})", info, name, if nullable.is_some() { "true" } else { "false" })
    }
    Type::Generic { ty, nullable, params } => {
      let main = ty.value.0.to_owned();
      match main.as_str() {
        "List" => format!("new CollectionCodecInfo({},{},1)", type_to_hl_codec(&params[0]), if nullable.is_some() { "true" } else { "false" }),
        "Map" => format!("new MapCodecInfo({},{},{})", type_to_hl_codec(&params[0]), type_to_hl_codec(&params[1]), if nullable.is_some() { "true" } else { "false" }),
        _ => todo!()
      }
    }
    Type::Nested { ty, inner } => {
      let base = match &**ty {
        Type::Ident { ty, nullable } => {
          ty.value.0.to_owned()
        }
        _ => todo!()
      };
      match &**inner {
        Type::Ident { ty, nullable } => {
          let name = ty.value.0.to_owned();
          let info = if ENUM_TYPES.lock().unwrap().contains(&name) { "EnumCodecInfo" } else { "TypeCodecInfo" };
          format!("new {}({}.{},{})", info, base, name, if nullable.is_some() { "true" } else { "false" })
        }
        Type::Generic { ty, nullable, params } => {
          let main = ty.value.0.to_owned();
          match main.as_str() {
            "List" => format!("new CollectionCodecInfo({}.{},{},1)", type_to_hl_codec(&params[0]), base, if nullable.is_some() { "true" } else { "false" }),
            "Map" => format!("new MapCodecInfo({}.{},{},{})", type_to_hl_codec(&params[0]), base, type_to_hl_codec(&params[1]), if nullable.is_some() { "true" } else { "false" }),
            _ => todo!()
          }
        }
        _ => todo!()
      }
    }
  }
}

pub fn model_to_definition(input: &ModelDeclaration) -> Result<hl::Model, SyntaxError> {
  let constructor = input.body.iter().filter_map(|item| if let ModelItem::Constructor(value) = item { Some(value) } else { None }).next();
  let client_methods = input.body.iter().filter_map(|item| if let ModelItem::ClientMethod(value) = item { Some(value) } else { None });
  let server_methods = input.body.iter().filter_map(|item| if let ModelItem::ServerMethod(value) = item { Some(value) } else { None });

  Ok(hl::Model {
    name: input.name.value.0.to_owned(),
    id: input.id.value.0,
    constructor: constructor.map(|it| hl::ModelConstructor {
      fields: it.fields.iter().map(|it| hl::Field {
        name: it.name.value.0.to_owned(),
        kind: type_to_hl(&it.kind),
        codec: type_to_hl_codec(&it.kind),
        position: it.position.value.0 as usize,
        comments: convert_comments(&it.comments),
      }).collect_vec(),
      meta: convert_meta(&it.meta),
      comments: convert_comments(&it.comments),
    }),
    client_methods: client_methods.map(|it| hl::ClientMethod {
      name: it.name.value.0.to_owned(),
      id: it.id.value.0,
      params: it.params.iter().map(|it| hl::Param {
        name: it.name.value.0.to_owned(),
        kind: type_to_hl(&it.kind),
        codec: type_to_hl_codec(&it.kind),
      }).collect_vec(),
      comments: convert_comments(&it.comments),
    }).collect_vec(),
    server_methods: server_methods.map(|it| hl::ServerMethod {
      name: it.name.value.0.to_owned(),
      id: it.id.value.0,
      params: it.params.iter().map(|it| hl::Param {
        name: it.name.value.0.to_owned(),
        kind: type_to_hl(&it.kind),
        codec: type_to_hl_codec(&it.kind),
      }).collect_vec(),
      comments: convert_comments(&it.comments),
    }).collect_vec(),
    meta: convert_meta(&input.meta),
    comments: convert_comments(&input.comments),
  })
}

pub fn type_to_definition(input: &TypeDeclaration) -> Result<hl::Type, SyntaxError> {
  Ok(hl::Type {
    name: input.name.value.0.to_owned(),
    fields: input.fields.iter().map(|it| hl::Field {
      name: it.name.value.0.to_owned(),
      kind: type_to_hl(&it.kind),
      codec: type_to_hl_codec(&it.kind),
      position: it.position.value.0 as usize,
      comments: convert_comments(&it.comments),
    }).collect_vec(),
    meta: convert_meta(&input.meta),
    comments: convert_comments(&input.comments),
  })
}

pub fn enum_to_definition(input: &EnumDeclaration) -> Result<hl::Enum, SyntaxError> {
  Ok(hl::Enum {
    name: input.name.value.0.to_owned(),
    repr: input.repr.value.0.to_owned(),
    variants: input.variants.iter().map(|it| hl::Variant {
      name: it.name.value.0.to_owned(),
      value: it.value.value.0,
      comments: convert_comments(&it.comments),
    }).collect_vec(),
    meta: convert_meta(&input.meta),
    comments: convert_comments(&input.comments),
  })
}

pub fn convert_comments(comments: &[CommentLit]) -> Vec<String> {
  comments.iter().map(|it| it.0[2..].trim().to_owned()).collect::<_>()
}

pub fn convert_meta(meta: &[MetaDeclaration]) -> Vec<Meta> {
  meta.iter().map(|it| {
    Meta {
      key: it.key.value.0.to_owned(),
      value: it.value.value.0.to_owned(),
    }
  }).collect::<_>()
}

#[cfg(test)]
mod tests {
  use test_log::test;
  use tracing::{debug, info};
  use std::fs;

  use super::*;

  #[test]
  fn it_works() {
    let tokens = tokenizer(r#"
      /// Example model
      /// that demonstrates parser's abilities
      model SusModel = 213242343 {
        meta client_name = "SusModel";

        /// Comment for
        /// required SusEntity
        required entity SusEntity;

        /// Comment for constructor
        constructor {
          meta client_name = "SusData";

          /// Comment for constructor field
          /// CTOR FIELD 1 of type i32
          CTORFIELD1: i32 = 1;
        }

        /// This tests
        /// client ABC
        server ABC() = 1245;
        client DEF() = 6789;
        client GHI(time: i32) = 101112;
        /// AAA
        /// BBB
        ///
        client JKL(param1: i8, param2: Map<String, String<String?>?>) = 1011412;
        // client JKL(param1: i8 param2: i16) = 1011412;
      }

      /// Example type
      /// that generates codec
      type SusType {
        /// Example field with position 1
        field1: i32 = 1;
        /// Second field
        field2: Map<String, String<String?>?> = 2;
      }

      // Example enum
      enum SusEnum : i32 {
        // Default value
        A = 0;
        B = 1;
        // Special behaviour
        C = 2;
      }
    "#).unwrap();
    // debug!("{:?}", tokens);
    for token in &tokens {
      debug!("{:?}", token);
    }

    let mut iter = itertools::multipeek(&tokens);
    let ast = parse_program(&mut iter).unwrap();
    info!("{:?}", ast);

    let model = match &ast.body[0] {
      ProgramItem::Model(model) => model,
      _ => todo!()
    };
    let definition = model_to_definition(model);
    info!("{:?}", definition);
  }

  #[test]
  fn type_to_string() {
    assert_eq!(type_to_hl(&Type::Ident { ty: Positioned::identity(Identifier("String".to_owned())), nullable: None }), "String");
    assert_eq!(type_to_hl(&Type::Ident { ty: Positioned::identity(Identifier("String".to_owned())), nullable: Some(Positioned::identity(Token::Question)) }), "String?");
    assert_eq!(type_to_hl(&Type::Generic { ty: Positioned::identity(Identifier("ZeroParam".to_owned())), nullable: None, params: vec![] }), "ZeroParam<>");
    assert_eq!(type_to_hl(&Type::Generic { ty: Positioned::identity(Identifier("ZeroParam".to_owned())), nullable: Some(Positioned::identity(Token::Question)), params: vec![] }), "ZeroParam<>?");

    assert_eq!(type_to_hl(&Type::Generic {
      ty: Positioned::identity(Identifier("OneParam".to_owned())),
      nullable: None,
      params: vec![
        Type::Ident { ty: Positioned::identity(Identifier("String".to_owned())), nullable: None }
      ],
    }), "OneParam<String>");
    assert_eq!(type_to_hl(&Type::Generic {
      ty: Positioned::identity(Identifier("OneParam".to_owned())),
      nullable: Some(Positioned::identity(Token::Question)),
      params: vec![
        Type::Ident { ty: Positioned::identity(Identifier("String".to_owned())), nullable: None }
      ],
    }), "OneParam<String>?");
    assert_eq!(type_to_hl(&Type::Generic {
      ty: Positioned::identity(Identifier("OneParam".to_owned())),
      nullable: None,
      params: vec![
        Type::Ident { ty: Positioned::identity(Identifier("String".to_owned())), nullable: Some(Positioned::identity(Token::Question)) }
      ],
    }), "OneParam<String?>");
    assert_eq!(type_to_hl(&Type::Generic {
      ty: Positioned::identity(Identifier("OneParam".to_owned())),
      nullable: Some(Positioned::identity(Token::Question)),
      params: vec![
        Type::Ident { ty: Positioned::identity(Identifier("String".to_owned())), nullable: Some(Positioned::identity(Token::Question)) }
      ],
    }), "OneParam<String?>?");

    assert_eq!(type_to_hl(&Type::Generic {
      ty: Positioned::identity(Identifier("TwoParam".to_owned())),
      nullable: None,
      params: vec![
        Type::Ident { ty: Positioned::identity(Identifier("A".to_owned())), nullable: None },
        Type::Ident { ty: Positioned::identity(Identifier("B".to_owned())), nullable: None },
      ],
    }), "TwoParam<A, B>");

    assert_eq!(type_to_hl(&Type::Generic {
      ty: Positioned::identity(Identifier("TwoParam".to_owned())),
      nullable: Some(Positioned::identity(Token::Question)),
      params: vec![
        Type::Ident { ty: Positioned::identity(Identifier("A".to_owned())), nullable: Some(Positioned::identity(Token::Question)) },
        Type::Ident { ty: Positioned::identity(Identifier("B".to_owned())), nullable: None },
      ],
    }), "TwoParam<A?, B>?");
  }
}
