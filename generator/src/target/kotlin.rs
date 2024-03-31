use std::fmt::format;
use itertools::Itertools;
use lazy_static::lazy_static;
use protolang_parser::hl::{Enum, Model, Type};
use regex::Regex;

use crate::{BUILTIN_FQN, DEFINITION_FQN, REGEX_CACHE};

/*
@ModelInfo(6071565290933648049)
abstract class ChatModelBase : Model(),
                               IConstructableModel<ChatModelBase.Constructor>,
                               IModelCI<ChatModelBase.Client> by ModelCI(Client::class),
                               IModelSI<ChatModelBase.ServerBase> by ModelSI(ServerBase::class) {
  @Wired
  data class Constructor(
    @Wire(0) val admin: Boolean,
    @Wire(1) val antifloodEnabled: Boolean
  ) : ModelConstructor

  interface Client : ClientInterface {
    @ModelMethod(3430453981713932879) suspend fun cleanUsersMessages(username: String)
    @ModelMethod(4202027557179282961) suspend fun showMessages(messages: List<ChatMessage>)
  }

  sealed class ServerBase : ServerInterface {
    override lateinit var client: ISpaceClient

    @ModelMethod(6683616035809206555) abstract suspend fun changeChannel(channel: String)
    @ModelMethod(3122753540375943279) abstract suspend fun sendMessage(params: SendMessageParams)
  }
}
*/

pub fn generate_model_kotlin_code(model: &Model) -> String {
  let mut builder = String::new();

  if !model.comments.is_empty() {
    builder.push_str("/**\n");
    for comment in &model.comments {
      builder.push_str(&format!(" * {}\n", comment));
    }
    builder.push_str(" */\n");
  }

  builder.push_str(&format!("@ModelInfo({})\n", model.id));
  builder.push_str(&format!("abstract class {}Base : Model(),\n", model.name));
  if model.constructor.is_some() {
    builder.push_str("  IConstructableModel,\n");
  }
  if !model.client_methods.is_empty() {
    builder.push_str(&format!("  IModelCI<{}Base.Client> by ModelCI(Client::class),\n", model.name));
  }
  if !model.server_methods.is_empty() {
    builder.push_str(&format!("  IModelSI<{}Base.Server> by ModelSI(ServerBase::class),\n", model.name));
  }
  builder.push_str("{\n");

  let mut segments = Vec::new();
  if let Some(constructor) = &model.constructor {
    let mut builder = String::new();

    if !constructor.comments.is_empty() {
      builder.push_str("  /**\n");
      for comment in &constructor.comments {
        builder.push_str(&format!("   * {}\n", comment));
      }
      builder.push_str("   */\n");
    }

    builder.push_str("  @Wired\n");
    builder.push_str("  data class Constructor(\n");
    for field in &constructor.fields {
      if !field.comments.is_empty() {
        builder.push_str("    /**\n");
        for comment in &field.comments {
          builder.push_str(&format!("     * {}\n", comment));
        }
        builder.push_str("     */\n");
      }
      builder.push_str(&format!("    @Wire({}) val {}: {},\n", field.position - 1, field.name, convert_type(&field.kind)));
    }
    builder.push_str("  )\n");
    segments.push(builder);
  }

  if !model.client_methods.is_empty() {
    let mut builder = String::new();

    builder.push_str("  interface Client : ClientInterface {\n");
    for method in &model.client_methods {
      let params = method.params.iter().map(|it| format!("{}: {}", it.name, convert_type(&it.kind))).join(", ");
      builder.push_str(&format!("    @ModelMethod({}) suspend fun {}({})\n", method.id, method.name, params))
    }
    builder.push_str("  }\n");

    segments.push(builder);
  }

  if !model.server_methods.is_empty() {
    let mut builder = String::new();

    builder.push_str("  sealed class ServerBase : ServerInterface {\n");
    builder.push_str("    override lateinit var client: ISpaceClient\n");
    builder.push_str("\n");
    for method in &model.client_methods {
      let params = method.params.iter().map(|it| format!("{}: {}", it.name, convert_type(&it.kind))).join(", ");
      if !method.comments.is_empty() {
        builder.push_str("    /**\n");
        for comment in &method.comments {
          builder.push_str(&format!("     * {}\n", comment));
        }
        builder.push_str("     */\n");
      }

      builder.push_str(&format!("    @ModelMethod({}) abstract suspend fun {}({})\n", method.id, method.name, params))
    }
    builder.push_str("  }\n");

    segments.push(builder);
  }

  builder.push_str(&segments.join("\n"));
  builder.push_str("}\n");

  builder
}

