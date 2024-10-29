use crate::parser::ParsedData;
use crate::rust_types::{RustEnumShared, RustItem, RustType, RustTypeFormatError, SpecialRustType};
use crate::topsort::topsort;
use crate::{
    language::Language,
    rust_types::{RustEnum, RustEnumVariant, RustField, RustStruct, RustTypeAlias},
};
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::hash::Hash;
use std::{collections::HashMap, io::Write};

use super::CrateTypes;

use convert_case::{Case, Casing};
// Collect unique type vars from an enum field
// Since we explode enums into unions of types, we need to extract all of the generics
// used by each individual field
// We do this by exploring each field's type and comparing against the generics used by the enum
// itself
fn collect_generics_for_variant(variant_type: &RustType, generics: &[String]) -> Vec<String> {
    let mut all = vec![];
    match variant_type {
        RustType::Generic { id, parameters } => {
            if generics.contains(id) {
                all.push(id.clone())
            }
            // Recurse into the params for the case of `Foo(HashMap<K, V>)`
            for param in parameters {
                all.extend(collect_generics_for_variant(param, generics))
            }
        }
        RustType::Simple { id } => {
            if generics.contains(id) {
                all.push(id.clone())
            }
        }
        RustType::Special(special) => match &special {
            SpecialRustType::HashMap(key_type, value_type) => {
                all.extend(collect_generics_for_variant(key_type, generics));
                all.extend(collect_generics_for_variant(value_type, generics));
            }
            SpecialRustType::Option(some_type) => {
                all.extend(collect_generics_for_variant(some_type, generics));
            }
            SpecialRustType::Vec(value_type) => {
                all.extend(collect_generics_for_variant(value_type, generics));
            }
            _ => {}
        },
    }
    // Remove any duplicates
    // E.g. Foo(HashMap<T, T>) should only produce a single type var
    dedup(&mut all);
    all
}

fn dedup<T: Eq + Hash + Clone>(v: &mut Vec<T>) {
    // note the Copy constraint
    let mut uniques = HashSet::new();
    v.retain(|e| uniques.insert(e.clone()));
}

/// All information needed to generate Python type-code
#[derive(Default)]
pub struct Python {
    /// Mappings from Rust type names to Python type names
    pub type_mappings: HashMap<String, String>,
    // HashMap<ModuleName, HashSet<Identifier>
    pub imports: HashMap<String, HashSet<String>>,
    // HashMap<Identifier, Vec<DependencyIdentifiers>>
    // Used to lay out runtime references in the module
    // such that it can be read top to bottom
    // globals: HashMap<String, Vec<String>>,
    pub type_variables: HashSet<String>,
}

impl Language for Python {
    fn type_map(&mut self) -> &HashMap<String, String> {
        &self.type_mappings
    }
    fn generate_types(
        &mut self,
        w: &mut dyn Write,
        _imports: &CrateTypes,
        data: ParsedData,
    ) -> std::io::Result<()> {
        self.begin_file(w, &data)?;

        let ParsedData {
            structs,
            enums,
            aliases,
            ..
        } = data;

        let mut items = aliases
            .into_iter()
            .map(RustItem::Alias)
            .chain(structs.into_iter().map(RustItem::Struct))
            .chain(enums.into_iter().map(RustItem::Enum))
            .collect::<Vec<_>>();

        topsort(&mut items);

        let mut body: Vec<u8> = Vec::new();
        for thing in items {
            match thing {
                RustItem::Enum(e) => self.write_enum(&mut body, &e)?,
                RustItem::Struct(rs) => self.write_struct(&mut body, &rs)?,
                RustItem::Alias(t) => self.write_type_alias(&mut body, &t)?,
            };
        }

        self.write_all_imports(w)?;

        w.write_all(&body)?;
        Ok(())
    }

    fn format_generic_type(
        &mut self,
        base: &String,
        parameters: &[RustType],
        generic_types: &[String],
    ) -> Result<String, RustTypeFormatError> {
        if let Some(mapped) = self.type_map().get(base) {
            Ok(mapped.into())
        } else {
            let parameters: Result<Vec<String>, RustTypeFormatError> = parameters
                .iter()
                .map(|p| self.format_type(p, generic_types))
                .collect();
            let parameters = parameters?;
            Ok(format!(
                "{}{}",
                self.format_simple_type(base, generic_types)?,
                (!parameters.is_empty())
                    .then(|| format!("[{}]", parameters.join(", ")))
                    .unwrap_or_default()
            ))
        }
    }

