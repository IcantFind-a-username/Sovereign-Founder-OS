//! Source-closure gate machinery (v1) for the vault-v2 engine crate.
//!
//! `run_gate` starts from the build script and every explicit Cargo target,
//! parses the complete recursive closure of inline and external modules with
//! `syn`, and returns every violation it finds instead of stopping at the
//! first. What v1 proves over that closure:
//!
//! - **Completeness.** Every `.rs` file under the crate directory is either a
//!   configured root, reached through ordinary `mod` resolution, or reached
//!   through an explicitly admitted `include!` edge — anything else is an
//!   orphan. `#[path]` is rejected outright (it is not in the closed attribute
//!   allowlist), as are symlinks, escapes outside the crate directory,
//!   ambiguous module candidates (`x.rs` and `x/mod.rs` both present), and
//!   missing module files. `cfg`-disabled modules are resolved and scanned as
//!   syntax like any other.
//! - **No project-authored unsafety outside the declared FFI boundary.**
//!   `unsafe` blocks/functions/impls/traits, bare-`fn` unsafety, and
//!   `extern` blocks are rejected in every file not listed in
//!   `GateConfig::ffi_boundary_files`; `extern crate` is rejected everywhere.
//! - **Closed macro and attribute surface.** Project macro definitions are
//!   forbidden; every invocation, attribute, and derive must be on a closed
//!   allowlist; and every macro/attribute token tree is walked structurally
//!   through `proc_macro2::TokenTree` (nested groups included), rejecting
//!   denied identifiers (`unsafe`, `extern`, `include`, …), raw identifiers,
//!   and `#[…]` attribute forms smuggled inside macro tokens. Identifier
//!   classification inspects only the individual `Ident` value — never
//!   serialized token-stream text.
//!
//! The gate also proves exactly two production unsafe FFI entry points plus the
//! single `cfg(test)` OpenSSL LOAD_CONFIG negative control (RFC 0005 / Program 1A
//! plan lines 540-546). The five `tests/ui/` compile-fail fixtures are admitted
//! as auxiliary roots, not module-closure members.

use crate::ffi_entry::{
    admitted_extern_symbol, admitted_production_entry_function,
    admitted_test_only_entry_function, attr_is_cfg_test, path_tail_is_forbidden,
    use_tree_forbidden_root, use_tree_has_forbidden_glob, FORBIDDEN_FFI_SYMBOLS,
    FORBIDDEN_USE_ROOTS,
};
use proc_macro2::{Delimiter, TokenTree};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use syn::visit::{self, Visit};

pub struct GateConfig<'a> {
    /// Compilation roots relative to the crate directory: the build script and
    /// every explicit Cargo target. Nothing is discovered implicitly.
    pub roots: &'a [&'a str],
    /// `(containing file, literal argument)` pairs that `include!` may use.
    /// Any other `include!`, and every `include_str!`/`include_bytes!`, is a
    /// violation.
    pub admitted_includes: &'a [(&'a str, &'a str)],
    /// Files (relative paths) admitted to contain `unsafe` and `extern`
    /// blocks. Empty until the queued FFI item lands `src/engine/ffi.rs` and
    /// `src/engine/process.rs`; entries beyond those two need review.
    pub ffi_boundary_files: &'a [&'a str],
    /// Macro invocations allowed, by single-segment name.
    pub allowed_macros: &'a [&'a str],
    /// Attributes allowed, by path (single segment).
    pub allowed_attributes: &'a [&'a str],
    /// Derive names allowed inside `#[derive(...)]`.
    pub allowed_derives: &'a [&'a str],
    /// Standalone `.rs` files that are not reached through `mod` resolution but
    /// are explicitly admitted (for example trybuild UI fixtures).
    pub auxiliary_roots: &'a [&'a str],
}

pub struct GateOutcome {
    /// Every file the closure reached, as sorted crate-relative paths.
    pub closure: BTreeSet<String>,
    /// Human-readable violations, empty when the crate passes.
    pub violations: Vec<String>,
}

/// Identifiers that may never appear as a token inside macro or attribute
/// argument token trees. String and byte literals are inert and not scanned;
/// these are exact matches against individual `Ident` values only.
const DENIED_TOKEN_IDENTS: &[&str] = &[
    "unsafe",
    "extern",
    "include",
    "include_str",
    "include_bytes",
    "macro_rules",
    "asm",
    "global_asm",
    "naked_asm",
    "no_mangle",
    "link_name",
    "link_section",
    "export_name",
];

