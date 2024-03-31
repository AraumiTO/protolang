pub mod target;

use std::collections::{HashMap, HashSet};
use std::fmt::format;
use std::{fs, i64, path::Path};
use std::borrow::ToOwned;
use std::ffi::OsStr;
use std::ops::Index;
use std::path::{Component, PathBuf};
use std::sync::Mutex;
use clap::{Parser, Subcommand};
use itertools::Itertools;

use walkdir::WalkDir;
use protolang_parser::{enum_to_definition, hl, model_to_definition, type_to_definition, ProgramItem};
use regex::Regex;
use once_cell::sync::Lazy;
use protolang_parser::hl::Meta;
use crate::target::kotlin::{generate_enum_kotlin_code, generate_model_kotlin_code, generate_type_kotlin_code};
use crate::target::protolang::{generate_protolang_code, generate_protolang_code_enum, generate_protolang_code_type};

fn generate_kotlin(root_package: Option<&str>, input_root: &Path, output_root: &Path) {
  for entry in WalkDir::new(input_root) {
    let entry = entry.unwrap();
    let path = entry.path();
    let relative_path = path.strip_prefix(input_root).unwrap();
    if is_path_hidden(relative_path) {
      continue;
    }

    if !entry.file_type().is_file() {
      continue;
    }

    println!("Parsing {:?}...", path);
    let content = fs::read_to_string(path).unwrap();

    let tokens = protolang_parser::tokenizer(&content).unwrap();
    for token in &tokens {
      println!("{:?}", token);
    }

    let mut iter = itertools::multipeek(&tokens);
    let ast = protolang_parser::parse_program(&mut iter).unwrap();
    println!("{:?}", ast);

    let mut meta = Vec::new();
    for item in &ast.body {
      match item {
        ProgramItem::Meta(item) => meta.push(Meta {
          key: item.key.value.0.to_owned(),
          value: item.key.value.0.to_owned()
        }),
        _ => continue
      };
    }

    for item in &ast.body {
      let code = match item {
        ProgramItem::Model(model) => {
          let definition = model_to_definition(model).unwrap();
          println!("{:?}", definition);

          generate_model_kotlin_code(&definition)
        },
        ProgramItem::Type(type_def) => {
          let definition = type_to_definition(type_def).unwrap();
          println!("{:?}", definition);

          generate_type_kotlin_code(&definition)
        },
        ProgramItem::Enum(enum_def) => {
          let definition = enum_to_definition(enum_def).unwrap();
          println!("{:?}", definition);

          generate_enum_kotlin_code(&definition)
        },
        _ => continue
      };

      let relative_path = relative_path.with_file_name(relative_path.file_name().unwrap().to_string_lossy().replace(".proto", ".kt"));
      let output_path = output_root.join(&relative_path);
      let package = relative_path.parent().unwrap().to_string_lossy().replace('/', ".");
      println!("generate kotlin code into {:?}", output_path);

      let mut full_package = String::new();
      if let Some(root_package) = root_package {
        full_package.push_str(root_package);
        full_package.push_str(".");
      }
      full_package.push_str(&package);

      let mut wrapped_code = String::new();
      wrapped_code.push_str(&format!("package {}\n\n", full_package));
      wrapped_code.push_str("import jp.assasans.araumi.models.*\n");
      wrapped_code.push_str("import jp.assasans.araumi.protocol.codec.wired.*\n");
      wrapped_code.push_str("\n");
      wrapped_code.push_str(&code);
      println!("{}", wrapped_code);

      fs::create_dir_all(output_path.parent().unwrap()).unwrap();
      fs::write(output_path, wrapped_code).unwrap();
    }
  }
}

#[derive(Debug)]
struct ParsedField {
  pub name: String,
  pub codec: String,
  pub kind: String
}

#[derive(Debug)]
struct ParsedVariant {
  pub name: String,
  pub value: i64
}

#[derive(Debug)]
struct ParsedMethod {
  pub name: String,
  pub id: i64,
  pub params: Vec<ParsedMethodParam>
}

#[derive(Debug)]
struct ParsedMethodParam {
  pub name: String,
  pub codec: String,
  pub kind: String
}

pub static EXISTING_TYPES: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));
pub static MODEL_TYPES: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

