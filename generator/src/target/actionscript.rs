use itertools::Itertools;
use lazy_static::lazy_static;
use regex::{escape, Regex};
use tracing::{debug, error, warn};

use protolang_parser::hl::{Enum, Model, Type};
use protolang_parser::type_to_hl_codec;

use crate::{BUILTIN_FQN, convert_from_id, DEFINITION_FQN, REGEX_CACHE, get_types_from_generic, DEFINITION_FQN_2};

pub fn generate_model_server_actionscript_code(model: &Model, root_package: Option<&str>) -> String {
  let mut builder = String::new();

  let mut full_package = String::new();
  if let Some(root_package) = root_package {
    full_package.push_str(root_package);
    full_package.push_str(".");
  }
  if let Some(meta) = model.meta.iter().find(|it| it.key == "client_package") {
    full_package.push_str(&meta.value);
  }
  builder.push_str(&format!("package {} {{\n", full_package));

  builder.push_str(r#"  import alternativa.osgi.OSGi;
  import alternativa.protocol.ICodec;
  import alternativa.protocol.IProtocol;
  import alternativa.protocol.OptionalMap;
  import alternativa.protocol.ProtocolBuffer;
  import alternativa.protocol.info.TypeCodecInfo;
  import alternativa.protocol.info.EnumCodecInfo;
  import alternativa.protocol.info.CollectionCodecInfo;
  import alternativa.protocol.info.MapCodecInfo;
  import alternativa.types.Long;
  import flash.utils.ByteArray;
  import platform.client.fp10.core.model.IModel;
  import platform.client.fp10.core.model.impl.Model;
  import platform.client.fp10.core.network.command.SpaceCommand;
  import platform.client.fp10.core.type.IGameObject;
  import platform.client.fp10.core.type.ISpace;
"#);

  let mut imports = Vec::<String>::new();
  for method in &model.server_methods {
    for param in &method.params {
      imports.append(&mut get_types_from_generic(&convert_type(&param.kind, root_package)));
    }
  }
  for method in &model.client_methods {
    for param in &method.params {
      imports.append(&mut get_types_from_generic(&convert_type(&param.kind, root_package)));
    }
  }
  let imports = imports.iter().unique().map(|import| format!("  import {};", import)).join("\n");
  builder.push_str(&imports);
  builder.push_str("\n\n");

  let class_name = if let Some(meta) = model.meta.iter().find(|it| it.key == "client_name") {
    &meta.value
  } else {
    &model.name
  };
  builder.push_str(&format!("  public class {}Server {{\n", class_name));

  builder.push_str("    private var protocol:IProtocol;\n");
  builder.push_str("    private var protocolBuffer:ProtocolBuffer;\n");

  for method in &model.server_methods {
    builder.push_str(&format!("    private var _{}Id:Long;\n", method.name));

    for param in &method.params {
      builder.push_str(&format!("    private var _{}_{}Codec:ICodec;\n", method.name, param.name));
    }
    builder.push_str("\n");
  }

  builder.push_str("    private var model:IModel;\n");
  builder.push_str("\n");

  builder.push_str(&format!("    public function {}Server(model:IModel) {{\n", class_name));
  for method in &model.server_methods {
    let (high, low) = convert_from_id(method.id);
    builder.push_str(&format!("      this._{}Id = Long.getLong({},{});\n", method.name, high, low));
  }
  builder.push_str("      super();\n");
  builder.push_str("      this.model = model;\n");
  builder.push_str("      var buffer:ByteArray = new ByteArray();\n");
  builder.push_str("      this.protocol = IProtocol(OSGi.getInstance().getService(IProtocol));\n");
  builder.push_str("      this.protocolBuffer = new ProtocolBuffer(buffer,buffer,new OptionalMap());\n");
  for method in &model.server_methods {
    for param in &method.params {
      builder.push_str(&format!("      this._{}_{}Codec = this.protocol.getCodec({});\n", method.name, param.name, convert_type(&param.codec, root_package)));
    }
  }
  builder.push_str("    }\n");
  builder.push_str("\n");

  for method in &model.server_methods {
    let params = method.params.iter().map(|param| format!("{}:{}", param.name, convert_type_to_native_final(&convert_type(&param.kind, root_package)))).join(", ");
    builder.push_str(&format!("    public function {}({}) : void {{\n", method.name, params));
    builder.push_str("      ByteArray(this.protocolBuffer.writer).position = 0;\n");
    builder.push_str("      ByteArray(this.protocolBuffer.writer).length = 0;\n");
    for param in &method.params {
      builder.push_str(&format!("      this._{}_{}Codec.encode(this.protocolBuffer,{});\n", method.name, param.name, param.name));
    }
    builder.push_str("      ByteArray(this.protocolBuffer.writer).position = 0;\n");
    builder.push_str("      if(Model.object == null) {\n");
    builder.push_str("        throw new Error(\"Execute method without model context.\");\n");
    builder.push_str("      }\n");
    builder.push_str(&format!("      var spaceCommand:SpaceCommand = new SpaceCommand(Model.object.id,this._{}Id,this.protocolBuffer);\n", method.name));
    builder.push_str("      var gameObject:IGameObject = Model.object;\n");
    builder.push_str("      var space:ISpace = gameObject.space;\n");
    builder.push_str("      space.commandSender.sendCommand(spaceCommand);\n");
    builder.push_str("      this.protocolBuffer.optionalMap.clear();\n");
    builder.push_str("    }\n");
    builder.push_str("\n");
  }

  builder.push_str("  }\n");

  builder.push_str("}\n");

  builder
}

pub fn generate_model_base_actionscript_code(model: &Model, root_package: Option<&str>) -> String {
  let mut builder = String::new();

  let mut full_package = String::new();
  if let Some(root_package) = root_package {
    full_package.push_str(root_package);
    full_package.push_str(".");
  }
  if let Some(meta) = model.meta.iter().find(|it| it.key == "client_package") {
    full_package.push_str(&meta.value);
  }
  builder.push_str(&format!("package {} {{\n", full_package));

  builder.push_str(r#"  import alternativa.osgi.OSGi;
  import alternativa.protocol.ICodec;
  import alternativa.protocol.IProtocol;
  import alternativa.protocol.ProtocolBuffer;
  import alternativa.protocol.info.TypeCodecInfo;
  import alternativa.protocol.info.EnumCodecInfo;
  import alternativa.protocol.info.CollectionCodecInfo;
  import alternativa.protocol.info.MapCodecInfo;
  import alternativa.types.Long;
  import platform.client.fp10.core.model.IModel;
  import platform.client.fp10.core.model.impl.Model;
  import platform.client.fp10.core.registry.ModelRegistry;
"#);

  let mut imports = Vec::<String>::new();
  if let Some(constructor) = &model.constructor {
    imports.append(&mut get_types_from_generic(&convert_type(&format!("{}Base.Constructor", model.name), root_package)));
  }
  for method in &model.server_methods {
    for param in &method.params {
      imports.append(&mut get_types_from_generic(&convert_type(&param.kind, root_package)));
    }
  }
  for method in &model.client_methods {
    for param in &method.params {
      imports.append(&mut get_types_from_generic(&convert_type(&param.kind, root_package)));
    }
  }
  let imports = imports.iter().unique().map(|import| format!("  import {};", import)).join("\n");
  builder.push_str(&imports);
  builder.push_str("\n\n");

  let class_name = if let Some(meta) = model.meta.iter().find(|it| it.key == "client_name") {
    &meta.value
  } else {
    &model.name
  };
  builder.push_str(&format!("  public class {}Base extends Model {{\n", class_name));

  builder.push_str("    private var _protocol:IProtocol;\n");
  builder.push_str(&format!("    protected var server:{}Server;\n", class_name));
  builder.push_str(&format!("    private var client:I{}Base;\n", class_name));
  builder.push_str("    private var modelId:Long;\n");
  builder.push_str("\n");

  for method in &model.client_methods {
    builder.push_str(&format!("    private var _{}Id:Long;\n", method.name));

    for param in &method.params {
      builder.push_str(&format!("    private var _{}_{}Codec:ICodec;\n", method.name, param.name));
    }
    builder.push_str("\n");
  }

  builder.push_str(&format!("    public function {}Base() {{\n", class_name));
  builder.push_str("      this._protocol = IProtocol(OSGi.getInstance().getService(IProtocol));\n");
  builder.push_str(&format!("      this.client = I{}Base(this);\n", class_name));
  let (high, low) = convert_from_id(model.id);
  builder.push_str(&format!("      this.modelId = Long.getLong({},{});\n", high, low));
  for method in &model.client_methods {
    let (high, low) = convert_from_id(method.id);
    builder.push_str(&format!("      this._{}Id = Long.getLong({},{});\n", method.name, high, low));
  }
  builder.push_str("      super();\n");
  builder.push_str("      this.initCodecs();\n");
  builder.push_str("    }\n");
  builder.push_str("\n");

  builder.push_str("    protected function initCodecs() : void {\n");
  builder.push_str(&format!("      this.server = new {}Server(IModel(this));\n", class_name));
  builder.push_str("      var modelRegistry:ModelRegistry = ModelRegistry(OSGi.getInstance().getService(ModelRegistry));\n");
  if let Some(constructor) = &model.constructor {
    let constructor_class_name = if let Some(meta) = constructor.meta.iter().find(|it| it.key == "client_name") {
      &meta.value
    } else {
      todo!()
    };
    builder.push_str(&format!("      modelRegistry.registerModelConstructorCodec(this.modelId,this._protocol.getCodec(new TypeCodecInfo({},false)));\n", convert_type(constructor_class_name, root_package)));
  }
  for method in &model.client_methods {
    for param in &method.params {
      builder.push_str(&format!("      this._{}_{}Codec = this._protocol.getCodec({});\n", method.name, param.name, convert_type(&param.codec, root_package)));
    }
  }
  builder.push_str("    }\n");
  builder.push_str("\n");

  if let Some(constructor) = &model.constructor {
    let constructor_class_name = if let Some(meta) = constructor.meta.iter().find(|it| it.key == "client_name") {
      &meta.value
    } else {
      todo!()
    };
    builder.push_str(&format!("    protected function getInitParam() : {} {{\n", convert_type(constructor_class_name, root_package)));
    builder.push_str(&format!("      return {}(initParams[Model.object]);\n", convert_type(constructor_class_name, root_package)));
    builder.push_str("    }\n");
    builder.push_str("\n");
  }

  builder.push_str("    override public function invoke(methodId:Long, buffer:ProtocolBuffer) : void {\n");
  builder.push_str("      switch(methodId) {\n");
  for method in &model.client_methods {
    let mut params = Vec::new();
    for param in &method.params {
      let native_type = convert_type_to_native_final(&convert_type(&param.kind, root_package));
      params.push(format!("{}(this._{}_{}Codec.decode(buffer))", native_type, method.name, param.name));
    }

    builder.push_str(&format!("        case this._{}Id:\n", method.name));
    builder.push_str(&format!("          this.client.{}({});\n", method.name, params.join(", ")));
    builder.push_str("          break;\n");
  }
  builder.push_str("      }\n");
  builder.push_str("    }\n");
  builder.push_str("\n");

  builder.push_str("    override public function get id() : Long {\n");
  builder.push_str("      return this.modelId;\n");
  builder.push_str("    }\n");

  builder.push_str("  }\n");

  builder.push_str("}\n");

  builder
}

pub fn generate_model_client_interface_actionscript_code(model: &Model, root_package: Option<&str>) -> String {
  let mut builder = String::new();

  let mut full_package = String::new();
  if let Some(root_package) = root_package {
    full_package.push_str(root_package);
    full_package.push_str(".");
  }
  if let Some(meta) = model.meta.iter().find(|it| it.key == "client_package") {
    full_package.push_str(&meta.value);
  }
  builder.push_str(&format!("package {} {{\n", full_package));

  let mut imports = Vec::<String>::new();
  for method in &model.server_methods {
    for param in &method.params {
      imports.append(&mut get_types_from_generic(&convert_type_to_native_final(&convert_type(&param.kind, root_package))));
    }
  }
  for method in &model.client_methods {
    for param in &method.params {
      imports.append(&mut get_types_from_generic(&convert_type_to_native_final(&convert_type(&param.kind, root_package))));
    }
  }
  let imports = imports.iter().unique().map(|import| format!("  import {};", import)).join("\n");
  builder.push_str(&imports);
  builder.push_str("\n\n");

  let class_name = if let Some(meta) = model.meta.iter().find(|it| it.key == "client_name") {
    &meta.value
  } else {
    &model.name
  };
  builder.push_str(&format!("  public interface I{}Base {{\n", class_name));

  for method in &model.client_methods {
    let params = method.params.iter().map(|param| format!("{}:{}", param.name, convert_type_to_native_final(&convert_type(&param.kind, root_package)))).join(", ");
    builder.push_str(&format!(
      "    function {}({}) : void;\n",
      method.name,
      params
    ));
  }

  builder.push_str("  }\n");

  builder.push_str("}\n");

  builder
}

pub fn generate_type_actionscript_code(type_def: &Type, root_package: Option<&str>) -> String {
  let mut builder = String::new();

  let mut full_package = String::new();
  if let Some(root_package) = root_package {
    full_package.push_str(root_package);
    full_package.push_str(".");
  }
  if let Some(meta) = type_def.meta.iter().find(|it| it.key == "client_package") {
    full_package.push_str(&meta.value);
  }
  builder.push_str(&format!("package {} {{\n", full_package));

  let mut imports = Vec::<String>::new();
  for field in &type_def.fields {
    imports.append(&mut get_types_from_generic(&convert_type_to_native_final(&convert_type(&field.kind, root_package))));
  }
  let imports = imports.iter().unique().map(|import| format!("  import {};", import)).join("\n");
  builder.push_str(&imports);
  builder.push_str("\n\n");

  let class_name = if let Some(meta) = type_def.meta.iter().find(|it| it.key == "client_name") {
    &meta.value
  } else {
    &type_def.name
  };
  builder.push_str(&format!("  public class {} {{\n", class_name));

  for field in &type_def.fields {
    let native_type = &convert_type_to_native_final(&convert_type(&field.kind, root_package));
    builder.push_str(&format!(
      "    private var _{}:{};\n",
      field.name,
      native_type
    ));
  }
  if !type_def.fields.is_empty() {
    builder.push_str("\n");
  }

  let mut params = Vec::new();
  for field in &type_def.fields {
    let native_type = convert_type_to_native_final(&convert_type(&field.kind, root_package));
    let default = match native_type.as_str() {
      "int" => "0",
      "Number" => "0",
      "Boolean" => "false",
      _ => "null"
    };
    params.push(format!("{}:{} = {}", field.name, native_type, default));
  }
  builder.push_str(&format!("    public function {}({}) {{\n", class_name, params.join(", ")));
  builder.push_str("      super();\n");
  for field in &type_def.fields {
    builder.push_str(&format!("      this._{} = {};\n", field.name, field.name));
  }
  builder.push_str("    }\n");
  builder.push_str("\n");

  for field in &type_def.fields {
    let native_type = convert_type_to_native_final(&convert_type(&field.kind, root_package));
    builder.push_str(&format!("    public function get {}() : {} {{\n", field.name, native_type));
    builder.push_str(&format!("      return this._{};\n", field.name));
    builder.push_str("    }\n");
    builder.push_str("\n");
    builder.push_str(&format!("    public function set {}(value:{}) : void {{\n", field.name, native_type));
    builder.push_str(&format!("      this._{} = value;\n", field.name));
    builder.push_str("    }\n");
    builder.push_str("\n");
  }

  builder.push_str("    public function toString() : String {\n");
  builder.push_str(&format!("      var string:String = \"{} [\";\n", class_name));
  for field in &type_def.fields {
    builder.push_str(&format!("      string += \"{} = \" + this._{} + \" \";\n", field.name, field.name));
  }
  builder.push_str("      return string + \"]\";\n");
  builder.push_str("    }\n");
  builder.push_str("\n");

  builder.push_str("  }\n");

  builder.push_str("}\n");

  builder
}

pub fn generate_enum_actionscript_code(enum_def: &Enum, root_package: Option<&str>) -> String {
  let mut builder = String::new();

  let mut full_package = String::new();
  if let Some(root_package) = root_package {
    full_package.push_str(root_package);
    full_package.push_str(".");
  }
  if let Some(meta) = enum_def.meta.iter().find(|it| it.key == "client_package") {
    full_package.push_str(&meta.value);
  }
  builder.push_str(&format!("package {} {{\n", full_package));

  let class_name = if let Some(meta) = enum_def.meta.iter().find(|it| it.key == "client_name") {
    &meta.value
  } else {
    &enum_def.name
  };
  builder.push_str(&format!("  public class {} {{\n", class_name));

  for variant in &enum_def.variants {
    builder.push_str(&format!(
      "    public static const {}:{} = new {}({},\"{}\");\n",
      variant.name,
      class_name,
      class_name,
      variant.value,
      variant.name
    ));
  }
  if !enum_def.variants.is_empty() {
    builder.push_str("\n");
  }

  let native_repr = convert_type(&enum_def.repr, root_package);

  builder.push_str(&format!("    private var _value:{};\n", native_repr));
  builder.push_str("    private var _name:String;\n");
  builder.push_str("\n");

  builder.push_str(&format!("    public function {}(value:{}, name:String) {{\n", class_name, native_repr));
  builder.push_str("      super();\n");
  builder.push_str("      this._value = value;\n");
  builder.push_str("      this._name = name;\n");
  builder.push_str("    }\n");
  builder.push_str("\n");

  builder.push_str(&format!("    public static function get values() : Vector.<{}> {{\n", class_name));
  builder.push_str(&format!("      var values:Vector.<{}> = new Vector.<{}>();\n", class_name, class_name));
  for variant in &enum_def.variants {
    builder.push_str(&format!("      values.push({});\n", variant.name));
  }
  builder.push_str("      return values;\n");
  builder.push_str("    }\n");
  builder.push_str("\n");

  builder.push_str("    public function toString() : String {\n");
  builder.push_str(&format!("      return \"{} [\" + this._name + \"]\";\n", class_name));
  builder.push_str("    }\n");
  builder.push_str("\n");

  builder.push_str(&format!("    public function get value() : {} {{\n", native_repr));
  builder.push_str("      return this._value;\n");
  builder.push_str("    }\n");
  builder.push_str("\n");

  builder.push_str("    public function get name() : String {\n");
  builder.push_str("      return this._name;\n");
  builder.push_str("    }\n");

  builder.push_str("  }\n");

  builder.push_str("}\n");

  builder
}

pub fn generate_type_codec_actionscript_code(type_def: &Type, root_package: Option<&str>) -> String {
  let mut builder = String::new();

  let mut full_package = String::new();
  full_package.push_str("_codec.");
  if let Some(root_package) = root_package {
    full_package.push_str(root_package);
    full_package.push_str(".");
  }
  if let Some(meta) = type_def.meta.iter().find(|it| it.key == "client_package") {
    full_package.push_str(&meta.value);
  }
  builder.push_str(&format!("package {} {{\n", full_package));

  let class_name = if let Some(meta) = type_def.meta.iter().find(|it| it.key == "client_name") {
    &meta.value
  } else {
    &type_def.name
  };

  builder.push_str(r#"  import alternativa.osgi.OSGi;
  import alternativa.osgi.service.clientlog.IClientLog;
  import alternativa.protocol.ICodec;
  import alternativa.protocol.IProtocol;
  import alternativa.protocol.ProtocolBuffer;
  import alternativa.protocol.info.TypeCodecInfo;
  import alternativa.protocol.info.EnumCodecInfo;
  import alternativa.protocol.info.CollectionCodecInfo;
  import alternativa.protocol.info.MapCodecInfo;
"#);

  let mut imports = Vec::<String>::new();
  imports.append(&mut get_types_from_generic(&convert_type(class_name, root_package)));
  for field in &type_def.fields {
    imports.append(&mut get_types_from_generic(&convert_type(&field.kind, root_package)));
  }
  let imports = imports.iter().unique().map(|import| format!("  import {};", import)).join("\n");
  builder.push_str(&imports);
  builder.push_str("\n\n");

  builder.push_str(&format!("  public class Codec{} implements ICodec {{\n", class_name));
  builder.push_str("    public static var log:IClientLog = IClientLog(OSGi.getInstance().getService(IClientLog));\n\n");

  for field in &type_def.fields {
    builder.push_str(&format!(
      "    private var codec_{}:ICodec;\n",
      field.name
    ));
  }
  if !type_def.fields.is_empty() {
    builder.push_str("\n");
  }

  builder.push_str(&format!("    public function Codec{}() {{\n", class_name));
  builder.push_str("      super();\n");
  builder.push_str("    }\n");
  builder.push_str("\n");

  builder.push_str("    public function init(protocol:IProtocol) : void {\n");
  for field in &type_def.fields {
    // Do not call [convert_type_to_native_final] because int conflicts with Short and Byte
    let native_codec = convert_type(&field.codec, root_package);
    builder.push_str(&format!("      this.codec_{} = protocol.getCodec({});\n", field.name, native_codec));
  }
  builder.push_str("    }\n");
  builder.push_str("\n");

  builder.push_str("    public function decode(buffer:ProtocolBuffer) : Object {\n");
  builder.push_str(&format!("      var result:{} = new {}();\n", convert_type_to_native_final(&convert_type(class_name, root_package)), convert_type_to_native_final(&convert_type(class_name, root_package))));
  for field in &type_def.fields {
    let native_type = convert_type_to_native_final(&convert_type(&field.kind, root_package));
    builder.push_str(&format!("      result.{} = this.codec_{}.decode(buffer) as {};\n", field.name, field.name, native_type));
  }
  builder.push_str("      return result;\n");
  builder.push_str("    }\n");
  builder.push_str("\n");

  builder.push_str("    public function encode(buffer:ProtocolBuffer, value:Object) : void {\n");
  builder.push_str("      if(value == null) {\n");
  builder.push_str("        throw new Error(\"Object is null. Use @ProtocolOptional annotation.\");\n");
  builder.push_str("      }\n");
  builder.push_str(&format!("      var castValue:{} = {}(value);\n", convert_type_to_native_final(&convert_type(class_name, root_package)), convert_type_to_native_final(&convert_type(class_name, root_package))));
  for field in &type_def.fields {
    let native_type = convert_type_to_native_final(&convert_type(&field.kind, root_package));
    builder.push_str(&format!("      this.codec_{}.encode(buffer,castValue.{});\n", field.name, field.name));
  }
  builder.push_str("    }\n");
  builder.push_str("\n");

  builder.push_str("  }\n");

  builder.push_str("}\n");

  builder
}

pub fn generate_enum_codec_actionscript_code(enum_def: &Enum, root_package: Option<&str>) -> String {
  let mut builder = String::new();

  let mut full_package = String::new();
  full_package.push_str("_codec.");
  if let Some(root_package) = root_package {
    full_package.push_str(root_package);
    full_package.push_str(".");
  }
  if let Some(meta) = enum_def.meta.iter().find(|it| it.key == "client_package") {
    full_package.push_str(&meta.value);
  }
  builder.push_str(&format!("package {} {{\n", full_package));

  let class_name = if let Some(meta) = enum_def.meta.iter().find(|it| it.key == "client_name") {
    &meta.value
  } else {
    &enum_def.name
  };

  builder.push_str(r#"  import alternativa.protocol.ICodec;
  import alternativa.protocol.IProtocol;
  import alternativa.protocol.ProtocolBuffer;
"#);

  let mut imports = Vec::<String>::new();
  imports.append(&mut get_types_from_generic(&convert_type(class_name, root_package)));
  let imports = imports.iter().unique().map(|import| format!("  import {};", import)).join("\n");
  builder.push_str(&imports);
  builder.push_str("\n\n");

  builder.push_str(&format!("  public class Codec{} implements ICodec {{\n", class_name));

  builder.push_str(&format!("    public function Codec{}() {{\n", class_name));
  builder.push_str("      super();\n");
  builder.push_str("    }\n");
  builder.push_str("\n");

  builder.push_str("    public function init(protocol:IProtocol) : void {\n");
  builder.push_str("    }\n");
  builder.push_str("\n");

  let native_type = convert_type(class_name, root_package);
  let native_repr = convert_type(&enum_def.repr, root_package);
  builder.push_str("    public function decode(buffer:ProtocolBuffer) : Object {\n");
  builder.push_str(&format!("      var result:{} = null;\n", native_type));
  assert_eq!(enum_def.repr, "i32");
  builder.push_str(&format!("      var repr:{} = {}(buffer.reader.readInt());\n", native_repr, native_repr));
  builder.push_str("      switch(repr) {\n");
  for variant in &enum_def.variants {
    builder.push_str(&format!("        case {}:\n", variant.value));
    builder.push_str(&format!("          result = {}.{};\n", native_type, variant.name));
    builder.push_str("          break;\n");
  }
  builder.push_str("      }\n");
  builder.push_str("      return result;\n");
  builder.push_str("    }\n");
  builder.push_str("\n");

  builder.push_str("    public function encode(buffer:ProtocolBuffer, value:Object) : void {\n");
  builder.push_str("      if(value == null) {\n");
  builder.push_str("        throw new Error(\"Object is null. Use @ProtocolOptional annotation.\");\n");
  builder.push_str("      }\n");
  builder.push_str(&format!("      var repr:{} = {}(value);\n", native_repr, native_repr));
  assert_eq!(enum_def.repr, "i32");
  builder.push_str("      buffer.writer.writeInt(repr);\n");
  builder.push_str("    }\n");
  builder.push_str("\n");

  builder.push_str("  }\n");

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
  static ref REGEX_8: Regex = Regex::new(r"\bObject3DResource\b").unwrap();
  static ref REGEX_9: Regex = Regex::new(r"\bInstant\b").unwrap();
  static ref REGEX_10: Regex = Regex::new(r"\bList<").unwrap();
  static ref REGEX_11: Regex = Regex::new(r"\bMap<.+>").unwrap();
  static ref REGEX_NULLABLE: Regex = Regex::new(r"\?").unwrap();

  static ref REGEX_12: Regex = Regex::new(r"\balternativa\.types\.(Byte|Short)\b").unwrap();
  static ref REGEX_13: Regex = Regex::new(r"\balternativa\.types\.Float\b").unwrap();
}

pub fn convert_type_to_native_final(value: &str) -> String {
  let value = REGEX_12.replace_all(&value, "int");
  let value = REGEX_13.replace_all(&value, "Number");
  value.to_string()
}

pub fn convert_type(value: &str, root_package: Option<&str>) -> String {
  let value = REGEX_1.replace_all(&value, "Boolean");
  let value = REGEX_2.replace_all(&value, "Byte");
  let value = REGEX_3.replace_all(&value, "Short");
  let value = REGEX_4.replace_all(&value, "int");
  let value = REGEX_5.replace_all(&value, "Long");
  let value = REGEX_6.replace_all(&value, "Float");
  let value = REGEX_7.replace_all(&value, "Number");
  let value = REGEX_8.replace_all(&value, "Tanks3DSResource");
  let value = REGEX_9.replace_all(&value, "Date");
  let value = REGEX_10.replace_all(&value, "Vector.<");
  let value = REGEX_11.replace_all(&value, "Dictionary");
  let value = REGEX_NULLABLE.replace_all(&value, "");

  let mut cache = REGEX_CACHE.lock().unwrap();

  // TODO: What the fuck
  let mut value = value.to_string();
  let paths = BUILTIN_FQN.lock().unwrap();
  for (simple_name, full_name) in paths.iter() {
    let regex = cache.entry(simple_name.to_owned()).or_insert_with(|| Regex::new(&format!(r"\b{}\b", escape(simple_name))).unwrap());
    value = regex.replace_all(&value, full_name).to_string();
  }

  let definitions = DEFINITION_FQN.lock().unwrap();
  for (simple_name, full_name) in definitions.iter() {
    let mut fqn = String::new();
    if let Some(root_package) = root_package {
      fqn.push_str(root_package);
      fqn.push_str(".");
    }
    fqn.push_str(full_name);

    // Replace all "ShortName" with "fqn.FullName"
    let regex = cache.entry(format!("{}.level1", simple_name)).or_insert_with(|| Regex::new(&format!(r"\b{}\b", escape(simple_name))).unwrap());
    let old_value = value.clone();
    value = regex.replace_all(&value, fqn).to_string();
    if value != old_value {
      debug!("replaced level 1 {old_value} -> {value}");
    }
  }

  let definitions = DEFINITION_FQN_2.lock().unwrap();
  for (simple_name, full_name) in definitions.iter() {
    let mut fqn = String::new();
    if let Some(root_package) = root_package {
      fqn.push_str(root_package);
      fqn.push_str(".");
    }
    fqn.push_str(full_name);

    // Replace all "ShortName" with "fqn.FullName"
    let regex = cache.entry(format!("{}.level2", simple_name)).or_insert_with(|| Regex::new(&format!(r"\b{}\b", escape(simple_name))).unwrap());
    let old_value = value.clone();
    value = regex.replace_all(&value, fqn).to_string();
    if value != old_value {
      debug!("replaced level 2 {old_value} -> {value}");
    }
  }

  value.to_string()
}
