//! Visitor pattern for traversing the AST.
//!
//! This module provides a visitor trait that can be implemented to perform
//! operations on the AST, such as type checking, name resolution, or code generation.

use super::*;
use crate::expr::*;
use crate::pat::*;
use crate::stmt::*;
use crate::ty::*;

/// A visitor for traversing the AST.
///
/// Implement this trait to perform custom operations on AST nodes.
/// The default implementation recursively visits child nodes.
pub trait Visitor: Sized {
    /// Visit a module.
    fn visit_module(&mut self, module: &Module) {
        walk_module(self, module);
    }

    /// Visit an item.
    fn visit_item(&mut self, item: &Item) {
        walk_item(self, item);
    }

    /// Visit an expression.
    fn visit_expr(&mut self, expr: &Expr) {
        walk_expr(self, expr);
    }

    /// Visit a statement.
    fn visit_stmt(&mut self, stmt: &Stmt) {
        walk_stmt(self, stmt);
    }

    /// Visit a pattern.
    fn visit_pattern(&mut self, pattern: &Pattern) {
        walk_pattern(self, pattern);
    }

    /// Visit a type.
    fn visit_type(&mut self, ty: &Type) {
        walk_type(self, ty);
    }

    /// Visit a block.
    fn visit_block(&mut self, block: &Block) {
        walk_block(self, block);
    }

    /// Visit a path.
    fn visit_path(&mut self, path: &Path) {
        walk_path(self, path);
    }

    /// Visit an identifier.
    fn visit_ident(&mut self, _ident: &Ident) {}

    /// Visit a function.
    fn visit_function(&mut self, func: &Function) {
        walk_function(self, func);
    }

    /// Visit a struct declaration.
    fn visit_struct(&mut self, strukt: &StructDecl) {
        walk_struct(self, strukt);
    }

    /// Visit an enum declaration.
    fn visit_enum(&mut self, enm: &EnumDecl) {
        walk_enum(self, enm);
    }

    /// Visit a trait declaration.
    fn visit_trait(&mut self, trt: &TraitDecl) {
        walk_trait(self, trt);
    }

    /// Visit an impl block.
    fn visit_impl(&mut self, impl_decl: &ImplDecl) {
        walk_impl(self, impl_decl);
    }
}

/// Walk a module, visiting all items.
pub fn walk_module<V: Visitor>(visitor: &mut V, module: &Module) {
    for item in &module.items {
        visitor.visit_item(item);
    }
}

/// Walk an item.
pub fn walk_item<V: Visitor>(visitor: &mut V, item: &Item) {
    match &item.kind {
        ItemKind::Function(func) => visitor.visit_function(func),
        ItemKind::Struct(strukt) => visitor.visit_struct(strukt),
        ItemKind::Enum(enm) => visitor.visit_enum(enm),
        ItemKind::Trait(trt) => visitor.visit_trait(trt),
        ItemKind::Impl(impl_decl) => visitor.visit_impl(impl_decl),
        ItemKind::TypeAlias(alias) => {
            visitor.visit_ident(&alias.name);
            visitor.visit_type(&alias.ty);
        }
        ItemKind::Const(const_decl) => {
            visitor.visit_ident(&const_decl.name);
            if let Some(ty) = &const_decl.ty {
                visitor.visit_type(ty);
            }
            visitor.visit_expr(&const_decl.value);
        }
        ItemKind::Static(static_decl) => {
            visitor.visit_ident(&static_decl.name);
            visitor.visit_type(&static_decl.ty);
            if let Some(value) = &static_decl.value {
                visitor.visit_expr(value);
            }
        }
        ItemKind::Import(import) => {
            walk_import_tree(visitor, &import.tree);
        }
        ItemKind::Export(_) => {}
        ItemKind::Extern(extern_block) => {
            for item in &extern_block.items {
                match &item.kind {
                    ExternItemKind::Function(sig) => {
                        visitor.visit_ident(&sig.name);
                        for param in &sig.params {
                            if let Some(ty) = &param.ty {
                                visitor.visit_type(ty);
                            }
                        }
                        if let Some(ret) = &sig.return_type {
                            visitor.visit_type(ret);
                        }
                    }
                    ExternItemKind::Static { name, ty, .. } => {
                        visitor.visit_ident(name);
                        visitor.visit_type(ty);
                    }
                    ExternItemKind::Type(name) => {
                        visitor.visit_ident(name);
                    }
                }
            }
        }
        ItemKind::Module(module_decl) => {
            visitor.visit_ident(&module_decl.name);
            if let Some(items) = &module_decl.content {
                for item in items {
                    visitor.visit_item(item);
                }
            }
        }
        ItemKind::Macro(_) => {}
    }
}