// file name -> path
pub static BUILTIN_FQN: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));
pub static DEFINITION_FQN: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));
pub static REGEX_CACHE: Lazy<Mutex<HashMap<String, Regex>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn generate_definition_index(input_root: &Path) {
  for entry in WalkDir::new(input_root) {
    let entry = entry.unwrap();
    let path = entry.path();
    let relative_path = path.strip_prefix(input_root).unwrap();
    if is_path_hidden(relative_path) {
      continue;
    }

    if !entry.file_type().is_file() {
      continue;
    }

    println!("Parsing {:?}...", path);
    let content = fs::read_to_string(path).unwrap();

    let tokens = protolang_parser::tokenizer(&content).unwrap();
    let mut iter = itertools::multipeek(&tokens);
    let ast = protolang_parser::parse_program(&mut iter).unwrap();

    for item in &ast.body {
      let simple_name = match item {
        ProgramItem::Model(model) => {
          let definition = model_to_definition(model).unwrap();
          definition.name
        },
        ProgramItem::Type(type_def) => {
          let definition = type_to_definition(type_def).unwrap();
          definition.name
        },
        ProgramItem::Enum(enum_def) => {
          let definition = enum_to_definition(enum_def).unwrap();
          definition.name
        },
        _ => continue
      };

      let relative_path = relative_path.to_string_lossy().replace(".proto", "").replace('/', ".");
      println!("registered {} -> {}", simple_name, relative_path);
      DEFINITION_FQN.lock().unwrap().insert(simple_name, relative_path);
    }
  }
}

fn generate_model_index(input_root: &Path) {
  for entry in WalkDir::new(input_root) {
    let entry = entry.unwrap();
    let path = entry.path();
    let relative_path = path.strip_prefix(input_root).unwrap();
    if is_path_hidden(relative_path) {
      continue;
    }

    if !entry.file_type().is_file() {
      continue;
    }

    if !path.extension().is_some_and(|it| it == "as") {
      continue;
    }

    if path.to_string_lossy().contains("excluded/") {
      continue;
    }

    let file_name = path.file_name().unwrap().to_string_lossy();
    if !file_name.contains("Model") {
      continue;
    }

    // println!("Parsing {:?}...", path);
    let content = fs::read_to_string(path).unwrap();
    if !content.contains("[ModelInfo]") {
      continue;
    }

    let model_class = Regex::new(r"class (\w+) extends (\w+) implements (\w+)").unwrap();
    let captures = model_class.captures(&content).expect("no model class capture");
    let model_name = captures.get(1).expect("no model name").as_str();
    let model_base_name = captures.get(2).expect("no model base name").as_str();

    if model_name != model_base_name.replace("ModelBase", "Model") {
      // println!("{:?} {:?}", model_name, model_base_name.replace("ModelBase", "Model"));
    }

    let class_import = Regex::new(&format!(r"import ([\w.]+\.{})", model_base_name));
    let captures = class_import.unwrap().captures(&content).expect("no class import capture");
    let model_base_import_fqdn = captures.get(1).expect("no import path").as_str();
    let model_base_import_path = model_base_import_fqdn.replace('.', "/") + ".as";
    let model_base_import_path = Path::new(&model_base_import_path);
    // println!("{:?} {:?}", model_base_import_fqdn, model_base_import_path);

    let mut sources_root = path;
    while let Some(parent) = sources_root.parent() {
      sources_root = parent;
      if parent.file_name().unwrap().to_string_lossy() == "src" {
        break;
      }
    }
    // println!("{:?}", sources_root);

    let model_base_path = sources_root.join(model_base_import_path);
    // println!("{:?}", model_base_path);
    let (model_base_path, model_base_contents) = match fs::read_to_string(&model_base_path) {
      Ok(contents) => (model_base_path, contents),
      Err(error) => {
        // eprintln!("failed to read model base file (trying entrance) {:?}: {:?}", model_base_path, error);

        // Try "entrance"
        let model_base_path = model_base_path.components().map(|it| if it.as_os_str().to_string_lossy() == "game" { Component::Normal(OsStr::new("entrance")) } else { it }).collect::<PathBuf>();
        // println!("{:?}", model_base_path);
        match fs::read_to_string(&model_base_path) {
          Ok(contents) => (model_base_path, contents),
          Err(error) => {
            eprintln!("failed to read model base file {:?}: {:?}", model_base_path, error);
            todo!();
          }
        }
      }
    };
    // println!("{}", model_base_contents);

    if !model_base_contents.contains("registerModelConstructorCodec") {
      continue;
    }

    // println!("{}", content);

    let constructor_regex = Regex::new(r"registerModelConstructorCodec.+\((.+?),\s*false\)\)\);").unwrap();
    let captures = constructor_regex.captures(&model_base_contents).expect("no model constructor capture");
    let constructor_name = captures.get(1).unwrap().as_str();

    let model_name = model_base_name.replace("ModelBase", "Model");

    MODEL_TYPES.lock().unwrap().insert(constructor_name.to_owned(), model_name.clone());

    // Do not generate definition files
    // EXISTING_TYPES.lock().unwrap().insert(constructor_name.to_owned());
    EXISTING_TYPES.lock().unwrap().insert(format!("{}.Constructor", model_name));

    println!("registered {}", format!("{}.Constructor", model_name.clone()));
  }
}

