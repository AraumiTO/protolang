use std::fmt::format;
use itertools::Itertools;

use protolang_parser::hl::{Enum, Meta, Model, Type};

pub fn generate_protolang_code(model: &Model) -> String {
  let mut builder = String::new();

  for comment in &model.comments {
    builder.push_str(&format!("/// {}\n", comment));
  }
  builder.push_str(&format!("model {} = {} {{\n", model.name, model.id));

  let mut segments = Vec::new();

  if !model.meta.is_empty() {
    let mut builder = String::new();
    for item in &model.meta {
      builder.push_str(&format!("  meta {} = \"{}\";\n", item.key, item.value));
    }
    segments.push(builder);
  }

  if let Some(constructor) = &model.constructor {
    let mut builder = String::new();
    for comment in &constructor.comments {
      builder.push_str(&format!("  /// {}\n", comment));
    }
    builder.push_str("  constructor {\n");

    for item in &constructor.meta {
      builder.push_str(&format!("    meta {} = \"{}\";\n", item.key, item.value));
    }
    if !constructor.fields.is_empty() {
      builder.push_str("\n");
    }

    for field in &constructor.fields {
      for comment in &field.comments {
        builder.push_str(&format!("    /// {}\n", comment));
      }
      builder.push_str(&format!("    {}: {} = {};\n", field.name, field.kind, field.position));
    }
    builder.push_str("  }\n");

    segments.push(builder);
  }

  if !model.client_methods.is_empty() {
    let mut builder = String::new();
    for method in &model.client_methods {
      for comment in &method.comments {
        builder.push_str(&format!("  /// {}\n", comment));
      }

      let params = method.params.iter().map(|it| format!("{}: {}", it.name, it.kind)).join(", ");
      builder.push_str(&format!("  client {}({}) = {};\n", method.name, params, method.id));
    }

    segments.push(builder);
  }

  if !model.server_methods.is_empty() {
    let mut builder = String::new();
    for method in &model.server_methods {
      for comment in &method.comments {
        builder.push_str(&format!("  /// {}\n", comment));
      }

      let params = method.params.iter().map(|it| format!("{}: {}", it.name, it.kind)).join(", ");
      builder.push_str(&format!("  server {}({}) = {};\n", method.name, params, method.id));
    }

    segments.push(builder);
  }

  builder.push_str(&segments.join("\n"));

  builder.push_str("}\n");
  builder
}

pub fn generate_protolang_code_type(type_def: &Type) -> String {
  let mut builder = String::new();
  for comment in &type_def.comments {
    builder.push_str(&format!("/// {}\n", comment));
  }
  builder.push_str(&format!("type {} {{\n", type_def.name));

  for item in &type_def.meta {
    builder.push_str(&format!("  meta {} = \"{}\";\n", item.key, item.value));
  }
  if !type_def.fields.is_empty() {
    builder.push_str("\n");
  }

  for field in &type_def.fields {
    for comment in &field.comments {
      builder.push_str(&format!("  /// {}\n", comment));
    }

    builder.push_str(&format!("  {}: {} = {};\n", field.name, field.kind, field.position));
  }

  builder.push_str("}\n");
  builder
}

pub fn generate_protolang_code_enum(enum_def: &Enum) -> String {
  let mut builder = String::new();

  for comment in &enum_def.comments {
    builder.push_str(&format!("/// {}\n", comment));
  }
  builder.push_str(&format!("enum {} : {} {{\n", enum_def.name, enum_def.repr));

  for item in &enum_def.meta {
    builder.push_str(&format!("  meta {} = \"{}\";\n", item.key, item.value));
  }
  if !enum_def.variants.is_empty() {
    builder.push_str("\n");
  }

  for variant in &enum_def.variants {
    for comment in &variant.comments {
      builder.push_str(&format!("  /// {}\n", comment));
    }

    builder.push_str(&format!("  {} = {};\n", variant.name, variant.value));
  }

  builder.push_str("}\n");
  builder
}