/// Walk a function.
pub fn walk_function<V: Visitor>(visitor: &mut V, func: &Function) {
    visitor.visit_ident(&func.name);
    for param in &func.params {
        visitor.visit_pattern(&param.pattern);
        if let Some(ty) = &param.ty {
            visitor.visit_type(ty);
        }
        if let Some(default) = &param.default {
            visitor.visit_expr(default);
        }
    }
    if let Some(ret_ty) = &func.return_type {
        visitor.visit_type(ret_ty);
    }
    if let Some(body) = &func.body {
        visitor.visit_block(body);
    }
}

/// Walk a struct declaration.
pub fn walk_struct<V: Visitor>(visitor: &mut V, strukt: &StructDecl) {
    visitor.visit_ident(&strukt.name);
    match &strukt.kind {
        StructKind::Named(fields) => {
            for field in fields {
                visitor.visit_ident(&field.name);
                visitor.visit_type(&field.ty);
                if let Some(default) = &field.default {
                    visitor.visit_expr(default);
                }
            }
        }
        StructKind::Tuple(fields) => {
            for field in fields {
                visitor.visit_type(&field.ty);
            }
        }
        StructKind::Unit => {}
    }
}

/// Walk an enum declaration.
pub fn walk_enum<V: Visitor>(visitor: &mut V, enm: &EnumDecl) {
    visitor.visit_ident(&enm.name);
    for variant in &enm.variants {
        visitor.visit_ident(&variant.name);
        match &variant.kind {
            VariantKind::Unit => {}
            VariantKind::Tuple(fields) => {
                for field in fields {
                    visitor.visit_type(&field.ty);
                }
            }
            VariantKind::Struct(fields) => {
                for field in fields {
                    visitor.visit_ident(&field.name);
                    visitor.visit_type(&field.ty);
                }
            }
        }
        if let Some(disc) = &variant.discriminant {
            visitor.visit_expr(disc);
        }
    }
}

/// Walk a trait declaration.
pub fn walk_trait<V: Visitor>(visitor: &mut V, trt: &TraitDecl) {
    visitor.visit_ident(&trt.name);
    for item in &trt.items {
        match &item.kind {
            TraitItemKind::Method { sig, body } => {
                visitor.visit_ident(&sig.name);
                for param in &sig.params {
                    if let Some(ty) = &param.ty {
                        visitor.visit_type(ty);
                    }
                }
                if let Some(ret) = &sig.return_type {
                    visitor.visit_type(ret);
                }
                if let Some(body) = body {
                    visitor.visit_block(body);
                }
            }
            TraitItemKind::Type { name, default, .. } => {
                visitor.visit_ident(name);
                if let Some(ty) = default {
                    visitor.visit_type(ty);
                }
            }
            TraitItemKind::Const { name, ty, default } => {
                visitor.visit_ident(name);
                visitor.visit_type(ty);
                if let Some(expr) = default {
                    visitor.visit_expr(expr);
                }
            }
        }
    }
}

/// Walk an impl block.
pub fn walk_impl<V: Visitor>(visitor: &mut V, impl_decl: &ImplDecl) {
    if let Some(trait_ref) = &impl_decl.trait_ref {
        visitor.visit_path(trait_ref);
    }
    visitor.visit_type(&impl_decl.self_ty);
    for item in &impl_decl.items {
        match &item.kind {
            ImplItemKind::Method(func) => visitor.visit_function(func),
            ImplItemKind::Type { name, ty } => {
                visitor.visit_ident(name);
                visitor.visit_type(ty);
            }
            ImplItemKind::Const { name, ty, value } => {
                visitor.visit_ident(name);
                visitor.visit_type(ty);
                visitor.visit_expr(value);
            }
        }
    }
}