fn generate_protolang_model(input_root: &Path, output_root: &Path) {
  for entry in WalkDir::new(input_root) {
    let entry = entry.unwrap();
    let path = entry.path();
    let relative_path = path.strip_prefix(input_root).unwrap();
    if is_path_hidden(relative_path) {
      continue;
    }

    if !entry.file_type().is_file() {
      continue;
    }

    if !path.extension().is_some_and(|it| it == "as") {
      continue;
    }

    if path.to_string_lossy().contains("excluded/") {
      continue;
    }

    let file_name = path.file_name().unwrap().to_string_lossy();
    if !file_name.contains("Model") {
      continue;
    }

    // println!("Parsing {:?}...", path);
    let content = fs::read_to_string(path).unwrap();
    if !content.contains("[ModelInfo]") {
      continue;
    }

    // println!("{}", content);

    let model_class = Regex::new(r"class (\w+) extends (\w+) implements (\w+)").unwrap();
    let captures = model_class.captures(&content).expect("no model class capture");
    // let model_name = captures.get(1).expect("no model name").as_str();
    let model_base_name = captures.get(2).expect("no model base name").as_str();
    let model_interface_name = captures.get(3).expect("no model interface name").as_str();
    let model_server_name = model_base_name.replace("ModelBase", "ModelServer");
    println!("{:?} {:?} {:?} {:?}", captures.get(1).expect("no model name").as_str(), model_base_name, model_interface_name, model_server_name);

    let class_import = Regex::new(&format!(r"import ([\w.]+\.{})", model_base_name));
    let captures = class_import.unwrap().captures(&content).expect("no class import capture");
    let model_base_import_fqdn = captures.get(1).expect("no import path").as_str();
    let model_base_import_path = model_base_import_fqdn.replace('.', "/") + ".as";
    let model_base_import_path = Path::new(&model_base_import_path);
    // println!("{:?} {:?}", model_base_import_fqdn, model_base_import_path);

    let mut sources_root = path;
    while let Some(parent) = sources_root.parent() {
      sources_root = parent;
      if parent.file_name().unwrap().to_string_lossy() == "src" {
        break;
      }
    }
    // println!("{:?}", sources_root);

    let model_base_path = sources_root.join(model_base_import_path);
    // println!("{:?}", model_base_path);
    let (model_base_path, model_base_contents) = match fs::read_to_string(&model_base_path) {
      Ok(contents) => (model_base_path, contents),
      Err(error) => {
        // eprintln!("failed to read model base file (trying entrance) {:?}: {:?}", model_base_path, error);

        // Try "entrance"
        let model_base_path = model_base_path.components().map(|it| if it.as_os_str().to_string_lossy() == "game" { Component::Normal(OsStr::new("entrance")) } else { it }).collect::<PathBuf>();
        // println!("{:?}", model_base_path);
        match fs::read_to_string(&model_base_path) {
          Ok(contents) => (model_base_path, contents),
          Err(error) => {
            eprintln!("failed to read model base file {:?}: {:?}", model_base_path, error);
            todo!();
          }
        }
      }
    };
    let relative_model_base_path = model_base_path.strip_prefix(input_root).unwrap();
    // println!("{}", model_base_contents);

    let model_id_regex = Regex::new(r"this.modelId = Long.getLong\((?<high>-?(0x)?[0-9a-f]+),(?<low>-?(0x)?[0-9a-f]+)\)").unwrap();
    let model_constructor_regex = Regex::new(r"registerModelConstructorCodec\(this.modelId,this._protocol.getCodec\((?<codec>.+)\)\)").unwrap();
    let model_method_regex = Regex::new(r"this._(?<method>[A-Za-z0-9_]+)Id = Long.getLong\((?<high>-?(0x)?[0-9a-f]+),(?<low>-?(0x)?[0-9a-f]+)\)").unwrap();
    let model_method_param_regex = Regex::new(r"this._(?<method>[A-Za-z0-9_]+)_(?<param>[A-Za-z0-9_]+)Codec = this._protocol.getCodec\((?<codec>.+)\)").unwrap();

    let captures = model_id_regex.captures(&model_base_contents).expect("no model id");
    let model_id = convert_to_id(parse_id_from_dec_or_hex(captures.name("high").unwrap().as_str()), parse_id_from_dec_or_hex(captures.name("low").unwrap().as_str()));
    println!("model id: {}", model_id);

    let model_constructor = if let Some(captures) = model_constructor_regex.captures(&model_base_contents) {
      let model_constructor = captures.name("codec").unwrap().as_str();
      println!("model constructor: {}", model_constructor);
      Some(model_constructor.to_owned())
    } else {
      None
    };

    let mut client_methods = Vec::new();
    let captures = model_method_regex.captures_iter(&model_base_contents);
    for capture in captures {
      let method_name = capture.name("method").unwrap().as_str();
      let model_id = convert_to_id(parse_id_from_dec_or_hex(capture.name("high").unwrap().as_str()), parse_id_from_dec_or_hex(capture.name("low").unwrap().as_str()));
      // println!("client method: {} = {}", method_name, model_id);

      client_methods.push(ParsedMethod {
        name: method_name.to_owned(),
        id: model_id,
        params: Vec::new()
      });
    }

    let captures = model_method_param_regex.captures_iter(&model_base_contents);
    for capture in captures {
      let method_name = capture.name("method").unwrap().as_str();
      let param = capture.name("param").unwrap().as_str();
      let codec = capture.name("codec").unwrap().as_str();
      // println!("client method: {} = {} by {}", method_name, param, codec);
      //
      // println!("searching for {method_name}");
      // println!("{:?}", client_methods);
      let method = client_methods.iter_mut().find(|it| it.name == method_name).unwrap();
      method.params.push(ParsedMethodParam {
        name: param.to_owned(),
        codec: codec.to_owned(),
        kind: codec_to_type(codec, false)
      });
    }

    let model_server_contents = fs::read_to_string(model_base_path.parent().unwrap().to_path_buf().join(model_server_name + ".as")).unwrap();
    // println!("{}", model_server_contents);

    let model_method_param_regex = Regex::new(r"this._(?<method>[A-Za-z0-9_]+)_(?<param>[A-Za-z0-9_]+)Codec = this.protocol.getCodec\((?<codec>.+)\)").unwrap();

    let mut server_methods = Vec::new();
    let captures = model_method_regex.captures_iter(&model_server_contents);
    for capture in captures {
      let method_name = capture.name("method").unwrap().as_str();
      let model_id = convert_to_id(parse_id_from_dec_or_hex(capture.name("high").unwrap().as_str()), parse_id_from_dec_or_hex(capture.name("low").unwrap().as_str()));
      // println!("server method: {} = {}", method_name, model_id);

      server_methods.push(ParsedMethod {
        name: method_name.to_owned(),
        id: model_id,
        params: Vec::new()
      });
    }

    let captures = model_method_param_regex.captures_iter(&model_server_contents);
    for capture in captures {
      let method_name = capture.name("method").unwrap().as_str();
      let param = capture.name("param").unwrap().as_str();
      let codec = capture.name("codec").unwrap().as_str();
      // println!("server method: {} = {} by {}", method_name, param, codec);

      let method = server_methods.iter_mut().find(|it| it.name == method_name).unwrap();
      method.params.push(ParsedMethodParam {
        name: param.to_owned(),
        codec: codec.to_owned(),
        kind: codec_to_type(codec, false)
      });
    }

    println!("CI {:?}", client_methods);
    println!("SI {:?}", server_methods);

    let model_name = model_base_name.replace("ModelBase", "Model");
    let model = hl::Model {
      name: model_name.clone(),
      id: model_id,
      constructor: model_constructor.map(|it| {
        let kind = codec_to_type(&it, true);
        let (_, type_def) = generate_protolang_type(&kind, input_root, output_root).unwrap();
        hl::ModelConstructor {
          fields: type_def.fields,
          comments: type_def.comments
        }
      }),
      client_methods: client_methods.iter().map(|it| hl::ClientMethod {
        name: it.name.to_owned(),
        id: it.id,
        params: it.params.iter().map(|it| hl::Param {
          name: it.name.to_owned(),
          kind: it.kind.to_owned()
        }).collect_vec(),
        comments: vec![],
      }).collect_vec(),
      server_methods: server_methods.iter().map(|it| hl::ServerMethod {
        name: it.name.to_owned(),
        id: it.id,
        params: it.params.iter().map(|it| hl::Param {
          name: it.name.to_owned(),
          kind: it.kind.to_owned()
        }).collect_vec(),
        comments: vec![],
      }).collect_vec(),
      comments: vec![
        format!("TODO: This is an automatically generated model definition for \"{}\"", model_name)
      ],
    };
    let project = relative_model_base_path.components().nth(0).map(|it| it.as_os_str().to_string_lossy()).unwrap();
    let definition = generate_protolang_code(&model, &[
      Meta { key: "client_package".to_owned(), value: format!("{}:{}", project, convert_path_to_definition(&relative_model_base_path).parent().unwrap().to_string_lossy().replace('/', ".")) }
    ]);
    println!("{}", definition);

    if let Some(constructor) = &model.constructor {
      for field in &constructor.fields {
        let types = get_types_from_generic(&field.kind);
        for name in &types {
          if !EXISTING_TYPES.lock().unwrap().contains(name) {
            println!("generating constructor type for {}", name);

            let project = relative_model_base_path.components().nth(0).map(|it| it.as_os_str().to_string_lossy()).unwrap().to_string();
            let (relative_model_base_path, definition) = generate_type_code_for(name, &project, input_root, output_root);
            println!("{}", definition);

            let relative_model_base_path = relative_model_base_path.with_file_name(relative_model_base_path.file_name().unwrap().to_string_lossy().replacen("Codec", "", 1).replace(".as", ".proto"));
            let output_path = output_root.join(convert_path_to_definition(&relative_model_base_path));
            println!("generate type into {:?}", output_path);
            fs::create_dir_all(output_path.parent().unwrap()).unwrap();
            fs::write(output_path, definition).unwrap();
          }
        }
      }
    }

    for method in &model.client_methods {
      for param in &method.params {
        let types = get_types_from_generic(&param.kind);
        // println!("{:?}", types);
        for name in &types {
          if !EXISTING_TYPES.lock().unwrap().contains(name) {
            println!("generating type for {}", name);

            // TODO
            let project = relative_model_base_path.components().nth(0).map(|it| it.as_os_str().to_string_lossy()).unwrap().to_string();
            let (relative_model_base_path, definition) = generate_type_code_for(name, &project, input_root, output_root);
            println!("{}", definition);

            let relative_model_base_path = relative_model_base_path.with_file_name(relative_model_base_path.file_name().unwrap().to_string_lossy().replacen("Codec", "", 1).replace(".as", ".proto"));
            let output_path = output_root.join(convert_path_to_definition(&relative_model_base_path));
            println!("generate type into {:?}", output_path);
            fs::create_dir_all(output_path.parent().unwrap()).unwrap();
            fs::write(output_path, definition).unwrap();
          }
        }
      }
    }

    for method in &model.server_methods {
      for param in &method.params {
        let types = get_types_from_generic(&param.kind);
        // println!("{:?}", types);
        for name in &types {
          if !EXISTING_TYPES.lock().unwrap().contains(name) {
            println!("generating type for {}", name);

            // TODO
            let project = relative_model_base_path.components().nth(0).map(|it| it.as_os_str().to_string_lossy()).unwrap().to_string();
            let (relative_model_base_path, definition) = generate_type_code_for(name, &project, input_root, output_root);
            println!("{}", definition);

            let relative_model_base_path = relative_model_base_path.with_file_name(relative_model_base_path.file_name().unwrap().to_string_lossy().replacen("Codec", "", 1).replace(".as", ".proto"));
            let output_path = output_root.join(convert_path_to_definition(&relative_model_base_path));
            println!("generate type into {:?}", output_path);
            fs::create_dir_all(output_path.parent().unwrap()).unwrap();
            fs::write(output_path, definition).unwrap();
          }
        }
      }
    }

    let relative_model_base_path = relative_model_base_path.with_file_name(relative_model_base_path.file_name().unwrap().to_string_lossy().replace("ModelBase.as", "Model.proto"));
    let output_path = output_root.join(convert_path_to_definition(&relative_model_base_path));
    fs::create_dir_all(output_path.parent().unwrap()).unwrap();
    fs::write(output_path, definition).unwrap();

    // break;
  }
}

