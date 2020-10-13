use anyhow::{anyhow as any_err, Error as AnyErr};
use serde::Serialize;
use std::rc::Rc;
use swc_common::comments::SingleThreadedComments;
use swc_common::{sync::Lrc, FileName, Loc, SourceMap, Span};
use swc_ecmascript::ast;
use swc_ecmascript::codegen::{self, text_writer::JsWriter, Emitter};
use swc_ecmascript::parser::lexer::Lexer;
use swc_ecmascript::parser::{EsConfig, Parser, StringInput, Syntax};
use swc_ecmascript::visit::{VisitMut, VisitMutWith};
use url::Url;
use wasm_bindgen::prelude::*;

#[serde(rename_all = "camelCase")]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub enum ImportKind {
  Import,
  Export,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ImportDescriptor {
  pub src: String,
  pub kind: ImportKind,
  pub col: usize,
  pub line: usize,
}

#[wasm_bindgen]
pub fn get_imports(url: String, source: String) -> String {
  let imports = get_imports_impl(url, source).ok();
  serde_json::to_string(&imports).unwrap()
}

#[inline(always)]
fn get_imports_impl(
  url: String,
  source: String,
) -> Result<Vec<ImportDescriptor>, AnyErr> {
  let mut module;
  let comments;
  let source_map;

  {
    let base_name = Url::parse(&url)?
      .path_segments()
      .ok_or(url::ParseError::RelativeUrlWithCannotBeABaseBase)?
      .nth_back(0)
      .unwrap()
      .to_owned();
    source_map = Lrc::<SourceMap>::default();
    let name = FileName::Custom(url);
    let source_file = source_map.new_source_file(name, source);
    comments = Rc::<SingleThreadedComments>::default();
    let lexer = Lexer::<StringInput<'_>>::new(
      Syntax::Es(EsConfig {
        jsx: true, // TODO: off?
        num_sep: true,
        class_private_props: true,
        class_private_methods: true,
        class_props: true,
        fn_bind: true,
        decorators: true,               // TODO: off?
        decorators_before_export: true, // TODO: off?
        export_default_from: true,
        export_namespace_from: true,
        dynamic_import: true,
        nullish_coalescing: true,
        optional_chaining: true,
        import_meta: true,
        top_level_await: true,
        import_assertions: true,
      }),
      Default::default(),
      (&*source_file).into(),
      Some(&*comments),
    );
    let mut parser = Parser::new_from(lexer);
    module = parser.parse_module().map_err(|e| any_err!("{:?}", e))?;

    let mut visitor = ImportVisitor {
      base_name,
      source_map: source_map.clone(),
      imports: vec![],
    };
    module.visit_mut_with(&mut visitor);
  };

  let mut source2 = Vec::new();
  let mut source_map_mappings = Vec::new();
  Emitter {
    cfg: codegen::Config { minify: false },
    cm: source_map.clone(),
    wr: Box::new(JsWriter::new(
      source_map.clone(),
      "\n",
      &mut source2,
      Some(&mut source_map_mappings),
    )),
    comments: Some(&comments),
  }
  .emit_module(&module)?;

  let mut source2 = String::from_utf8(source2).unwrap();

  let mut source_map_buf = Vec::new();
  source_map
    .build_source_map_from(&mut source_map_mappings, None)
    .to_writer(&mut source_map_buf)?;
  let source_map_string = String::from_utf8(source_map_buf)?;
  eprintln!("{}", &source_map_string);

  source2.push_str("\n//# sourceMappingURL=data:application/json;base64,");
  let base64_config = base64::Config::new(base64::CharacterSet::UrlSafe, true);
  base64::encode_config_buf(&*source_map_string, base64_config, &mut source2);

  println!("SRC:\n{}", &source2);

  Ok(vec![])
}

struct ImportVisitor {
  imports: Vec<ImportDescriptor>,
  base_name: String,
  source_map: Lrc<SourceMap>,
}

impl ImportVisitor {
  #[inline(always)]
  fn get_location(&self, span: Span) -> (usize, usize) {
    let Loc { line, col, .. } = self.source_map.lookup_char_pos(span.lo);
    // Swc does not export any public module to convert a CharPos to a number.
    let col: usize = unsafe { std::mem::transmute(col) };
    (line, col)
  }

  #[inline(always)]
  fn update_import_src(&mut self, src: &mut ast::Str) {
    if src.value.starts_with('#') {
      src.value = format!("./{}{}", self.base_name, &src.value).into();
    }
  }
}