/// Walk an expression.
pub fn walk_expr<V: Visitor>(visitor: &mut V, expr: &Expr) {
    match &expr.kind {
        ExprKind::Literal(_) => {}
        ExprKind::Path(path) => visitor.visit_path(path),
        ExprKind::Binary { left, right, .. } => {
            visitor.visit_expr(left);
            visitor.visit_expr(right);
        }
        ExprKind::Unary { operand, .. } => visitor.visit_expr(operand),
        ExprKind::Assign { target, value, .. } => {
            visitor.visit_expr(target);
            visitor.visit_expr(value);
        }
        ExprKind::Call { func, args } => {
            visitor.visit_expr(func);
            for arg in args {
                visitor.visit_expr(&arg.value);
            }
        }
        ExprKind::MethodCall {
            receiver,
            method,
            args,
            ..
        } => {
            visitor.visit_expr(receiver);
            visitor.visit_ident(method);
            for arg in args {
                visitor.visit_expr(&arg.value);
            }
        }
        ExprKind::Field { object, field } => {
            visitor.visit_expr(object);
            visitor.visit_ident(field);
        }
        ExprKind::TupleIndex { object, .. } => visitor.visit_expr(object),
        ExprKind::Index { object, index } => {
            visitor.visit_expr(object);
            visitor.visit_expr(index);
        }
        ExprKind::Range { start, end, .. } => {
            if let Some(s) = start {
                visitor.visit_expr(s);
            }
            if let Some(e) = end {
                visitor.visit_expr(e);
            }
        }
        ExprKind::Tuple(exprs) | ExprKind::Array(exprs) | ExprKind::Set(exprs) => {
            for e in exprs {
                visitor.visit_expr(e);
            }
        }
        ExprKind::ArrayRepeat { value, count } => {
            visitor.visit_expr(value);
            visitor.visit_expr(count);
        }
        ExprKind::Dict(pairs) => {
            for (k, v) in pairs {
                visitor.visit_expr(k);
                visitor.visit_expr(v);
            }
        }
        ExprKind::ListComp {
            element,
            comprehensions,
        }
        | ExprKind::SetComp {
            element,
            comprehensions,
        }
        | ExprKind::Generator {
            element,
            comprehensions,
        } => {
            visitor.visit_expr(element);
            for comp in comprehensions {
                visitor.visit_pattern(&comp.pattern);
                visitor.visit_expr(&comp.iter);
                for filter in &comp.filters {
                    visitor.visit_expr(filter);
                }
            }
        }
        ExprKind::DictComp {
            key,
            value,
            comprehensions,
        } => {
            visitor.visit_expr(key);
            visitor.visit_expr(value);
            for comp in comprehensions {
                visitor.visit_pattern(&comp.pattern);
                visitor.visit_expr(&comp.iter);
                for filter in &comp.filters {
                    visitor.visit_expr(filter);
                }
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
        } => {
            visitor.visit_expr(condition);
            visitor.visit_expr(then_branch);
            for (cond, branch) in elif_branches {
                visitor.visit_expr(cond);
                visitor.visit_expr(branch);
            }
            if let Some(else_expr) = else_branch {
                visitor.visit_expr(else_expr);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            visitor.visit_expr(scrutinee);
            for arm in arms {
                visitor.visit_pattern(&arm.pattern);
                if let Some(guard) = &arm.guard {
                    visitor.visit_expr(guard);
                }
                visitor.visit_expr(&arm.body);
            }
        }
        ExprKind::Block(block) => visitor.visit_block(block),
        ExprKind::Lambda { params, body, .. } => {
            for param in params {
                visitor.visit_pattern(&param.pattern);
                if let Some(ty) = &param.ty {
                    visitor.visit_type(ty);
                }
            }
            visitor.visit_expr(body);
        }
        ExprKind::Return(Some(e))
        | ExprKind::Break(Some(e))
        | ExprKind::Yield(Some(e))
        | ExprKind::Await(e)
        | ExprKind::Spawn(e)
        | ExprKind::Paren(e)
        | ExprKind::Move(e)
        | ExprKind::Clone(e)
        | ExprKind::Copy(e)
        | ExprKind::Box(e) => visitor.visit_expr(e),
        ExprKind::Return(None) | ExprKind::Break(None) | ExprKind::Yield(None) => {}
        ExprKind::Continue => {}
        ExprKind::Async(block) | ExprKind::Comptime(block) | ExprKind::Unsafe(block) => {
            visitor.visit_block(block)
        }
        ExprKind::Try {
            body,
            catch_clauses,
            finally_block,
        } => {
            visitor.visit_block(body);
            for clause in catch_clauses {
                if let Some(pat) = &clause.pattern {
                    visitor.visit_pattern(pat);
                }
                visitor.visit_block(&clause.body);
            }
            if let Some(finally) = finally_block {
                visitor.visit_block(finally);
            }
        }
        ExprKind::Cast { expr, ty } | ExprKind::Type { expr, ty } => {
            visitor.visit_expr(expr);
            visitor.visit_type(ty);
        }
        ExprKind::Struct { path, fields, base } => {
            visitor.visit_path(path);
            for field in fields {
                visitor.visit_ident(&field.name);
                if let Some(value) = &field.value {
                    visitor.visit_expr(value);
                }
            }
            if let Some(base_expr) = base {
                visitor.visit_expr(base_expr);
            }
        }
        ExprKind::TupleStruct { path, fields } => {
            visitor.visit_path(path);
            for field in fields {
                visitor.visit_expr(field);
            }
        }
        ExprKind::UnitStruct(path) => visitor.visit_path(path),
        ExprKind::Macro { path, args } => {
            visitor.visit_path(path);
            for arg in args {
                visitor.visit_expr(arg);
            }
        }
        ExprKind::Error => {}
    }
}

/// Walk a statement.
pub fn walk_stmt<V: Visitor>(visitor: &mut V, stmt: &Stmt) {
    match &stmt.kind {
        StmtKind::Expr(expr) => visitor.visit_expr(expr),
        StmtKind::Let {
            pattern, ty, init, ..
        } => {
            visitor.visit_pattern(pattern);
            if let Some(ty) = ty {
                visitor.visit_type(ty);
            }
            if let Some(init) = init {
                visitor.visit_expr(init);
            }
        }
        StmtKind::Var { pattern, ty, init } => {
            visitor.visit_pattern(pattern);
            if let Some(ty) = ty {
                visitor.visit_type(ty);
            }
            if let Some(init) = init {
                visitor.visit_expr(init);
            }
        }
        StmtKind::Const { name, ty, value } => {
            visitor.visit_ident(name);
            if let Some(ty) = ty {
                visitor.visit_type(ty);
            }
            visitor.visit_expr(value);
        }
        StmtKind::Assign { target, value } => {
            visitor.visit_expr(target);
            visitor.visit_expr(value);
        }
        StmtKind::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
        } => {
            visitor.visit_expr(condition);
            visitor.visit_block(then_branch);
            for (cond, block) in elif_branches {
                visitor.visit_expr(cond);
                visitor.visit_block(block);
            }
            if let Some(else_block) = else_branch {
                visitor.visit_block(else_block);
            }
        }
        StmtKind::Match { scrutinee, arms } => {
            visitor.visit_expr(scrutinee);
            for arm in arms {
                visitor.visit_pattern(&arm.pattern);
                if let Some(guard) = &arm.guard {
                    visitor.visit_expr(guard);
                }
                visitor.visit_expr(&arm.body);
            }
        }
        StmtKind::While { condition, body } => {
            visitor.visit_expr(condition);
            visitor.visit_block(body);
        }
        StmtKind::For { pattern, iter, body } => {
            visitor.visit_pattern(pattern);
            visitor.visit_expr(iter);
            visitor.visit_block(body);
        }
        StmtKind::Loop { body } => visitor.visit_block(body),
        StmtKind::Break(Some(e)) | StmtKind::Return(Some(e)) | StmtKind::Yield(Some(e)) => {
            visitor.visit_expr(e)
        }
        StmtKind::Break(None) | StmtKind::Return(None) | StmtKind::Yield(None) => {}
        StmtKind::Continue => {}
        StmtKind::Defer(stmt) => visitor.visit_stmt(stmt),
        StmtKind::Drop(expr) => visitor.visit_expr(expr),
        StmtKind::Assert { condition, message } => {
            visitor.visit_expr(condition);
            if let Some(msg) = message {
                visitor.visit_expr(msg);
            }
        }
        StmtKind::Try {
            body,
            catch_clauses,
            finally_block,
        } => {
            visitor.visit_block(body);
            for clause in catch_clauses {
                if let Some(pat) = &clause.pattern {
                    visitor.visit_pattern(pat);
                }
                visitor.visit_block(&clause.body);
            }
            if let Some(finally) = finally_block {
                visitor.visit_block(finally);
            }
        }
        StmtKind::Raise(Some(expr)) => visitor.visit_expr(expr),
        StmtKind::Raise(None) => {}
        StmtKind::With { items, body } => {
            for item in items {
                visitor.visit_expr(&item.context);
                if let Some(binding) = &item.binding {
                    visitor.visit_pattern(binding);
                }
            }
            visitor.visit_block(body);
        }
        StmtKind::Async(block) | StmtKind::Unsafe(block) | StmtKind::Comptime(block) => {
            visitor.visit_block(block)
        }
        StmtKind::Spawn(expr) => visitor.visit_expr(expr),
        StmtKind::Select { arms } => {
            for arm in arms {
                match &arm.kind {
                    SelectArmKind::Recv { pattern, channel } => {
                        visitor.visit_pattern(pattern);
                        visitor.visit_expr(channel);
                    }
                    SelectArmKind::Send { value, channel } => {
                        visitor.visit_expr(value);
                        visitor.visit_expr(channel);
                    }
                    SelectArmKind::Default => {}
                }
                visitor.visit_block(&arm.body);
            }
        }
        StmtKind::Item(item) => visitor.visit_item(item),
        StmtKind::Empty => {}
    }
}

/// Walk a pattern.
pub fn walk_pattern<V: Visitor>(visitor: &mut V, pattern: &Pattern) {
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Rest => {}
        PatternKind::Ident {
            name, subpattern, ..
        } => {
            visitor.visit_ident(name);
            if let Some(sub) = subpattern {
                visitor.visit_pattern(sub);
            }
        }
        PatternKind::Literal(_) => {}
        PatternKind::Range { start, end, .. } => {
            visitor.visit_pattern(start);
            visitor.visit_pattern(end);
        }
        PatternKind::Tuple(patterns) | PatternKind::Array(patterns) | PatternKind::Or(patterns) => {
            for pat in patterns {
                visitor.visit_pattern(pat);
            }
        }
        PatternKind::Struct {
            path,
            fields,
            ignore_rest: _,
        } => {
            visitor.visit_path(path);
            for field in fields {
                visitor.visit_ident(&field.name);
                if let Some(pat) = &field.pattern {
                    visitor.visit_pattern(pat);
                }
            }
        }
        PatternKind::TupleStruct { path, elements } => {
            visitor.visit_path(path);
            for elem in elements {
                visitor.visit_pattern(elem);
            }
        }
        PatternKind::UnitStruct(path) | PatternKind::Path(path) => visitor.visit_path(path),
        PatternKind::Paren(pat) | PatternKind::Box(pat) => visitor.visit_pattern(pat),
        PatternKind::Ref { pattern, .. } => visitor.visit_pattern(pattern),
        PatternKind::Type { pattern, ty } => {
            visitor.visit_pattern(pattern);
            visitor.visit_type(ty);
        }
        PatternKind::Macro { path, args } => {
            visitor.visit_path(path);
            for arg in args {
                visitor.visit_pattern(arg);
            }
        }
        PatternKind::Guard { pattern, condition } => {
            visitor.visit_pattern(pattern);
            visitor.visit_expr(condition);
        }
        PatternKind::Error => {}
    }
}

