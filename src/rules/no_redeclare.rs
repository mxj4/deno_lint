// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::{
  ast::*, utils::find_ids, utils::ident::IdentLike, utils::Id, visit::Node,
  visit::Visit, visit::VisitWith,
};

use std::collections::HashSet;

pub struct NoRedeclare;

impl LintRule for NoRedeclare {
  fn new() -> Box<Self> {
    Box::new(NoRedeclare)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-redeclare"
  }

  fn lint_program(&self, context: &mut Context, program: &Program) {
    let mut visitor = NoRedeclareVisitor {
      context,
      bindings: Default::default(),
    };
    program.visit_with(program, &mut visitor);
  }
}

struct NoRedeclareVisitor<'c> {
  context: &'c mut Context,
  /// TODO(kdy1): Change this to HashMap<Id, Vec<Span>> and use those spans to point previous bindings/
  bindings: HashSet<Id>,
}

impl<'c> NoRedeclareVisitor<'c> {
  fn declare(&mut self, i: &Ident) {
    let id = i.to_id();

    if !self.bindings.insert(id) {
      self.context.add_diagnostic(
        i.span,
        "no-redeclare",
        "Redeclaration is not allowed",
      );
    }
  }
}

impl<'c> Visit for NoRedeclareVisitor<'c> {
  noop_visit_type!();

  fn visit_fn_decl(&mut self, f: &FnDecl, _: &dyn Node) {
    if f.function.body.is_none() {
      return;
    }

    self.declare(&f.ident);

    f.visit_children_with(self);
  }

  fn visit_var_declarator(&mut self, v: &VarDeclarator, _: &dyn Node) {
    let ids: Vec<Ident> = find_ids(&v.name);

    for id in ids {
      self.declare(&id);
    }
  }

  fn visit_param(&mut self, p: &Param, _: &dyn Node) {
    let ids: Vec<Ident> = find_ids(&p.pat);

    for id in ids {
      self.declare(&id);
    }
  }

  fn visit_class_prop(&mut self, p: &ClassProp, _: &dyn Node) {
    if p.computed {
      p.key.visit_with(p, self);
    }

    p.value.visit_with(p, self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_redeclare_valid() {
    assert_lint_ok! {
      NoRedeclare,
      "var a = 3; var b = function() { var a = 10; };",
      "var a = 3; a = 10;",
      "if (true) {\n    let b = 2;\n} else {    \nlet b = 3;\n}",
      "class C {
        constructor(a: string) {}
      }
      class D {
        constructor(a: string) {}
      }",
    };
  }

  #[test]
  fn no_redeclare_invalid() {
    assert_lint_err::<NoRedeclare>("var a = 3; var a = 10;", 15);
    assert_lint_err_on_line::<NoRedeclare>(
      "switch(foo) { case a: var b = 3;\ncase b: var b = 4}",
      2,
      12,
    );
    assert_lint_err::<NoRedeclare>("var a = 3; var a = 10;", 15);
    assert_lint_err::<NoRedeclare>("var a = {}; var a = [];", 16);

    assert_lint_err::<NoRedeclare>("var a; function a() {}", 16);

    assert_lint_err::<NoRedeclare>("function a() {} function a() {}", 25);
    assert_lint_err::<NoRedeclare>(
      "var a = function() { }; var a = function() { }",
      28,
    );

    assert_lint_err::<NoRedeclare>(
      "var a = function() { }; var a = new Date();",
      28,
    );

    assert_lint_err_n::<NoRedeclare>(
      "var a = 3; var a = 10; var a = 15;",
      vec![15, 27],
    );
    assert_lint_err::<NoRedeclare>("var a; var a;", 11);

    assert_lint_err::<NoRedeclare>("export var a; var a;", 18);
    assert_lint_err_on_line_n::<NoRedeclare>(
      "var a; var {a = 0, b: Object = 0} = {};",
      vec![(1, 12)],
    );
    assert_lint_err_on_line_n::<NoRedeclare>(
      "var a; var {a = 0, b: Object = 0} = {};",
      vec![(1, 12)],
    );

    assert_lint_err_on_line_n::<NoRedeclare>(
      "var a; var {a = 0, b: Object = 0} = {};",
      vec![(1, 12)],
    );
    assert_lint_err_on_line_n::<NoRedeclare>(
      "var a; var {a = 0, b: globalThis = 0} = {};",
      vec![(1, 12)],
    );
    assert_lint_err::<NoRedeclare>("function f() { var a; var a; }", 26);

    assert_lint_err::<NoRedeclare>("function f(a) { var a; }", 20);

    assert_lint_err::<NoRedeclare>(
      "function f() { var a; if (test) { var a; } }",
      38,
    );
    assert_lint_err::<NoRedeclare>("for (var a, a;;);", 12);

    assert_lint_err::<NoRedeclare>("let a; let a;", 11);

    assert_lint_err::<NoRedeclare>("let a; const a = 0;", 13);
    assert_lint_err::<NoRedeclare>("let a; const a = 0;", 13);

    assert_lint_err::<NoRedeclare>("const a = 0; const a = 0;", 19);

    assert_lint_err::<NoRedeclare>("if (test) { let a; let a; }", 23);
    assert_lint_err::<NoRedeclare>(
      "switch (test) { case 0: let a; let a; }",
      35,
    );

    assert_lint_err::<NoRedeclare>("for (let a, a;;);", 12);

    assert_lint_err::<NoRedeclare>("for (let [a, a] in xs);", 13);
    assert_lint_err::<NoRedeclare>("for (let [a, a] in xs);", 13);

    assert_lint_err::<NoRedeclare>("function f() { let a; let a; }", 26);

    assert_lint_err::<NoRedeclare>("function f(a) { let a; }", 20);
    assert_lint_err::<NoRedeclare>(
      "function f() { if (test) { let a; let a; } }",
      38,
    );
  }
}