/// Additional identifiers rejected when they open a `#[…]` attribute form
/// found inside a macro token stream (attribute smuggling).
const DENIED_SMUGGLED_ATTRIBUTES: &[&str] = &["path"];

pub fn run_gate(crate_dir: &Path, config: &GateConfig) -> GateOutcome {
    let mut outcome = GateOutcome {
        closure: BTreeSet::new(),
        violations: Vec::new(),
    };
    let crate_root = match crate_dir.canonicalize() {
        Ok(root) => root,
        Err(error) => {
            outcome
                .violations
                .push(format!("crate directory {}: {error}", crate_dir.display()));
            return outcome;
        }
    };
    let mut gate = Gate {
        crate_root,
        config,
        outcome: &mut outcome,
        queue: Vec::new(),
    };
    for root in config.roots {
        let module_dir = match Path::new(root).parent() {
            Some(parent) => parent.to_path_buf(),
            None => PathBuf::new(),
        };
        gate.schedule((*root).to_string(), module_dir);
    }
    while let Some((relative, module_dir)) = gate.queue.pop() {
        gate.process_file(&relative, &module_dir);
    }
    gate.reject_orphans_and_symlinks();
    outcome
}

struct Gate<'a, 'b> {
    crate_root: PathBuf,
    config: &'a GateConfig<'a>,
    outcome: &'b mut GateOutcome,
    /// Pending `(relative path, module directory)` pairs. The module directory
    /// is where `mod child;` declared in that file resolves.
    queue: Vec<(String, PathBuf)>,
}

impl Gate<'_, '_> {
    fn schedule(&mut self, relative: String, module_dir: PathBuf) {
        // First visit wins; a repeat edge to the same file (for example
        // `build_gate.rs`, included by two roots) is membership, not a cycle,
        // and an actual include cycle terminates here instead of recursing.
        if self.outcome.closure.insert(relative.clone()) {
            self.queue.push((relative, module_dir));
        }
    }

    fn violation(&mut self, file: &str, message: &str) {
        self.outcome.violations.push(format!("{file}: {message}"));
    }

    fn process_file(&mut self, relative: &str, module_dir: &Path) {
        let absolute = self.crate_root.join(relative);
        match std::fs::symlink_metadata(&absolute) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    self.violation(relative, "is a symlink; the closure admits real files only");
                    return;
                }
            }
            Err(error) => {
                self.violation(relative, &format!("cannot stat: {error}"));
                return;
            }
        }
        match absolute.canonicalize() {
            Ok(canonical) => {
                if !canonical.starts_with(&self.crate_root) {
                    self.violation(relative, "escapes the crate directory");
                    return;
                }
            }
            Err(error) => {
                self.violation(relative, &format!("cannot canonicalize: {error}"));
                return;
            }
        }
        let source = match std::fs::read_to_string(&absolute) {
            Ok(source) => source,
            Err(error) => {
                self.violation(relative, &format!("cannot read: {error}"));
                return;
            }
        };
        let ast = match syn::parse_file(&source) {
            Ok(ast) => ast,
            Err(error) => {
                self.violation(relative, &format!("does not parse: {error}"));
                return;
            }
        };
        let in_boundary = self.config.ffi_boundary_files.contains(&relative);
        let mut scanner = FileScanner {
            gate: self,
            file: relative.to_string(),
            module_dir: module_dir.to_path_buf(),
            inline_modules: Vec::new(),
            in_boundary,
            cfg_test_depth: 0,
            ffi_entry_depth: 0,
        };
        scanner.visit_file(&ast);
    }

    fn reject_orphans_and_symlinks(&mut self) {
        let root = self.crate_root.clone();
        self.walk_directory(&root);
    }

    fn walk_directory(&mut self, directory: &Path) {
        let directory_relative = relative_display(&self.crate_root, directory);
        let entries = match std::fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) => {
                self.violation(&directory_relative, &format!("cannot list: {error}"));
                return;
            }
        };
        for entry in entries {
            let path = match entry {
                Ok(entry) => entry.path(),
                Err(error) => {
                    self.violation(&directory_relative, &format!("cannot list entry: {error}"));
                    continue;
                }
            };
            let relative = relative_display(&self.crate_root, &path);
            let metadata = match std::fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    self.violation(&relative, &format!("cannot stat: {error}"));
                    continue;
                }
            };
            if metadata.file_type().is_symlink() {
                self.violation(
                    &relative,
                    "is a symlink; the crate tree admits real entries only",
                );
                continue;
            }
            if metadata.is_dir() {
                if path
                    .file_name()
                    .is_some_and(|name| name == "target" || name == ".git")
                {
                    continue;
                }
                self.walk_directory(&path);
            } else if path.extension().is_some_and(|extension| extension == "rs")
                && !self.outcome.closure.contains(&relative)
                && !self
                    .config
                    .auxiliary_roots
                    .contains(&relative.as_str())
            {
                self.violation(
                    &relative,
                    "orphan: not a configured root, not reached by module resolution, \
                     not an admitted include, and not an auxiliary fixture root",
                );
            }
        }
    }
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Normalize `../` and `./` components without touching the filesystem, so an
/// include target can be expressed crate-relatively before canonicalization.
fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            other => normalized.push(other),
        }
    }
    normalized
}