/// Walk a type.
pub fn walk_type<V: Visitor>(visitor: &mut V, ty: &Type) {
    match &ty.kind {
        TypeKind::Path(path) => visitor.visit_path(path),
        TypeKind::Tuple(types) | TypeKind::Union(types) | TypeKind::Intersection(types) => {
            for t in types {
                visitor.visit_type(t);
            }
        }
        TypeKind::Array { element, size } => {
            visitor.visit_type(element);
            visitor.visit_expr(size);
        }
        TypeKind::Slice(inner)
        | TypeKind::Paren(inner)
        | TypeKind::Linear(inner)
        | TypeKind::Pointer { inner, .. }
        | TypeKind::Ref { inner, .. } => visitor.visit_type(inner),
        TypeKind::Function {
            params,
            return_type,
            ..
        } => {
            for param in params {
                visitor.visit_type(param);
            }
            visitor.visit_type(return_type);
        }
        TypeKind::TraitObject { .. } | TypeKind::ImplTrait(_) => {}
        TypeKind::Typeof(expr) => visitor.visit_expr(expr),
        TypeKind::Refinement { base, .. } => visitor.visit_type(base),
        TypeKind::Effect { base, .. } => visitor.visit_type(base),
        TypeKind::Associated { base, ident } => {
            visitor.visit_type(base);
            visitor.visit_ident(ident);
        }
        TypeKind::Opaque { name, .. } => visitor.visit_ident(name),
        TypeKind::Macro { path, args } => {
            visitor.visit_path(path);
            for arg in args {
                visitor.visit_type(arg);
            }
        }
        TypeKind::Var(ident) => visitor.visit_ident(ident),
        TypeKind::Never | TypeKind::Infer | TypeKind::Error => {}
    }
}