fn generate_type_code_for(name: &str, project: &str, input_root: &Path, output_root: &Path) -> (PathBuf, String) {
  match generate_protolang_type(name, input_root, output_root) {
    Some((relative_path, type_def)) => {
      let definition = generate_protolang_code_type(&type_def, &[
        Meta { key: "client_package".to_owned(), value: format!("{}:{}", project, convert_path_to_definition(&relative_path).parent().unwrap().to_string_lossy().replace('/', ".")) }
      ]);
      (relative_path, definition)
    }

    None => match generate_protolang_enum(name, input_root) {
      Some((relative_path, enum_def)) => {
        let project = relative_path.components().nth(0).map(|it| it.as_os_str().to_string_lossy()).unwrap();
        let definition = generate_protolang_code_enum(&enum_def, &[
          Meta { key: "client_package".to_owned(), value: format!("{}:{}", project, convert_path_to_definition(&relative_path).parent().unwrap().to_string_lossy().replace('/', ".")) }
        ]);
        println!("{}", definition);

        (relative_path, definition)
      }

      None => panic!("cannot generate type/enum definition for {}", name)
    }
  }
}

fn generate_protolang_type(name: &str, input_root: &Path, output_root: &Path) -> Option<(PathBuf, hl::Type)> {
  for entry in WalkDir::new(input_root) {
    let entry = entry.unwrap();
    let path = entry.path();
    let relative_path = path.strip_prefix(input_root).unwrap();
    if is_path_hidden(relative_path) {
      continue;
    }

    if !entry.file_type().is_file() {
      continue;
    }

    if !path.extension().is_some_and(|it| it == "as") {
      continue;
    }

    if path.to_string_lossy().contains("excluded/") {
      continue;
    }

    let file_name = path.file_name().unwrap().to_string_lossy();
    if file_name != format!("Codec{}.as", name) {
      continue;
    }

    println!("Parsing {:?}...", relative_path);
    let content = fs::read_to_string(path).unwrap();
    // println!("{}", content);

    if content.contains(" switch(") {
      // Enum
      return None;
    }

    let field_regex = Regex::new(r"this.codec_(?<field>[A-Za-z0-9_]+) = param1.getCodec\((?<codec>.+)\)").unwrap();

    let mut fields = Vec::new();
    let captures = field_regex.captures_iter(&content);
    for capture in captures {
      let field_name = capture.name("field").unwrap().as_str();
      let codec = capture.name("codec").unwrap().as_str();
      // println!("field: {} by {}", field_name, codec =);

      fields.push(ParsedField {
        name: field_name.to_owned(),
        codec: codec.to_owned(),
        kind: codec_to_type(codec, false)
      });
    }

    println!("{:?}", fields);

    let type_def = hl::Type {
      name: name.to_owned(),
      fields: fields.iter().enumerate().map(|(index, it)| hl::Field {
        name: it.name.to_owned(),
        kind: it.kind.to_owned(),
        position: index + 1,
        comments: vec![],
      }).collect_vec(),
      comments: vec![
        format!("TODO: This is an automatically generated type definition for \"{}\"", name)
      ],
    };

    EXISTING_TYPES.lock().unwrap().insert(name.to_owned());

    for field in &type_def.fields {
      let types = get_types_from_generic(&field.kind);
      for name in &types {
        if !EXISTING_TYPES.lock().unwrap().contains(name) {
          println!("generating recursive type for {}", name);

          let project = relative_path.components().nth(0).map(|it| it.as_os_str().to_string_lossy()).unwrap().to_string();
          let (relative_path, definition) = generate_type_code_for(name, &project, input_root, output_root);
          println!("{}", definition);

          let relative_path = relative_path.with_file_name(relative_path.file_name().unwrap().to_string_lossy().replacen("Codec", "", 1).replace(".as", ".proto"));
          let output_path = output_root.join(convert_path_to_definition(&relative_path));
          println!("generate type into {:?}", output_path);
          fs::create_dir_all(output_path.parent().unwrap()).unwrap();
          fs::write(output_path, definition).unwrap();
        }
      }
    }

    return Some((relative_path.to_path_buf(), type_def));
  }
  panic!("cannot find type {}", name);
}