struct FileScanner<'a, 'b, 'c> {
    gate: &'c mut Gate<'a, 'b>,
    file: String,
    module_dir: PathBuf,
    inline_modules: Vec<String>,
    in_boundary: bool,
    /// Nesting depth inside `#[cfg(test)]` items.
    cfg_test_depth: usize,
    /// Nesting depth inside an admitted FFI entry function.
    ffi_entry_depth: usize,
}

impl FileScanner<'_, '_, '_> {
    fn violation(&mut self, message: &str) {
        let file = self.file.clone();
        self.gate.violation(&file, message);
    }

    fn resolve_module(&mut self, declaration: &syn::ItemMod) {
        let name = declaration.ident.to_string();
        let mut directory = self.module_dir.clone();
        for inline in &self.inline_modules {
            directory.push(inline);
        }
        let file_candidate = directory.join(format!("{name}.rs"));
        let mod_candidate = directory.join(&name).join("mod.rs");
        let file_exists = self.gate.crate_root.join(&file_candidate).is_file();
        let mod_exists = self.gate.crate_root.join(&mod_candidate).is_file();
        match (file_exists, mod_exists) {
            (true, true) => self.violation(&format!(
                "module `{name}` is ambiguous: both {} and {} exist",
                relative_display(Path::new(""), &file_candidate),
                relative_display(Path::new(""), &mod_candidate),
            )),
            (false, false) => self.violation(&format!(
                "module `{name}` has no file at {} or {}",
                relative_display(Path::new(""), &file_candidate),
                relative_display(Path::new(""), &mod_candidate),
            )),
            (file_form, _) => {
                let chosen = if file_form {
                    file_candidate
                } else {
                    mod_candidate
                };
                let child_module_dir = directory.join(&name);
                let relative = relative_display(Path::new(""), &chosen);
                self.gate.schedule(relative, child_module_dir);
            }
        }
    }

    fn handle_include(&mut self, invocation: &syn::Macro) {
        let literal: syn::LitStr = match syn::parse2(invocation.tokens.clone()) {
            Ok(literal) => literal,
            Err(_) => {
                self.violation("include! with a non-literal argument");
                return;
            }
        };
        let argument = literal.value();
        let admitted = self
            .gate
            .config
            .admitted_includes
            .iter()
            .any(|(file, admitted)| *file == self.file && *admitted == argument);
        if !admitted {
            self.violation(&format!(
                "include!(\"{argument}\") is not an admitted include edge"
            ));
            return;
        }
        // `include!` resolves relative to the file that invokes it.
        let including_dir = Path::new(&self.file)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let target = normalize(&including_dir.join(&argument));
        let module_dir = target.parent().map(Path::to_path_buf).unwrap_or_default();
        let relative = relative_display(Path::new(""), &target);
        self.gate.schedule(relative, module_dir);
    }

    fn scan_token_stream(&mut self, stream: proc_macro2::TokenStream, context: &str) {
        let tokens: Vec<TokenTree> = stream.into_iter().collect();
        for (index, token) in tokens.iter().enumerate() {
            match token {
                TokenTree::Ident(ident) => {
                    let value = ident.to_string();
                    if value.starts_with("r#") {
                        self.violation(&format!("raw identifier `{value}` in {context} tokens"));
                    } else if DENIED_TOKEN_IDENTS.contains(&value.as_str())
                        || FORBIDDEN_FFI_SYMBOLS.contains(&value.as_str())
                    {
                        self.violation(&format!("forbidden token `{value}` in {context} tokens"));
                    }
                }
                TokenTree::Group(group) => {
                    if group.delimiter() == Delimiter::Bracket
                        && index > 0
                        && matches!(&tokens[index - 1], TokenTree::Punct(punct) if punct.as_char() == '#')
                    {
                        self.reject_smuggled_attribute(group);
                    }
                    self.scan_token_stream(group.stream(), context);
                }
                TokenTree::Punct(_) | TokenTree::Literal(_) => {}
            }
        }
    }

    fn reject_smuggled_attribute(&mut self, group: &proc_macro2::Group) {
        if let Some(TokenTree::Ident(first)) = group.stream().into_iter().next() {
            let value = first.to_string();
            if DENIED_SMUGGLED_ATTRIBUTES.contains(&value.as_str()) {
                self.violation(&format!(
                    "attribute `#[{value} …]` smuggled inside macro tokens"
                ));
            }
        }
    }

    fn inside_admitted_ffi_entry(&self) -> bool {
        self.ffi_entry_depth > 0
    }

    fn check_forbidden_path(&mut self, path: &syn::Path, context: &str) {
        if self.inside_admitted_ffi_entry() {
            return;
        }
        if let Some(symbol) = path_tail_is_forbidden(path) {
            self.violation(&format!(
                "forbidden FFI symbol `{symbol}` in {context}"
            ));
        }
        if path
            .segments
            .first()
            .is_some_and(|segment| FORBIDDEN_USE_ROOTS.contains(&segment.ident.to_string().as_str()))
        {
            self.violation(&format!("forbidden import root in {context}"));
        }
    }

    fn check_use_tree(&mut self, tree: &syn::UseTree) {
        if use_tree_has_forbidden_glob(tree) {
            self.violation("glob imports are forbidden");
        }
        if let Some(root) = use_tree_forbidden_root(tree) {
            self.violation(&format!("import of `{root}` is forbidden"));
        }
    }

    fn check_derive_list(&mut self, attribute: &syn::Attribute) {
        let list = match attribute.meta.require_list() {
            Ok(list) => list.tokens.clone(),
            Err(_) => {
                self.violation("derive attribute without a derive list");
                return;
            }
        };
        // The last identifier of each comma-separated path is the derive name:
        // `#[derive(Debug, serde::Serialize)]` checks Debug and Serialize.
        let mut names: Vec<String> = Vec::new();
        let mut current: Option<String> = None;
        for token in list {
            match token {
                TokenTree::Ident(ident) => current = Some(ident.to_string()),
                TokenTree::Punct(punct) if punct.as_char() == ',' => {
                    names.extend(current.take());
                }
                _ => {}
            }
        }
        names.extend(current.take());
        for name in names {
            if !self.gate.config.allowed_derives.contains(&name.as_str()) {
                self.violation(&format!("derive `{name}` outside the closed allowlist"));
            }
        }
    }
}