    fn format_simple_type(
        &mut self,
        base: &String,
        _generic_types: &[String],
    ) -> Result<String, RustTypeFormatError> {
        self.add_imports(base);
        Ok(if let Some(mapped) = self.type_map().get(base) {
            mapped.into()
        } else {
            base.into()
        })
    }

    fn format_special_type(
        &mut self,
        special_ty: &SpecialRustType,
        generic_types: &[String],
    ) -> Result<String, RustTypeFormatError> {
        match special_ty {
            SpecialRustType::Vec(rtype)
            | SpecialRustType::Array(rtype, _)
            | SpecialRustType::Slice(rtype) => {
                self.add_import("typing".to_string(), "List".to_string());
                Ok(format!("List[{}]", self.format_type(rtype, generic_types)?))
            }
            // We add optionality above the type formatting level
            SpecialRustType::Option(rtype) => self.format_type(rtype, generic_types),
            SpecialRustType::HashMap(rtype1, rtype2) => {
                self.add_import("typing".to_string(), "Dict".to_string());
                Ok(format!(
                    "Dict[{}, {}]",
                    match rtype1.as_ref() {
                        RustType::Simple { id } if generic_types.contains(id) => {
                            return Err(RustTypeFormatError::GenericKeyForbiddenInTS(id.clone()));
                        }
                        _ => self.format_type(rtype1, generic_types)?,
                    },
                    self.format_type(rtype2, generic_types)?
                ))
            }
            SpecialRustType::Unit => Ok("None".into()),
            SpecialRustType::String | SpecialRustType::Char => Ok("str".into()),
            SpecialRustType::I8
            | SpecialRustType::U8
            | SpecialRustType::I16
            | SpecialRustType::U16
            | SpecialRustType::I32
            | SpecialRustType::U32
            | SpecialRustType::I54
            | SpecialRustType::U53
            | SpecialRustType::U64
            | SpecialRustType::I64
            | SpecialRustType::ISize
            | SpecialRustType::USize => Ok("int".into()),
            SpecialRustType::F32 | SpecialRustType::F64 => Ok("float".into()),
            SpecialRustType::Bool => Ok("bool".into()),
        }
    }

    fn begin_file(&mut self, w: &mut dyn Write, _parsed_data: &ParsedData) -> std::io::Result<()> {
        writeln!(w, "\"\"\"")?;
        writeln!(w, " Generated by typeshare {}", env!("CARGO_PKG_VERSION"))?;
        writeln!(w, "\"\"\"")?;
        Ok(())
    }

    fn write_type_alias(&mut self, w: &mut dyn Write, ty: &RustTypeAlias) -> std::io::Result<()> {
        let r#type = self
            .format_type(&ty.r#type, ty.generic_types.as_slice())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        writeln!(
            w,
            "{}{} = {}\n\n",
            ty.id.renamed,
            (!ty.generic_types.is_empty())
                .then(|| format!("[{}]", ty.generic_types.join(", ")))
                .unwrap_or_default(),
            r#type,
        )?;

        self.write_comments(w, true, &ty.comments, 0)?;

        Ok(())
    }

    fn write_struct(&mut self, w: &mut dyn Write, rs: &RustStruct) -> std::io::Result<()> {
        {
            rs.generic_types
                .iter()
                .cloned()
                .for_each(|v| self.add_type_var(v))
        }
        let bases = match rs.generic_types.is_empty() {
            true => "BaseModel".to_string(),
            false => {
                self.add_import("pydantic.generics".to_string(), "GenericModel".to_string());
                self.add_import("typing".to_string(), "Generic".to_string());
                format!("GenericModel, Generic[{}]", rs.generic_types.join(", "))
            }
        };
        writeln!(w, "class {}({}):", rs.id.renamed, bases,)?;

        self.write_comments(w, true, &rs.comments, 1)?;

        handle_model_config(w, self, rs);

        rs.fields
            .iter()
            .try_for_each(|f| self.write_field(w, f, rs.generic_types.as_slice()))?;

        if rs.fields.is_empty() {
            write!(w, "    pass")?
        }
        write!(w, "\n\n")?;
        self.add_import("pydantic".to_string(), "BaseModel".to_string());
        Ok(())
    }