fn generate_protolang_enum(name: &str, input_root: &Path) -> Option<(PathBuf, hl::Enum)> {
  for entry in WalkDir::new(input_root) {
    let entry = entry.unwrap();
    let path = entry.path();
    let relative_path = path.strip_prefix(input_root).unwrap();
    if is_path_hidden(relative_path) {
      continue;
    }

    if !entry.file_type().is_file() {
      continue;
    }

    if !path.extension().is_some_and(|it| it == "as") {
      continue;
    }

    if path.to_string_lossy().contains("excluded/") {
      continue;
    }

    let file_name = path.file_name().unwrap().to_string_lossy();
    if file_name != format!("Codec{}.as", name) {
      continue;
    }

    println!("Parsing {:?}...", relative_path);
    let content = fs::read_to_string(path).unwrap();
    // println!("{}", content);

    if !content.contains(" switch(") {
      return None;
    }

    let variant_regex = Regex::new(r"case (?<value>\d+):\s*.+\.(?<variant>\w+);").unwrap();

    let mut variants = Vec::new();
    let captures = variant_regex.captures_iter(&content);
    for capture in captures {
      let variant = capture.name("variant").unwrap().as_str();
      let value = capture.name("value").unwrap().as_str().parse::<i64>().unwrap();
      // println!("variant: {} = {}", variant, value);

      variants.push(ParsedVariant {
        name: variant.to_owned(),
        value
      });
    }

    println!("{:?}", variants);

    let enum_def = hl::Enum {
      name: name.to_owned(),
      repr: "i32".to_owned(),
      variants: variants.iter().map(|it| hl::Variant {
        name: it.name.to_owned(),
        value: it.value,
        comments: vec![],
      }).collect_vec(),
      comments: vec![
        format!("TODO: This is an automatically generated enum definition for \"{}\"", name)
      ],
    };

    EXISTING_TYPES.lock().unwrap().insert(name.to_owned());

    return Some((relative_path.to_path_buf(), enum_def));
  }
  panic!("cannot find enum {}", name);
}