impl<'ast> Visit<'ast> for FileScanner<'_, '_, '_> {
    fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
        let path = attribute.path();
        let name = match path.get_ident() {
            Some(ident) => ident.to_string(),
            None => {
                let joined: Vec<String> = path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect();
                self.violation(&format!(
                    "attribute `{}` outside the closed allowlist",
                    joined.join("::")
                ));
                return;
            }
        };
        if !self.gate.config.allowed_attributes.contains(&name.as_str()) {
            self.violation(&format!("attribute `{name}` outside the closed allowlist"));
            return;
        }
        if name == "derive" {
            self.check_derive_list(attribute);
            return;
        }
        if let syn::Meta::List(list) = &attribute.meta {
            self.scan_token_stream(list.tokens.clone(), &format!("attribute `{name}`"));
        }
        // Delegate so a macro nested in a name-value form (for example
        // `#[doc = include_str!(…)]`) still reaches `visit_macro`.
        visit::visit_attribute(self, attribute);
    }

    fn visit_item_mod(&mut self, declaration: &'ast syn::ItemMod) {
        let previous_cfg = self.cfg_test_depth;
        if attr_is_cfg_test(&declaration.attrs) {
            self.cfg_test_depth += 1;
        }
        if declaration.content.is_some() {
            self.inline_modules.push(declaration.ident.to_string());
            visit::visit_item_mod(self, declaration);
            self.inline_modules.pop();
        } else {
            visit::visit_item_mod(self, declaration);
            self.resolve_module(declaration);
        }
        self.cfg_test_depth = previous_cfg;
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if item.ident.is_some() {
            self.violation("project macro definitions are forbidden (macro_rules!)");
            return;
        }
        visit::visit_item_macro(self, item);
    }

    fn visit_macro(&mut self, invocation: &'ast syn::Macro) {
        let segments: Vec<String> = invocation
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        let joined = segments.join("::");
        match joined.as_str() {
            "include" => {
                self.handle_include(invocation);
                return;
            }
            "include_str" | "include_bytes" => {
                self.violation(&format!("{joined}! is forbidden"));
                return;
            }
            _ => {}
        }
        let allowed = segments.len() == 1
            && self
                .gate
                .config
                .allowed_macros
                .contains(&segments[0].as_str());
        if !allowed {
            self.violation(&format!("macro `{joined}!` outside the closed allowlist"));
            return;
        }
        self.scan_token_stream(invocation.tokens.clone(), &format!("macro `{joined}!`"));
    }

    fn visit_expr_unsafe(&mut self, expression: &'ast syn::ExprUnsafe) {
        if !self.in_boundary {
            self.violation("unsafe block outside the declared FFI boundary");
        } else if !self.inside_admitted_ffi_entry() {
            self.violation(
                "unsafe block outside the admitted production FFI entry points",
            );
        }
        visit::visit_expr_unsafe(self, expression);
    }

    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        if function.sig.unsafety.is_some() && !self.in_boundary {
            self.violation("unsafe fn outside the declared FFI boundary");
        }
        let previous_cfg = self.cfg_test_depth;
        if attr_is_cfg_test(&function.attrs) {
            self.cfg_test_depth += 1;
        }
        let name = function.sig.ident.to_string();
        let admitted = admitted_production_entry_function(&self.file, &name)
            || (self.cfg_test_depth > 0 && admitted_test_only_entry_function(&self.file, &name));
        let previous_entry = self.ffi_entry_depth;
        if admitted {
            self.ffi_entry_depth += 1;
        }
        visit::visit_item_fn(self, function);
        self.ffi_entry_depth = previous_entry;
        self.cfg_test_depth = previous_cfg;
    }

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        self.check_use_tree(&node.tree);
        visit::visit_item_use(self, node);
    }

    fn visit_item_const(&mut self, constant: &'ast syn::ItemConst) {
        if self.in_boundary
            && self.cfg_test_depth == 0
            && constant.ident == "OPENSSL_INIT_LOAD_CONFIG"
        {
            self.violation(
                "the OpenSSL LOAD_CONFIG constant may not be declared outside `#[cfg(test)]`",
            );
        }
        visit::visit_item_const(self, constant);
    }

    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        self.check_forbidden_path(&path.path, "expression");
        visit::visit_expr_path(self, path);
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = &*call.func {
            self.check_forbidden_path(&path.path, "call");
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_impl_item_fn(&mut self, function: &'ast syn::ImplItemFn) {
        if function.sig.unsafety.is_some() && !self.in_boundary {
            self.violation("unsafe method outside the declared FFI boundary");
        }
        visit::visit_impl_item_fn(self, function);
    }

    fn visit_trait_item_fn(&mut self, function: &'ast syn::TraitItemFn) {
        if function.sig.unsafety.is_some() && !self.in_boundary {
            self.violation("unsafe trait method outside the declared FFI boundary");
        }
        visit::visit_trait_item_fn(self, function);
    }

    fn visit_item_impl(&mut self, implementation: &'ast syn::ItemImpl) {
        if implementation.unsafety.is_some() && !self.in_boundary {
            self.violation("unsafe impl outside the declared FFI boundary");
        }
        visit::visit_item_impl(self, implementation);
    }

    fn visit_item_trait(&mut self, definition: &'ast syn::ItemTrait) {
        if definition.unsafety.is_some() && !self.in_boundary {
            self.violation("unsafe trait outside the declared FFI boundary");
        }
        visit::visit_item_trait(self, definition);
    }

    fn visit_type_bare_fn(&mut self, bare_fn: &'ast syn::TypeBareFn) {
        if bare_fn.unsafety.is_some() && !self.in_boundary {
            self.violation("unsafe fn pointer type outside the declared FFI boundary");
        }
        visit::visit_type_bare_fn(self, bare_fn);
    }

    fn visit_item_foreign_mod(&mut self, foreign: &'ast syn::ItemForeignMod) {
        if !self.in_boundary {
            self.violation("extern block outside the declared FFI boundary");
        } else {
            for item in &foreign.items {
                if let syn::ForeignItem::Fn(function) = item {
                    let name = function.sig.ident.to_string();
                    if !admitted_extern_symbol(&self.file, &name) {
                        self.violation(&format!(
                            "extern symbol `{name}` is not an admitted FFI declaration"
                        ));
                    }
                } else {
                    self.violation("only `extern \"C\" fn` declarations are admitted");
                }
            }
        }
        visit::visit_item_foreign_mod(self, foreign);
    }

    fn visit_item_extern_crate(&mut self, declaration: &'ast syn::ItemExternCrate) {
        self.violation("extern crate declarations are forbidden");
        visit::visit_item_extern_crate(self, declaration);
    }
}