    fn write_enum(&mut self, w: &mut dyn Write, e: &RustEnum) -> std::io::Result<()> {
        // Make a suitable name for an anonymous struct enum variant
        let make_anonymous_struct_name =
            |variant_name: &str| format!("{}{}", &e.shared().id.original, variant_name);

        // Generate named types for any anonymous struct variants of this enum
        self.write_types_for_anonymous_structs(w, e, &make_anonymous_struct_name)?;

        match e {
            // Write all the unit variants out (there can only be unit variants in
            // this case)
            RustEnum::Unit(shared) => {
                self.add_import("typing".to_string(), "Literal".to_string());
                write!(
                    w,
                    "{} = Literal[{}]",
                    shared.id.renamed,
                    shared
                        .variants
                        .iter()
                        .map(|v| format!(
                            "\"{}\"",
                            match v {
                                RustEnumVariant::Unit(v) => {
                                    v.id.renamed.replace("\"", "\\\"")
                                }
                                _ => panic!(),
                            }
                        ))
                        .collect::<Vec<String>>()
                        .join(", ")
                )?;
                write!(w, "\n\n")?;
            }
            // Write all the algebraic variants out (all three variant types are possible
            // here)
            RustEnum::Algebraic {
                tag_key,
                content_key,
                shared,
                ..
            } => {
                self.write_algebraic_enum(
                    tag_key,
                    content_key,
                    shared,
                    w,
                    &make_anonymous_struct_name,
                )?;
            }
        };
        Ok(())
    }

    fn write_imports(
        &mut self,
        _writer: &mut dyn Write,
        _imports: super::ScopedCrateTypes<'_>,
    ) -> std::io::Result<()> {
        todo!()
    }
}

impl Python {
    fn add_imports(&mut self, tp: &str) {
        match tp {
            "Url" => {
                self.add_import("pydantic.networks".to_string(), "AnyUrl".to_string());
            }
            "DateTime" => {
                self.add_import("datetime".to_string(), "datetime".to_string());
            }
            _ => {}
        }
    }

    fn write_field(
        &mut self,
        w: &mut dyn Write,
        field: &RustField,
        generic_types: &[String],
    ) -> std::io::Result<()> {
        let mut python_type = self
            .format_type(&field.ty, generic_types)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let python_field_name = python_property_aware_rename(&field.id.original);
        if field.ty.is_optional() {
            python_type = format!("Optional[{}]", python_type);
            self.add_import("typing".to_string(), "Optional".to_string());
        }
        python_type = match python_field_name == field.id.renamed {
            true => python_type,
            false => {
                self.add_import("typing".to_string(), "Annotated".to_string());
                self.add_import("pydantic".to_string(), "Field".to_string());
                format!(
                    "Annotated[{}, Field(alias=\"{}\")]",
                    python_type, field.id.renamed
                )
            }
        };
        // TODO: Add support for default values other than None
        match field.has_default && field.ty.is_optional() {
            true => {
                // in the future we will want to get the default value properly, something like:
                // let default_value = get_default_value(...)
                let default_value = "None";
                writeln!(
                    w,
                    "    {python_field_name}: {python_type} = {default_value}"
                )?
            }
            false => writeln!(w, "    {python_field_name}: {python_type}")?,
        }

        self.write_comments(w, true, &field.comments, 1)?;
        Ok(())
    }

    fn write_comments(
        &self,
        w: &mut dyn Write,
        is_docstring: bool,
        comments: &[String],
        indent_level: usize,
    ) -> std::io::Result<()> {
        // Only attempt to write a comment if there are some, otherwise we're Ok()
        let indent = "    ".repeat(indent_level);
        if !comments.is_empty() {
            let comment: String = {
                if is_docstring {
                    format!(
                        "{indent}\"\"\"\n{indented_comments}\n{indent}\"\"\"",
                        indent = indent,
                        indented_comments = comments
                            .iter()
                            .map(|v| format!("{}{}", indent, v))
                            .collect::<Vec<String>>()
                            .join("\n"),
                    )
                } else {
                    comments
                        .iter()
                        .map(|v| format!("{}# {}", indent, v))
                        .collect::<Vec<String>>()
                        .join("\n")
                }
            };
            writeln!(w, "{}", comment)?;
        }
        Ok(())
    }