pub fn wrap_to_u64(x: i64) -> u64 {
  (x as u64).wrapping_add(u64::MAX / 2 + 1)
}

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
  #[command(subcommand)]
  command: Actions,
}

#[derive(Subcommand, Debug)]
enum Actions {
  GenerateProtolang {
    input: PathBuf,

    #[arg(short, long)]
    output: PathBuf,
  },
  GenerateKotlin {
    input: PathBuf,

    #[arg(short, long)]
    output: PathBuf,

    #[arg(long)]
    package: Option<String>,
  }
}

fn main() {
  let args = Args::parse();

  {
    let mut types = EXISTING_TYPES.lock().unwrap();
    let mut paths = BUILTIN_FQN.lock().unwrap();

    types.insert("bool".to_owned());
    types.insert("i8".to_owned());
    types.insert("i16".to_owned());
    types.insert("i32".to_owned());
    types.insert("i64".to_owned());
    types.insert("f32".to_owned());
    types.insert("f64".to_owned());
    types.insert("String".to_owned());

    for ty in types.iter() {
      // Primitives are globally available
      paths.insert(ty.to_owned(), ty.to_owned());
    }

    types.insert("Instant".to_owned());
    paths.insert("Instant".to_owned(), "kotlinx.datetime.Instant".to_owned());
    types.insert("IGameObject".to_owned());
    paths.insert("IGameObject".to_owned(), "jp.assasans.araumi.architecture.objects.IGameObject".to_owned());

    types.insert("Object".to_owned()); // synthetic

    types.insert("Resource".to_owned());
    paths.insert("Resource".to_owned(), "jp.assasans.araumi.resources.Resource".to_owned());
    types.insert("SoundResource".to_owned());
    paths.insert("SoundResource".to_owned(), "jp.assasans.araumi.resources.SoundResource".to_owned());
    types.insert("MapResource".to_owned());
    paths.insert("MapResource".to_owned(), "jp.assasans.araumi.resources.MapResource".to_owned());
    types.insert("ProplibResource".to_owned());
    paths.insert("ProplibResource".to_owned(), "jp.assasans.araumi.resources.ProplibResource".to_owned());
    types.insert("TextureResource".to_owned());
    paths.insert("TextureResource".to_owned(), "jp.assasans.araumi.resources.TextureResource".to_owned());
    types.insert("ImageResource".to_owned());
    paths.insert("ImageResource".to_owned(), "jp.assasans.araumi.resources.ImageResource".to_owned());
    types.insert("MultiframeTextureResource".to_owned());
    paths.insert("MultiframeTextureResource".to_owned(), "jp.assasans.araumi.resources.MultiframeTextureResource".to_owned());
    types.insert("LocalizedImageResource".to_owned());
    paths.insert("LocalizedImageResource".to_owned(), "jp.assasans.araumi.resources.LocalizedImageResource".to_owned());
    types.insert("Object3DResource".to_owned());
    paths.insert("Object3DResource".to_owned(), "jp.assasans.araumi.resources.Object3DResource".to_owned());
  }

  match &args.command {
    Actions::GenerateProtolang { input, output } => {
      generate_model_index(input);
      let types = MODEL_TYPES.lock().unwrap();
      for (constructor_name, model_name) in types.iter() {
        println!("{} -> {}", constructor_name, model_name);
      }
      generate_protolang_model(input, output);
    }

    Actions::GenerateKotlin { input, output, package } => {
      generate_definition_index(input);
      generate_kotlin(package.as_deref(), input, output);
    }
  }

  // let (_, type_def) = generate_protolang_type("ExternalAuthParameters");
  // let definition = generate_protolang_code_type(&type_def, &[]);
  // let (_, enum_def) = generate_protolang_enum("TargetingMode");
  // let definition = generate_protolang_code_enum(&enum_def, &[]);
  // println!("{}", definition);
}