/*
@Wired
data class SomeConstructor(
  @Wire(0) val admin: Boolean,
  @Wire(1) val antifloodEnabled: Boolean
)
*/
pub fn generate_type_kotlin_code(type_def: &Type) -> String {
  let mut builder = String::new();

  if !type_def.comments.is_empty() {
    builder.push_str("/**\n");
    for comment in &type_def.comments {
      builder.push_str(&format!(" * {}\n", comment));
    }
    builder.push_str(" */\n");
  }

  builder.push_str("@Wired\n");
  builder.push_str(&format!("data class {}(\n", type_def.name));
  for field in &type_def.fields {
    if !field.comments.is_empty() {
      builder.push_str("  /**\n");
      for comment in &field.comments {
        builder.push_str(&format!("   * {}\n", comment));
      }
      builder.push_str("   */\n");
    }
    builder.push_str(&format!("  @Wire({}) val {}: {},\n", field.position - 1, field.name, convert_type(&field.kind)));
  }
  builder.push_str(")\n");

  builder
}

/*
@WiredEnum(Int::class)
enum class BattleTeam(override val value: Int) : IWiredEnum<Int> {
  RED(0),
  BLUE(1),
  NONE(2);
}
*/
pub fn generate_enum_kotlin_code(enum_def: &Enum) -> String {
  let mut builder = String::new();

  if !enum_def.comments.is_empty() {
    builder.push_str("/**\n");
    for comment in &enum_def.comments {
      builder.push_str(&format!(" * {}\n", comment));
    }
    builder.push_str(" */\n");
  }

  let repr_converted = convert_type(&enum_def.repr);
  builder.push_str(&format!("@WiredEnum({}::class)\n", repr_converted));
  builder.push_str(&format!("enum class {}(override val value: {}) : IWiredEnum<{}> {{\n", enum_def.name, repr_converted, repr_converted));
  for variant in &enum_def.variants {
    if !variant.comments.is_empty() {
      builder.push_str("  /**\n");
      for comment in &variant.comments {
        builder.push_str(&format!("   * {}\n", comment));
      }
      builder.push_str("   */\n");
    }
    builder.push_str(&format!("  {}({}),\n", variant.name, variant.value));
  }
  builder.push_str("}\n");

  builder
}

lazy_static! {
  static ref REGEX_1: Regex = Regex::new(r"\bbool\b").unwrap();
  static ref REGEX_2: Regex = Regex::new(r"\bi8\b").unwrap();
  static ref REGEX_3: Regex = Regex::new(r"\bi16\b").unwrap();
  static ref REGEX_4: Regex = Regex::new(r"\bi32\b").unwrap();
  static ref REGEX_5: Regex = Regex::new(r"\bi64\b").unwrap();
  static ref REGEX_6: Regex = Regex::new(r"\bf32\b").unwrap();
  static ref REGEX_7: Regex = Regex::new(r"\bf64\b").unwrap();
}

pub fn convert_type(value: &str) -> String {
  let value = REGEX_1.replace_all(&value, "Boolean");
  let value = REGEX_2.replace_all(&value, "Byte");
  let value = REGEX_3.replace_all(&value, "Short");
  let value = REGEX_4.replace_all(&value, "Int");
  let value = REGEX_5.replace_all(&value, "Long");
  let value = REGEX_6.replace_all(&value, "Float");
  let value = REGEX_7.replace_all(&value, "Double");

  let mut cache = REGEX_CACHE.lock().unwrap();

  // TODO: What the fuck
  let mut value = value.to_string();
  let paths = BUILTIN_FQN.lock().unwrap();
  for (simple_name, full_name) in paths.iter() {
    let regex = cache.entry(simple_name.to_owned()).or_insert_with(|| Regex::new(&format!(r"\b{}\b", simple_name)).unwrap());
    value = regex.replace_all(&value, full_name).to_string();
  }

  let definitions = DEFINITION_FQN.lock().unwrap();
  for (simple_name, full_name) in definitions.iter() {
    let regex = cache.entry(simple_name.to_owned()).or_insert_with(|| Regex::new(&format!(r"\b{}\b", simple_name)).unwrap());
    value = regex.replace_all(&value, full_name).to_string();
  }

  value.to_string()
}
