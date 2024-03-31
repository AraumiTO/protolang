#[derive(Debug)]
pub struct Model {
  pub name: String,
  pub id: i64,
  pub constructor: Option<ModelConstructor>,
  // pub entities: ModelItem,
  pub client_methods: Vec<ClientMethod>,
  pub server_methods: Vec<ServerMethod>,
  pub comments: Vec<String>
}

#[derive(Debug)]
pub struct ModelConstructor {
  pub fields: Vec<Field>,
  pub comments: Vec<String>
}

#[derive(Debug)]
pub struct Field {
  pub name: String,
  pub kind: String,
  pub position: usize,
  pub comments: Vec<String>
}

#[derive(Debug)]
pub struct ClientMethod {
  pub name: String,
  pub id: i64,
  pub params: Vec<Param>,
  pub comments: Vec<String>
}

#[derive(Debug)]
pub struct ServerMethod {
  pub name: String,
  pub id: i64,
  pub params: Vec<Param>,
  pub comments: Vec<String>
}

#[derive(Debug)]
pub struct Param {
  pub name: String,
  pub kind: String
}

#[derive(Debug)]
pub struct Type {
  pub name: String,
  pub fields: Vec<Field>,
  pub comments: Vec<String>
}

#[derive(Debug)]
pub struct Enum {
  pub name: String,
  pub repr: String,
  pub variants: Vec<Variant>,
  pub comments: Vec<String>
}

#[derive(Debug)]
pub struct Variant {
  pub name: String,
  pub value: i64,
  pub comments: Vec<String>
}

#[derive(Debug)]
pub struct Meta {
  pub key: String,
  pub value: String
}