/// Walk a block.
pub fn walk_block<V: Visitor>(visitor: &mut V, block: &Block) {
    for stmt in &block.stmts {
        visitor.visit_stmt(stmt);
    }
    if let Some(expr) = &block.expr {
        visitor.visit_expr(expr);
    }
}

/// Walk a path.
pub fn walk_path<V: Visitor>(visitor: &mut V, path: &Path) {
    for segment in &path.segments {
        visitor.visit_ident(&segment.ident);
        if let Some(args) = &segment.args {
            for ty in &args.types {
                visitor.visit_type(ty);
            }
            for expr in &args.consts {
                visitor.visit_expr(expr);
            }
        }
    }
}

/// Walk an import tree.
fn walk_import_tree<V: Visitor>(visitor: &mut V, tree: &ImportTree) {
    match tree {
        ImportTree::Simple { path, alias } => {
            visitor.visit_path(path);
            if let Some(ident) = alias {
                visitor.visit_ident(ident);
            }
        }
        ImportTree::Glob(path) => visitor.visit_path(path),
        ImportTree::Nested { prefix, trees } => {
            visitor.visit_path(prefix);
            for tree in trees {
                walk_import_tree(visitor, tree);
            }
        }
    }
}

/// A mutable visitor for modifying the AST.
pub trait VisitorMut: Sized {
    /// Visit a module mutably.
    fn visit_module_mut(&mut self, module: &mut Module) {
        walk_module_mut(self, module);
    }