fn is_path_hidden<P: AsRef<Path>>(path: P) -> bool {
  path.as_ref().components().any(|component| {
    if let Some(name) = component.as_os_str().to_str() {
      name.starts_with('.')
    } else {
      false
    }
  })
}

fn parse_id_from_dec_or_hex(value: &str) -> i32 {
  if value.starts_with("0x") {
    i32::from_str_radix(value.strip_prefix("0x").unwrap(), 16).unwrap()
  } else {
    value.parse::<i32>().unwrap()
  }
}

fn codec_to_type(codec: &str, is_constructor: bool) -> String {
  let type_regex = Regex::new(r"new (?:Type|Enum)CodecInfo\((.+?),\s*(false|true)\)").unwrap();
  let collection_regex = Regex::new(r"new CollectionCodecInfo\((.+?),\s*(false|true)(?:,\s*\d+)?\)").unwrap();
  let map_regex = Regex::new(r"new MapCodecInfo\((.+?),\s*(.+?),\s*(false|true)\)").unwrap();

  let codec = type_regex.replace_all(&codec, |captures: &regex::Captures| {
    let inner = captures.get(1).unwrap().as_str();
    let optional = captures.get(2).unwrap().as_str();
    format!("{}{}", inner, if optional == "true" { "?" } else { "" })
  });
  let codec = collection_regex.replace_all(&codec, |captures: &regex::Captures| {
    let inner = captures.get(1).unwrap().as_str();
    let optional = captures.get(2).unwrap().as_str();
    format!("List<{}>{}", inner, if optional == "true" { "?" } else { "" })
  });
  let codec = map_regex.replace_all(&codec, |captures: &regex::Captures| {
    let key = captures.get(1).unwrap().as_str();
    let value = captures.get(2).unwrap().as_str();
    let optional = captures.get(3).unwrap().as_str();
    format!("Map<{}, {}>{}", key, value, if optional == "true" { "?" } else { "" })
  });
  let codec = Regex::new(r"\bBoolean\b").unwrap().replace_all(&codec, "bool");
  let codec = Regex::new(r"\bByte\b").unwrap().replace_all(&codec, "i8");
  let codec = Regex::new(r"\bShort\b").unwrap().replace_all(&codec, "i16");
  let codec = Regex::new(r"\b[Ii]nt\b").unwrap().replace_all(&codec, "i32");
  let codec = Regex::new(r"\bLong\b").unwrap().replace_all(&codec, "i64");
  let codec = Regex::new(r"\bFloat\b").unwrap().replace_all(&codec, "f32");
  let codec = Regex::new(r"\b(Number|Double)\b").unwrap().replace_all(&codec, "f64");
  let codec = Regex::new(r"\bTanks3DSResource\b").unwrap().replace_all(&codec, "Object3DResource");
  let codec = Regex::new(r"\bDate\b").unwrap().replace_all(&codec, "Instant");

  if !is_constructor {
    // Convert CC to Model.Constructor references
    // TODO: This does not support wrapped types, only TypeCodecInfo
    let model_name = MODEL_TYPES.lock().unwrap().get(&codec.to_string()).cloned();
    if let Some(model_name) = &model_name {
      return format!("{}.Constructor", model_name);
    }
  }

  codec.to_string()
}

fn convert_to_id(high: i32, low: i32) -> i64 {
  ((u32::from_ne_bytes(i32::to_ne_bytes(high)) as i64) << 32) | (u32::from_ne_bytes(i32::to_ne_bytes(low)) as i64)
}

fn convert_path_to_definition(path: &Path) -> PathBuf {
  let relative_to_source_root = path.components().skip(3).collect::<PathBuf>();
  if relative_to_source_root.components().nth(0).unwrap().as_os_str() == "_codec" {
    relative_to_source_root.components().skip(1).collect::<PathBuf>()
  } else {
    relative_to_source_root
  }
}

fn get_types_from_generic(value: &str) -> Vec<String> {
  let regex = Regex::new(r"<([\w\s,?]+)>").unwrap();
  let captures = match regex.captures(value) {
    Some(value) => value,
    None => return vec![value.trim().trim_end_matches('?').to_owned()]
  };

  let inner = captures.get(1).unwrap().as_str();
  inner.split(',').map(|it| it.trim().trim_end_matches('?').to_owned()).collect_vec()
}