impl VisitMut for ImportVisitor {
  #[inline(always)]
  fn visit_mut_import_decl(&mut self, node: &mut ast::ImportDecl) {
    let src = node.src.value.to_string();
    let (line, col) = self.get_location(node.span);
    self.imports.push(ImportDescriptor {
      src,
      kind: ImportKind::Import,
      line,
      col,
    });
    self.update_import_src(&mut node.src);
  }

  #[inline(always)]
  fn visit_mut_named_export(&mut self, node: &mut ast::NamedExport) {
    if let Some(import_src) = &mut node.src {
      let src = import_src.value.to_string();
      let (line, col) = self.get_location(node.span);
      self.imports.push(ImportDescriptor {
        src,
        kind: ImportKind::Export,
        col,
        line,
      });
      self.update_import_src(import_src);
    }
  }

  #[inline(always)]
  fn visit_mut_export_all(&mut self, node: &mut ast::ExportAll) {
    let src = node.src.value.to_string();
    let (line, col) = self.get_location(node.span);
    self.imports.push(ImportDescriptor {
      src,
      kind: ImportKind::Export,
      line,
      col,
    });
  }

  #[inline(always)]
  fn visit_mut_stmts(&mut self, _imports: &mut Vec<ast::Stmt>) {}
}

#[cfg(test)]
mod tests {
  use super::*;
  use wasm_bindgen_test::*;

  wasm_bindgen_test_configure!(run_in_browser);

  #[test]
  #[wasm_bindgen_test]
  fn test_ok() {
    let url = "http://deno.cool/index.html";
    let source = r#"
import something from "../module_with_default_export.js";
import { something_else } from "ordinary_module";
import * as everything from "some_other_module.js";

import "same_line_1"; import "same_line_2";

import ""; // Empty specifier.

await everything();

import "after_top_level_await";

export * from "complete re-exported module";
export { that } from "partially re-exported module";

import "repeated_import";
import "repeated_import";

/* padding */ import _ from "at column 14";
"#;
    let imports_json = serde_json::json!([
      {
        "specifier": "../module_with_default_export.js",
        "kind": "import",
        "col": 0,
        "line": 2
      },
      {
        "specifier": "ordinary_module",
        "kind": "import",
        "col": 0,
        "line": 3
      },
      {
        "specifier": "some_other_module.js",
        "kind": "import",
        "col": 0,
        "line": 4
      },
      {
        "specifier": "same_line_1",
        "kind": "import",
        "col": 0,
        "line": 6
      },
      {
        "specifier": "same_line_2",
        "kind": "import",
        "col": 22,
        "line": 6
      },
      {
        "specifier": "",
        "kind": "import",
        "col": 0,
        "line": 8
      },
      {
        "specifier": "after_top_level_await",
        "kind": "import",
        "col": 0,
        "line": 12
      },
      {
        "specifier": "complete re-exported module",
        "kind": "export",
        "col": 0,
        "line": 14
      },
      {
        "specifier": "partially re-exported module",
        "kind": "export",
        "col": 0,
        "line": 15
      },
      {
        "specifier": "repeated_import",
        "kind": "import",
        "col": 0,
        "line": 17
      },
      {
        "specifier": "repeated_import",
        "kind": "import",
        "col": 0,
        "line": 18
      },
      {
        "specifier": "at column 14",
        "kind": "import",
        "col": 14,
        "line": 20
      }
    ]);
    let actual = get_imports(url.to_owned(), source.to_owned());
    let expected = serde_json::to_string(&imports_json).unwrap();
    assert_eq!(actual, expected);
  }

  #[test]
  #[wasm_bindgen_test]
  fn test_err() {
    let json = get_imports("http://yeah.deno/".to_owned(), "{;".to_owned());
    assert_eq!(&json, "null");
  }

  #[test]
  fn test_x() {
    let url = "http://deno.cool/index.html";
    let source = r##"
import something from "#../module_with_default_export.js";
import { something_else } from "#ordinary_module";
import * as everything from "#.js";

import "#same_line_1"; import "#same_line_2";

import ""; // Empty specifier.

await everything();

import "#after_top_level_await";

export * from "#complete re-exported module";
export { that } from "#partially re-exported module";

import "#repeated_import";
import "#repeated_import";

/* padding */ import _ from "#at column 14"; // Foo!
"##;
    let _ = get_imports(url.to_owned(), source.to_owned());
  }
}