    /// Visit an expression mutably.
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        walk_expr_mut(self, expr);
    }

    /// Visit a statement mutably.
    fn visit_stmt_mut(&mut self, stmt: &mut Stmt) {
        walk_stmt_mut(self, stmt);
    }

    /// Visit a pattern mutably.
    fn visit_pattern_mut(&mut self, pattern: &mut Pattern) {
        walk_pattern_mut(self, pattern);
    }

    /// Visit a type mutably.
    fn visit_type_mut(&mut self, ty: &mut Type) {
        walk_type_mut(self, ty);
    }
}

/// Walk a module mutably.
pub fn walk_module_mut<V: VisitorMut>(visitor: &mut V, module: &mut Module) {
    for item in &mut module.items {
        match &mut item.kind {
            ItemKind::Function(func) => {
                if let Some(body) = &mut func.body {
                    for stmt in &mut body.stmts {
                        visitor.visit_stmt_mut(stmt);
                    }
                    if let Some(expr) = &mut body.expr {
                        visitor.visit_expr_mut(expr);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Walk an expression mutably.
pub fn walk_expr_mut<V: VisitorMut>(visitor: &mut V, expr: &mut Expr) {
    match &mut expr.kind {
        ExprKind::Binary { left, right, .. } => {
            visitor.visit_expr_mut(left);
            visitor.visit_expr_mut(right);
        }
        ExprKind::Unary { operand, .. } => visitor.visit_expr_mut(operand),
        ExprKind::Block(block) => {
            for stmt in &mut block.stmts {
                visitor.visit_stmt_mut(stmt);
            }
            if let Some(e) = &mut block.expr {
                visitor.visit_expr_mut(e);
            }
        }
        _ => {}
    }
}

/// Walk a statement mutably.
pub fn walk_stmt_mut<V: VisitorMut>(visitor: &mut V, stmt: &mut Stmt) {
    match &mut stmt.kind {
        StmtKind::Expr(expr) => visitor.visit_expr_mut(expr),
        StmtKind::Let { init, .. } => {
            if let Some(e) = init {
                visitor.visit_expr_mut(e);
            }
        }
        _ => {}
    }
}

/// Walk a pattern mutably (placeholder).
pub fn walk_pattern_mut<V: VisitorMut>(_visitor: &mut V, _pattern: &mut Pattern) {}

/// Walk a type mutably (placeholder).
pub fn walk_type_mut<V: VisitorMut>(_visitor: &mut V, _ty: &mut Type) {}