    // Idempotently insert an import
    fn add_import(&mut self, module: String, identifier: String) {
        self.imports.entry(module).or_default().insert(identifier);
    }

    fn add_type_var(&mut self, name: String) {
        self.add_import("typing".to_string(), "TypeVar".to_string());
        self.type_variables.insert(name);
    }

    fn get_type_vars(&mut self, n: usize) -> Vec<String> {
        let vars: Vec<String> = (0..n)
            .map(|i| {
                if i == 0 {
                    "T".to_string()
                } else {
                    format!("T{}", i)
                }
            })
            .collect();
        vars.iter().for_each(|tv| self.add_type_var(tv.clone()));
        vars
    }

    fn write_all_imports(&self, w: &mut dyn Write) -> std::io::Result<()> {
        let mut type_var_names: Vec<String> = self.type_variables.iter().cloned().collect();
        type_var_names.sort();
        let type_vars: Vec<String> = type_var_names
            .iter()
            .map(|name| format!("{} = TypeVar(\"{}\")", name, name))
            .collect();
        let mut imports = vec![];
        for (import_module, identifiers) in &self.imports {
            let mut identifier_vec = identifiers.iter().cloned().collect::<Vec<String>>();
            identifier_vec.sort();
            imports.push(format!(
                "from {} import {}",
                import_module,
                identifier_vec.join(", ")
            ))
        }
        imports.sort();

        writeln!(w, "from __future__ import annotations\n")?;
        writeln!(w, "{}\n", imports.join("\n"))?;

        match type_vars.is_empty() {
            true => writeln!(w)?,
            false => writeln!(w, "{}\n\n", type_vars.join("\n"))?,
        };
        Ok(())
    }

    fn write_algebraic_enum(
        &mut self,
        tag_key: &str,
        content_key: &str,
        shared: &RustEnumShared,
        w: &mut dyn Write,
        make_struct_name: &dyn Fn(&str) -> String,
    ) -> std::io::Result<()> {
        shared
            .generic_types
            .iter()
            .cloned()
            .for_each(|v| self.add_type_var(v));
        let mut variants: Vec<(String, Vec<String>)> = Vec::new();
        self.add_import("typing".to_string(), "Union".to_string());
        self.add_import("enum".to_string(), "Enum".to_string());
        self.add_import("pydantic".to_string(), "ConfigDict".to_string());
        // write "types" class: a union of all the enum variants
        writeln!(w, "class {}Types(str, Enum):", shared.id.renamed)?;
        let all_enum_variants_name = shared
            .variants
            .iter()
            .map(|v| match v {
                RustEnumVariant::Unit(v) => v.id.renamed.clone(),
                RustEnumVariant::Tuple { shared, .. } => shared.id.renamed.clone(),
                RustEnumVariant::AnonymousStruct { shared, .. } => shared.id.renamed.clone(),
            })
            .collect::<Vec<String>>();
        writeln!(
            w,
            "{}",
            all_enum_variants_name
                .iter()
                .map(|name| format!(
                    "    {} = \"{}\"",
                    name.to_case(Case::Snake).to_uppercase(),
                    name
                ))
                .collect::<Vec<String>>()
                .join("\n")
        )?;
        writeln!(w)?;

        let mut variant_class_names = vec![];
        // write each of the enum variant as a class:
        for variant in &shared.variants {
            let class_name = make_struct_name(&variant.shared().id.original);
            variant_class_names.push(class_name.clone());

            match variant {
                RustEnumVariant::Unit(unit_variant) => {
                    self.add_import("typing".to_string(), "Literal".to_string());
                    let variant_name = format!("{}{}", shared.id.renamed, unit_variant.id.renamed);
                    variants.push((variant_name.clone(), vec![]));
                    writeln!(w, "class {class_name}(BaseModel):")?;
                    writeln!(
                        w,
                        "    {content_key} = Literal[\"{}\"]",
                        // escape any double quotes in the variant name
                        unit_variant.id.renamed.replace("\"", "\\\"")
                    )?;
                    writeln!(w)?;
                }
                RustEnumVariant::Tuple { ty, .. } => {
                    self.add_import("typing".to_string(), "Literal".to_string());
                    match ty {
                        RustType::Generic { parameters, .. } => {
                            let mut generic_parameters: Vec<String> = parameters
                                .iter()
                                .flat_map(|p| {
                                    collect_generics_for_variant(p, &shared.generic_types)
                                })
                                .collect();
                            dedup(&mut generic_parameters);
                            if generic_parameters.is_empty() {
                                self.add_import("pydantic".to_string(), "BaseModel".to_string());
                                writeln!(w, "class {class_name}(BaseModel):",).unwrap();
                            } else {
                                self.add_import("typing".to_string(), "Generic".to_string());
                                self.add_import(
                                    "pydantic.generics".to_string(),
                                    "GenericModel".to_string(),
                                );
                                writeln!(
                                    w,
                                    "class {class_name}(GenericModel, Generic[{}]):",
                                    // note: generics is always unique (a single item)
                                    generic_parameters.join(", ")
                                )
                                .unwrap();
                            }
                        }
                        other => {
                            let mut generics = vec![];
                            if let RustType::Simple { id } = other {
                                // This could be a bare generic
                                if shared.generic_types.contains(id) {
                                    generics = vec![id.clone()];
                                }
                            }
                            if generics.is_empty() {
                                self.add_import("pydantic".to_string(), "BaseModel".to_string());
                                writeln!(w, "class {class_name}(BaseModel):",).unwrap();
                            } else {
                                self.add_import("typing".to_string(), "Generic".to_string());
                                self.add_import(
                                    "pydantic.generics".to_string(),
                                    "GenericModel".to_string(),
                                );
                                writeln!(
                                    w,
                                    "class {class_name}(GenericModel, Generic[{}]):",
                                    generics.join(", ")
                                )
                                .unwrap();
                            }
                        }
                    }
                    let python_type = self
                        .format_type(ty, shared.generic_types.as_slice())
                        .unwrap();
                    writeln!(w, "    {content_key}: {python_type}")?;
                    writeln!(w)?;
                }
                RustEnumVariant::AnonymousStruct { .. } => {
                    // taken care of by write_types_for_anonymous_structs in write_enum
                }
            }
        }
        // finally, write the enum class itself consists of a type and a union of all the enum variants

        writeln!(w, "class {}(BaseModel):", shared.id.renamed)?;
        writeln!(w, "    model_config = ConfigDict(use_enum_values=True)")?;
        writeln!(w, "    {tag_key}: {}Types", shared.id.renamed)?;
        writeln!(
            w,
            "    {content_key}: Union[{}]",
            variant_class_names.join(", ")
        )?;
        writeln!(w)?;
        Ok(())
    }
}

