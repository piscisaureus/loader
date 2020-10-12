use swc_common::{sync::Lrc, FileName, Loc, SourceMap, Span, DUMMY_SP};
use swc_ecmascript::{
  ast,
  parser::{lexer::Lexer, EsConfig, PResult, Parser, StringInput, Syntax},
  visit::{Node, Visit, VisitWith},
};

use serde::Serialize;
use wasm_bindgen::prelude::*;

#[serde(rename_all = "camelCase")]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub enum ImportKind {
  Import,
  Export,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ImportDescriptor {
  pub specifier: String,
  pub kind: ImportKind,
  pub col: usize,
  pub line: usize,
}

#[wasm_bindgen]
pub fn get_imports(source: String) -> String {
  let imports = get_imports_impl(source).ok();
  serde_json::to_string(&imports).unwrap()
}

#[inline(always)]
fn get_imports_impl(source: String) -> PResult<Vec<ImportDescriptor>> {
  let source_map = Lrc::<SourceMap>::default();
  let source_file = source_map.new_source_file(FileName::Anon, source);
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
    None,
  );
  let mut parser = Parser::new_from(lexer);
  let module = parser.parse_module()?;
  let mut visitor = ImportVisitor {
    source_map: &source_map,
    imports: vec![],
  };
  module.visit_with(&ast::Invalid { span: DUMMY_SP }, &mut visitor);
  Ok(visitor.imports)
}

struct ImportVisitor<'a> {
  pub imports: Vec<ImportDescriptor>,
  source_map: &'a SourceMap,
}

impl<'a> ImportVisitor<'a> {
  #[inline(always)]
  fn get_location(&self, span: Span) -> (usize, usize) {
    let Loc { line, col, .. } = self.source_map.lookup_char_pos(span.lo);
    // Swc does not export any public module to convert a CharPos to a number.
    let col: usize = unsafe { std::mem::transmute(col) };
    (line, col)
  }
}

impl<'a> Visit for ImportVisitor<'a> {
  #[inline(always)]
  fn visit_import_decl(&mut self, node: &ast::ImportDecl, _parent: &dyn Node) {
    let specifier = node.src.value.to_string();
    let (line, col) = self.get_location(node.span);
    self.imports.push(ImportDescriptor {
      specifier,
      kind: ImportKind::Import,
      line,
      col,
    });
  }

  #[inline(always)]
  fn visit_named_export(
    &mut self,
    node: &ast::NamedExport,
    _parent: &dyn Node,
  ) {
    if let Some(src) = &node.src {
      let specifier = src.value.to_string();
      let (line, col) = self.get_location(node.span);
      self.imports.push(ImportDescriptor {
        specifier,
        kind: ImportKind::Export,
        col,
        line,
      });
    }
  }

  #[inline(always)]
  fn visit_export_all(&mut self, node: &ast::ExportAll, _parent: &dyn Node) {
    let specifier = node.src.value.to_string();
    let (line, col) = self.get_location(node.span);
    self.imports.push(ImportDescriptor {
      specifier,
      kind: ImportKind::Export,
      line,
      col,
    });
  }

  #[inline(always)]
  fn visit_stmts(&mut self, _imports: &[ast::Stmt], _parent: &dyn Node) {}
}

#[cfg(test)]
mod tests {
  use super::*;
  use wasm_bindgen_test::*;

  wasm_bindgen_test_configure!(run_in_browser);

  #[test]
  #[wasm_bindgen_test]
  fn test_ok() {
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
    let actual = get_imports(source.to_owned());
    let expected = serde_json::to_string(&imports_json).unwrap();
    assert_eq!(actual, expected);
  }

  #[test]
  #[wasm_bindgen_test]
  fn test_err() {
    let json = get_imports("{;".to_owned());
    assert_eq!(&json, "null");
  }
}