static PYTHON_KEYWORDS: Lazy<HashSet<String>> = Lazy::new(|| {
    HashSet::from_iter(
        vec![
            "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class",
            "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global",
            "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise",
            "return", "try", "while", "with", "yield",
        ]
        .iter()
        .map(|v| v.to_string()),
    )
});

fn python_property_aware_rename(name: &str) -> String {
    let snake_name = name.to_case(Case::Snake);
    match PYTHON_KEYWORDS.contains(&snake_name) {
        true => format!("{}_", name),
        false => snake_name,
    }
}

// If at least one field from within a class is changed when the serde rename is used (a.k.a the field has 2 words) then we must use aliasing and we must also use a config dict at the top level of the class.
fn handle_model_config(w: &mut dyn Write, python_module: &mut Python, rs: &RustStruct) {
    let visibly_renamed_field = rs.fields.iter().find(|f| {
        let python_field_name = python_property_aware_rename(&f.id.original);
        python_field_name != f.id.renamed
    });
    if visibly_renamed_field.is_some() {
        python_module.add_import("pydantic".to_string(), "ConfigDict".to_string());
        let _ = writeln!(w, "    model_config = ConfigDict(populate_by_name=True)\n");
    };
}

#[cfg(test)]
mod test {
    use syn::{parse_str, ItemEnum};

    use crate::{
        parser::parse_enum,
        rust_types::{Id, RustEnumShared, RustEnumVariantShared},
    };

    use super::*;
    #[test]
    fn test_python_property_aware_rename() {
        assert_eq!(python_property_aware_rename("class"), "class_");
        assert_eq!(python_property_aware_rename("snake_case"), "snake_case");
    }

    #[test]
    fn test_optional_value_with_serde_default() {
        let mut python = Python::default();
        let mock_writer = &mut Vec::new();
        let rust_field = RustField {
            id: Id {
                original: "field".to_string(),
                renamed: "field".to_string(),
            },
            ty: RustType::Special(SpecialRustType::Option(Box::new(RustType::Simple {
                id: "str".to_string(),
            }))),
            has_default: true,
            comments: Default::default(),
            decorators: Default::default(),
        };
        python.write_field(mock_writer, &rust_field, &[]).unwrap();
        assert_eq!(
            String::from_utf8_lossy(mock_writer),
            "    field: Optional[str] = None\n"
        );
    }

    #[test]
    fn test_optional_value_no_serde_default() {
        let mut python = Python::default();
        let mock_writer = &mut Vec::new();
        let rust_field = RustField {
            id: Id {
                original: "field".to_string(),
                renamed: "field".to_string(),
            },
            ty: RustType::Special(SpecialRustType::Option(Box::new(RustType::Simple {
                id: "str".to_string(),
            }))),
            has_default: false,
            comments: Default::default(),
            decorators: Default::default(),
        };
        python.write_field(mock_writer, &rust_field, &[]).unwrap();
        assert_eq!(
            String::from_utf8_lossy(mock_writer),
            "    field: Optional[str]\n"
        );
    }

    #[test]
    fn test_non_optional_value_with_serde_default() {
        // technically an invalid case at the moment, as we don't support serde default values other than None
        // TODO: change this test if we do
        let mut python = Python::default();
        let mock_writer = &mut Vec::new();
        let rust_field = RustField {
            id: Id {
                original: "field".to_string(),
                renamed: "field".to_string(),
            },
            ty: RustType::Simple {
                id: "str".to_string(),
            },
            has_default: true,
            comments: Default::default(),
            decorators: Default::default(),
        };
        python.write_field(mock_writer, &rust_field, &[]).unwrap();
        assert_eq!(String::from_utf8_lossy(mock_writer), "    field: str\n");
    }

    #[test]
    fn test_non_optional_value_with_no_serde_default() {
        let mut python = Python::default();
        let mock_writer = &mut Vec::new();
        let rust_field = RustField {
            id: Id {
                original: "field".to_string(),
                renamed: "field".to_string(),
            },
            ty: RustType::Simple {
                id: "str".to_string(),
            },
            has_default: false,
            comments: Default::default(),
            decorators: Default::default(),
        };
        python.write_field(mock_writer, &rust_field, &[]).unwrap();
        assert_eq!(String::from_utf8_lossy(mock_writer), "    field: str\n");
    }

    #[test]
    fn simple_test_tagged_enum() {
        let mut python = Python::default();

        let enum_source = r#"
		#[serde(tag = "type", content = "content")]
		enum Test {
            Unit,
            Tuple(i32),
            Struct { field: String },
        }"#;

        // Parse the source into an ItemEnum
        let item_enum: ItemEnum = parse_str(enum_source).unwrap();

        let test_enum = parse_enum(&item_enum, &[]).unwrap();
        // stdout as writer
        let w = &mut std::io::stdout();
        let rust_enum = if let RustItem::Enum(e) = test_enum {
            e
        } else {
            panic!("Expected enum")
        };
        // python
        //     .write_algebraic_enum("test_tag_key", "test_content_key", &shared_enum, w)
        //     .unwrap();
        println!("--------------");
        python.write_enum(w, &rust_enum).unwrap();
    }

    #[test]
    fn test_tagged_enum() {
        let mut python = Python::default();

        let enum_source = r#"        
		#[serde(tag = "type", content = "content")]
		pub enum ItemModification {
            SetTitle {
                new_title: String,
            },
            AddField {
                field_label: String,
                field_value: String,
                field_type: String,
                section_title: Option<String>,
            },
            SetField {
                field_name: String,
                new_value: String,
                section_title: Option<String>,
            },
            RemoveField {
                field_name: String,
                section_title: Option<String>,
            },
        }"#;

        let item_enum: ItemEnum = parse_str(enum_source).unwrap();

        let test_enum = parse_enum(&item_enum, &[]).unwrap();

        let rust_enum = if let RustItem::Enum(e) = test_enum {
            e
        } else {
            panic!("Expected enum")
        };

        let stdout_writer = &mut std::io::stdout();

        // let rust_enum = RustEnum::Algebraic {
        //     tag_key: "test_tag_key".to_string(),
        //     content_key: "test_content_key".to_string(),
        //     shared: shared_enum,
        // };
        python.write_enum(stdout_writer, &rust_enum).unwrap();
    }
}
