use tower_lsp::lsp_types::{
    SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens,
};

use crate::position::LineIndex;
use crate::snapshot::AnalysisSnapshot;
use syntax::ast::{Expression, ImportAlias, Pattern, RestPattern, Span};
use syntax::program::File;

/// Maps a token type string to its index in the legend.
pub(crate) struct TokenTypeMap {
    types: Vec<SemanticTokenType>,
}

impl TokenTypeMap {
    pub(crate) fn new(types: Vec<SemanticTokenType>) -> Self {
        Self { types }
    }

    pub(crate) fn get(&self, ty: SemanticTokenType) -> u32 {
        self.types
            .iter()
            .position(|t| t.as_str() == ty.as_str())
            .map(|i| i as u32)
            .unwrap_or(0)
    }
}

/// Maps a token modifier string to its bitflag index.
pub(crate) struct TokenModifierMap {
    modifiers: Vec<SemanticTokenModifier>,
}

impl TokenModifierMap {
    pub(crate) fn new(modifiers: Vec<SemanticTokenModifier>) -> Self {
        Self { modifiers }
    }

    pub(crate) fn encode(&self, modifiers: &[SemanticTokenModifier]) -> u32 {
        let mut result = 0;
        for m in modifiers {
            if let Some(bit) = self.modifiers.iter().position(|x| x.as_str() == m.as_str()) {
                result |= 1 << bit;
            }
        }
        result
    }
}

/// Accumulates semantic tokens for a single file.
pub(crate) struct SemanticTokensBuilder {
    tokens: Vec<SemanticToken>,
    prev_line: u32,
    prev_start: u32,
}

impl SemanticTokensBuilder {
    pub(crate) fn new() -> Self {
        Self {
            tokens: Vec::new(),
            prev_line: 0,
            prev_start: 0,
        }
    }

    pub(crate) fn push(
        &mut self,
        line_index: &LineIndex,
        span: &Span,
        token_type: u32,
        token_modifiers_bitset: u32,
    ) {
        if span.is_dummy() {
            return;
        }
        let range = line_index.span_to_range(*span);
        let line = range.start.line;
        let start = range.start.character;
        let length = range.end.character.saturating_sub(range.start.character);
        if length == 0 {
            return;
        }

        let delta_line = line.saturating_sub(self.prev_line);
        let delta_start = if delta_line == 0 {
            start.saturating_sub(self.prev_start)
        } else {
            start
        };

        self.tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset,
        });

        self.prev_line = line;
        self.prev_start = start;
    }

    pub(crate) fn build(self) -> SemanticTokens {
        SemanticTokens {
            result_id: None,
            data: self.tokens,
        }
    }
}

/// Entrypoint: compute semantic tokens for a file.
pub(crate) fn compute_semantic_tokens(
    file: &File,
    snapshot: &AnalysisSnapshot,
    line_index: &LineIndex,
) -> SemanticTokens {
    let type_map = TokenTypeMap::new(vec![
        SemanticTokenType::NAMESPACE,
        SemanticTokenType::TYPE,
        SemanticTokenType::CLASS,
        SemanticTokenType::ENUM,
        SemanticTokenType::INTERFACE,
        SemanticTokenType::STRUCT,
        SemanticTokenType::TYPE_PARAMETER,
        SemanticTokenType::PARAMETER,
        SemanticTokenType::VARIABLE,
        SemanticTokenType::PROPERTY,
        SemanticTokenType::ENUM_MEMBER,
        SemanticTokenType::FUNCTION,
        SemanticTokenType::METHOD,
        SemanticTokenType::COMMENT,
        SemanticTokenType::STRING,
        SemanticTokenType::NUMBER,
        SemanticTokenType::KEYWORD,
        SemanticTokenType::OPERATOR,
    ]);

    let modifier_map = TokenModifierMap::new(vec![
        SemanticTokenModifier::DECLARATION,
        SemanticTokenModifier::READONLY,
        SemanticTokenModifier::STATIC,
        SemanticTokenModifier::ASYNC,
        SemanticTokenModifier::MODIFICATION,
        SemanticTokenModifier::DOCUMENTATION,
    ]);

    let mut builder = SemanticTokensBuilder::new();
    for item in &file.items {
        emit_tokens(
            item,
            file,
            snapshot,
            line_index,
            &type_map,
            &modifier_map,
            &mut builder,
        );
    }
    for (offset, length) in &snapshot.trivia().comments {
        let span = Span::new(file.id, *offset, *length);
        builder.push(line_index, &span, type_map.get(SemanticTokenType::COMMENT), 0);
    }
    for (offset, length) in &snapshot.trivia().doc_comments {
        let span = Span::new(file.id, *offset, *length);
        builder.push(
            line_index,
            &span,
            type_map.get(SemanticTokenType::COMMENT),
            modifier_map.encode(&[SemanticTokenModifier::DOCUMENTATION]),
        );
    }
    builder.build()
}

fn emit_tokens(
    expr: &Expression,
    file: &File,
    snapshot: &AnalysisSnapshot,
    line_index: &LineIndex,
    type_map: &TokenTypeMap,
    modifier_map: &TokenModifierMap,
    builder: &mut SemanticTokensBuilder,
) {
    match expr {
        Expression::Struct {
            name: _, name_span, ..
        } => {
            builder.push(
                line_index,
                name_span,
                type_map.get(SemanticTokenType::STRUCT),
                modifier_map.encode(&[SemanticTokenModifier::DECLARATION]),
            );
        }
        Expression::Enum {
            name: _, name_span, ..
        } => {
            builder.push(
                line_index,
                name_span,
                type_map.get(SemanticTokenType::ENUM),
                modifier_map.encode(&[SemanticTokenModifier::DECLARATION]),
            );
        }
        Expression::ValueEnum {
            name: _, name_span, ..
        } => {
            builder.push(
                line_index,
                name_span,
                type_map.get(SemanticTokenType::ENUM),
                modifier_map.encode(&[SemanticTokenModifier::DECLARATION]),
            );
        }
        Expression::Interface {
            name: _, name_span, ..
        } => {
            builder.push(
                line_index,
                name_span,
                type_map.get(SemanticTokenType::INTERFACE),
                modifier_map.encode(&[SemanticTokenModifier::DECLARATION]),
            );
        }
        Expression::TypeAlias {
            name: _, name_span, ..
        } => {
            builder.push(
                line_index,
                name_span,
                type_map.get(SemanticTokenType::TYPE),
                modifier_map.encode(&[SemanticTokenModifier::DECLARATION]),
            );
        }
        Expression::Function {
            name: _,
            name_span,
            params,
            ..
        } => {
            builder.push(
                line_index,
                name_span,
                type_map.get(SemanticTokenType::FUNCTION),
                modifier_map.encode(&[SemanticTokenModifier::DECLARATION]),
            );
            for param in params {
                emit_pattern_tokens(&param.pattern, line_index, type_map, modifier_map, builder);
            }
        }
        Expression::ImplBlock { methods, .. } => {
            for method in methods {
                emit_tokens(
                    method,
                    file,
                    snapshot,
                    line_index,
                    type_map,
                    modifier_map,
                    builder,
                );
            }
        }
        Expression::Const {
            identifier: _,
            identifier_span,
            ..
        } => {
            builder.push(
                line_index,
                identifier_span,
                type_map.get(SemanticTokenType::VARIABLE),
                modifier_map.encode(&[
                    SemanticTokenModifier::READONLY,
                    SemanticTokenModifier::DECLARATION,
                ]),
            );
        }
        Expression::VariableDeclaration {
            name: _, name_span, ..
        } => {
            builder.push(
                line_index,
                name_span,
                type_map.get(SemanticTokenType::VARIABLE),
                modifier_map.encode(&[SemanticTokenModifier::DECLARATION]),
            );
        }
        Expression::Let { binding, value, .. } => {
            emit_pattern_tokens(
                &binding.pattern,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            emit_tokens(
                value,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
        }
        Expression::For {
            binding,
            iterable,
            body,
            ..
        } => {
            emit_pattern_tokens(
                &binding.pattern,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            emit_tokens(
                iterable,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            emit_tokens(
                body,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
        }
        Expression::Lambda { params, body, .. } => {
            for param in params {
                emit_pattern_tokens(&param.pattern, line_index, type_map, modifier_map, builder);
            }
            emit_tokens(
                body,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
        }
        Expression::Identifier {
            value,
            qualified,
            binding_id,
            span,
            ..
        } => {
            if let Some(qname) = qualified {
                if let Some(def) = snapshot.definitions().get(qname.as_str()) {
                    let token_type = definition_to_token_type(def, type_map);
                    builder.push(line_index, span, token_type, 0);
                } else if let Some(binding_id) = binding_id
                    && let Some(binding) = snapshot.facts().bindings.get(binding_id)
                {
                    let ty = if binding.kind.is_param() {
                        SemanticTokenType::PARAMETER
                    } else {
                        SemanticTokenType::VARIABLE
                    };
                    builder.push(line_index, span, type_map.get(ty), 0);
                }
            } else if let Some(binding_id) = binding_id {
                if let Some(binding) = snapshot.facts().bindings.get(binding_id) {
                    let ty = if binding.kind.is_param() {
                        SemanticTokenType::PARAMETER
                    } else {
                        SemanticTokenType::VARIABLE
                    };
                    builder.push(line_index, span, type_map.get(ty), 0);
                }
            } else {
                // Try to resolve unqualified identifier via definitions
                let qualified = format!("{}.{}", file.module_id, value);
                if let Some(def) = snapshot.definitions().get(qualified.as_str()) {
                    let token_type = definition_to_token_type(def, type_map);
                    builder.push(line_index, span, token_type, 0);
                }
            }
        }
        Expression::DotAccess {
            expression,
            member,
            span,
            ..
        } => {
            emit_tokens(
                expression,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            let member_type = resolve_dot_access_member(expression, member, snapshot, type_map);
            let member_len = member.len() as u32;
            if span.byte_length > member_len {
                let member_span = Span::new(
                    span.file_id,
                    span.byte_offset + span.byte_length - member_len,
                    member_len,
                );
                builder.push(line_index, &member_span, member_type, 0);
            }
        }
        Expression::Match { arms, .. } => {
            for arm in arms {
                emit_pattern_tokens(&arm.pattern, line_index, type_map, modifier_map, builder);
                if let Some(guard) = &arm.guard {
                    emit_tokens(
                        guard,
                        file,
                        snapshot,
                        line_index,
                        type_map,
                        modifier_map,
                        builder,
                    );
                }
                emit_tokens(
                    &arm.expression,
                    file,
                    snapshot,
                    line_index,
                    type_map,
                    modifier_map,
                    builder,
                );
            }
        }
        Expression::IfLet {
            pattern,
            scrutinee,
            consequence,
            alternative,
            ..
        } => {
            emit_pattern_tokens(pattern, line_index, type_map, modifier_map, builder);
            emit_tokens(
                scrutinee,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            emit_tokens(
                consequence,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            emit_tokens(
                alternative,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
        }
        Expression::WhileLet {
            pattern,
            scrutinee,
            body,
            ..
        } => {
            emit_pattern_tokens(pattern, line_index, type_map, modifier_map, builder);
            emit_tokens(
                scrutinee,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            emit_tokens(
                body,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
        }
        Expression::StructCall {
            field_assignments, ..
        } => {
            for fa in field_assignments {
                emit_tokens(
                    &fa.value,
                    file,
                    snapshot,
                    line_index,
                    type_map,
                    modifier_map,
                    builder,
                );
            }
        }
        Expression::Binary { left, right, span, .. } => {
            emit_tokens(
                left,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            emit_tokens(
                right,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            let left_span = left.get_span();
            let right_span = right.get_span();
            let op_start = left_span.byte_offset + left_span.byte_length;
            let op_end = right_span.byte_offset;
            if op_start < op_end {
                let op_source = &file.source[op_start as usize..op_end as usize];
                let trimmed = op_source.trim();
                if !trimmed.is_empty() {
                    let leading = op_source.len() - op_source.trim_start().len();
                    let op_span = Span::new(
                        span.file_id,
                        op_start + leading as u32,
                        trimmed.len() as u32,
                    );
                    builder.push(
                        line_index,
                        &op_span,
                        type_map.get(SemanticTokenType::OPERATOR),
                        0,
                    );
                }
            }
        }
        Expression::Unary { expression, span, .. } => {
            emit_tokens(
                expression,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            let expr_span = expression.get_span();
            if span.byte_offset < expr_span.byte_offset {
                let op_source =
                    &file.source[span.byte_offset as usize..expr_span.byte_offset as usize];
                let trimmed = op_source.trim();
                if !trimmed.is_empty() {
                    let leading = op_source.len() - op_source.trim_start().len();
                    let op_span = Span::new(
                        span.file_id,
                        span.byte_offset + leading as u32,
                        trimmed.len() as u32,
                    );
                    builder.push(
                        line_index,
                        &op_span,
                        type_map.get(SemanticTokenType::OPERATOR),
                        0,
                    );
                }
            }
        }
        Expression::Block { items, .. } => {
            for item in items {
                emit_tokens(
                    item,
                    file,
                    snapshot,
                    line_index,
                    type_map,
                    modifier_map,
                    builder,
                );
            }
        }
        Expression::Tuple { elements, .. } => {
            for e in elements {
                emit_tokens(
                    e,
                    file,
                    snapshot,
                    line_index,
                    type_map,
                    modifier_map,
                    builder,
                );
            }
        }
        Expression::Call {
            expression, args, ..
        } => {
            emit_tokens(
                expression,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            for arg in args {
                emit_tokens(
                    arg,
                    file,
                    snapshot,
                    line_index,
                    type_map,
                    modifier_map,
                    builder,
                );
            }
        }
        Expression::If {
            condition,
            consequence,
            alternative,
            ..
        } => {
            emit_tokens(
                condition,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            emit_tokens(
                consequence,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            emit_tokens(
                alternative,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
        }
        Expression::While {
            condition, body, ..
        } => {
            emit_tokens(
                condition,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            emit_tokens(
                body,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
        }
        Expression::Return { expression, .. } => {
            emit_tokens(
                expression,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
        }
        Expression::Defer { expression, .. } => {
            emit_tokens(
                expression,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
        }
        Expression::Select { arms, .. } => {
            for arm in arms {
                match &arm.pattern {
                    syntax::ast::SelectArmPattern::Receive { body, .. } => {
                        emit_tokens(
                            body,
                            file,
                            snapshot,
                            line_index,
                            type_map,
                            modifier_map,
                            builder,
                        );
                    }
                    syntax::ast::SelectArmPattern::Send { body, .. } => {
                        emit_tokens(
                            body,
                            file,
                            snapshot,
                            line_index,
                            type_map,
                            modifier_map,
                            builder,
                        );
                    }
                    syntax::ast::SelectArmPattern::MatchReceive { arms, .. } => {
                        for ma in arms {
                            emit_tokens(
                                &ma.expression,
                                file,
                                snapshot,
                                line_index,
                                type_map,
                                modifier_map,
                                builder,
                            );
                        }
                    }
                    syntax::ast::SelectArmPattern::WildCard { body } => {
                        emit_tokens(
                            body,
                            file,
                            snapshot,
                            line_index,
                            type_map,
                            modifier_map,
                            builder,
                        );
                    }
                }
            }
        }
        Expression::Task { expression, .. } => {
            emit_tokens(
                expression,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
        }
        Expression::TryBlock { items, .. } => {
            for item in items {
                emit_tokens(
                    item,
                    file,
                    snapshot,
                    line_index,
                    type_map,
                    modifier_map,
                    builder,
                );
            }
        }
        Expression::RecoverBlock { items, .. } => {
            for item in items {
                emit_tokens(
                    item,
                    file,
                    snapshot,
                    line_index,
                    type_map,
                    modifier_map,
                    builder,
                );
            }
        }
        Expression::Break { value, .. } => {
            if let Some(value) = value {
                emit_tokens(
                    value,
                    file,
                    snapshot,
                    line_index,
                    type_map,
                    modifier_map,
                    builder,
                );
            }
        }
        Expression::Continue { .. } => {}
        Expression::ModuleImport { alias, .. } => {
            if let Some(alias) = alias {
                match alias {
                    ImportAlias::Named(_, span) | ImportAlias::Blank(span) => {
                        builder.push(
                            line_index,
                            span,
                            type_map.get(SemanticTokenType::NAMESPACE),
                            modifier_map.encode(&[SemanticTokenModifier::DECLARATION]),
                        );
                    }
                }
            }
        }
        Expression::Paren { expression, .. } => {
            emit_tokens(
                expression,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
        }
        Expression::Range { start, end, span, .. } => {
            let mut range_start = span.byte_offset;
            let mut range_end = span.byte_offset + span.byte_length;
            if let Some(start) = start {
                emit_tokens(
                    start,
                    file,
                    snapshot,
                    line_index,
                    type_map,
                    modifier_map,
                    builder,
                );
                let start_span = start.get_span();
                range_start = start_span.byte_offset + start_span.byte_length;
            }
            if let Some(end) = end {
                emit_tokens(
                    end,
                    file,
                    snapshot,
                    line_index,
                    type_map,
                    modifier_map,
                    builder,
                );
                let end_span = end.get_span();
                range_end = end_span.byte_offset;
            }
            if range_start < range_end {
                let op_source = &file.source[range_start as usize..range_end as usize];
                if let Some(pos) = op_source.find("..") {
                    let op_span = Span::new(span.file_id, range_start + pos as u32, 2);
                    builder.push(
                        line_index,
                        &op_span,
                        type_map.get(SemanticTokenType::OPERATOR),
                        0,
                    );
                }
            }
        }
        Expression::IndexedAccess {
            expression, index, ..
        } => {
            emit_tokens(
                expression,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            emit_tokens(
                index,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
        }
        Expression::Cast { expression, .. } => {
            emit_tokens(
                expression,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
        }
        Expression::Loop { body, .. } => {
            emit_tokens(
                body,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
        }
        Expression::Propagate { expression, span, .. } => {
            emit_tokens(
                expression,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            let expr_span = expression.get_span();
            let op_start = expr_span.byte_offset + expr_span.byte_length;
            let op_end = span.byte_offset + span.byte_length;
            if op_start < op_end {
                let op_source = &file.source[op_start as usize..op_end as usize];
                if let Some(pos) = op_source.find('?') {
                    let op_span = Span::new(span.file_id, op_start + pos as u32, 1);
                    builder.push(
                        line_index,
                        &op_span,
                        type_map.get(SemanticTokenType::OPERATOR),
                        0,
                    );
                }
            }
        }
        Expression::Reference { expression, .. } => {
            emit_tokens(
                expression,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
        }
        Expression::Assignment { target, value, span, .. } => {
            emit_tokens(
                target,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            emit_tokens(
                value,
                file,
                snapshot,
                line_index,
                type_map,
                modifier_map,
                builder,
            );
            let target_span = target.get_span();
            let value_span = value.get_span();
            let op_start = target_span.byte_offset + target_span.byte_length;
            let op_end = value_span.byte_offset;
            if op_start < op_end {
                let op_source = &file.source[op_start as usize..op_end as usize];
                let trimmed = op_source.trim();
                if !trimmed.is_empty() {
                    let leading = op_source.len() - op_source.trim_start().len();
                    let op_span = Span::new(
                        span.file_id,
                        op_start + leading as u32,
                        trimmed.len() as u32,
                    );
                    builder.push(
                        line_index,
                        &op_span,
                        type_map.get(SemanticTokenType::OPERATOR),
                        0,
                    );
                }
            }
        }
        Expression::Literal { literal, span, .. } => {
            match literal {
                syntax::ast::Literal::FormatString(parts) => {
                    for part in parts {
                        if let syntax::ast::FormatStringPart::Expression(expr) = part {
                            emit_tokens(
                                expr,
                                file,
                                snapshot,
                                line_index,
                                type_map,
                                modifier_map,
                                builder,
                            );
                        }
                    }
                    builder.push(
                        line_index,
                        span,
                        type_map.get(SemanticTokenType::STRING),
                        0,
                    );
                }
                syntax::ast::Literal::Slice(elements) => {
                    for elem in elements {
                        emit_tokens(
                            elem,
                            file,
                            snapshot,
                            line_index,
                            type_map,
                            modifier_map,
                            builder,
                        );
                    }
                }
                other => {
                    let token_type = match other {
                        syntax::ast::Literal::Integer { .. }
                        | syntax::ast::Literal::Float { .. }
                        | syntax::ast::Literal::Imaginary(_) => SemanticTokenType::NUMBER,
                        syntax::ast::Literal::String { .. }
                        | syntax::ast::Literal::Char(_) => SemanticTokenType::STRING,
                        syntax::ast::Literal::Boolean(_) => SemanticTokenType::KEYWORD,
                        syntax::ast::Literal::FormatString(_)
                        | syntax::ast::Literal::Slice(_) => unreachable!(),
                    };
                    builder.push(line_index, span, type_map.get(token_type), 0);
                }
            }
        }
        Expression::Unit { .. } => {}
        Expression::RawGo { .. } | Expression::NoOp => {}
    }
}

fn emit_pattern_tokens(
    pattern: &Pattern,
    line_index: &LineIndex,
    type_map: &TokenTypeMap,
    modifier_map: &TokenModifierMap,
    builder: &mut SemanticTokensBuilder,
) {
    match pattern {
        Pattern::Identifier {
            identifier: _,
            span,
            ..
        } => {
            builder.push(
                line_index,
                span,
                type_map.get(SemanticTokenType::VARIABLE),
                modifier_map.encode(&[SemanticTokenModifier::DECLARATION]),
            );
        }
        Pattern::EnumVariant {
            identifier: _,
            fields,
            span,
            ..
        } => {
            builder.push(
                line_index,
                span,
                type_map.get(SemanticTokenType::ENUM_MEMBER),
                0,
            );
            for field in fields {
                emit_pattern_tokens(field, line_index, type_map, modifier_map, builder);
            }
        }
        Pattern::Tuple { elements, .. } => {
            for e in elements {
                emit_pattern_tokens(e, line_index, type_map, modifier_map, builder);
            }
        }
        Pattern::Struct {
            identifier: _,
            fields,
            span,
            ..
        } => {
            builder.push(
                line_index,
                span,
                type_map.get(SemanticTokenType::ENUM_MEMBER),
                0,
            );
            for field in fields {
                emit_pattern_tokens(&field.value, line_index, type_map, modifier_map, builder);
            }
        }
        Pattern::WildCard { .. } | Pattern::Literal { .. } | Pattern::Unit { .. } => {}
        Pattern::AsBinding {
            name: _,
            pattern,
            span,
            ..
        } => {
            builder.push(
                line_index,
                span,
                type_map.get(SemanticTokenType::VARIABLE),
                modifier_map.encode(&[SemanticTokenModifier::DECLARATION]),
            );
            emit_pattern_tokens(pattern, line_index, type_map, modifier_map, builder);
        }
        Pattern::Or { patterns, .. } => {
            for p in patterns {
                emit_pattern_tokens(p, line_index, type_map, modifier_map, builder);
            }
        }
        Pattern::Slice { prefix, rest, .. } => {
            for p in prefix {
                emit_pattern_tokens(p, line_index, type_map, modifier_map, builder);
            }
            if let RestPattern::Bind { name: _, span } = rest {
                builder.push(
                    line_index,
                    span,
                    type_map.get(SemanticTokenType::VARIABLE),
                    modifier_map.encode(&[SemanticTokenModifier::DECLARATION]),
                );
            }
        }
    }
}

fn definition_to_token_type(def: &syntax::program::Definition, type_map: &TokenTypeMap) -> u32 {
    use syntax::program::DefinitionBody;
    match &def.body {
        DefinitionBody::Struct { .. } => type_map.get(SemanticTokenType::STRUCT),
        DefinitionBody::Enum { .. } | DefinitionBody::ValueEnum { .. } => {
            type_map.get(SemanticTokenType::ENUM)
        }
        DefinitionBody::Interface { .. } => type_map.get(SemanticTokenType::INTERFACE),
        DefinitionBody::TypeAlias { .. } => type_map.get(SemanticTokenType::TYPE),
        DefinitionBody::Value { .. } => {
            if matches!(
                &def.ty,
                syntax::types::Type::Function(_) | syntax::types::Type::Forall { .. }
            ) {
                type_map.get(SemanticTokenType::FUNCTION)
            } else {
                type_map.get(SemanticTokenType::VARIABLE)
            }
        }
    }
}

fn resolve_dot_access_member(
    receiver: &Expression,
    member: &str,
    snapshot: &AnalysisSnapshot,
    type_map: &TokenTypeMap,
) -> u32 {
    let Some(type_id) = crate::analysis::type_name(&receiver.get_type()) else {
        return type_map.get(SemanticTokenType::PROPERTY);
    };

    // Check if it's a struct field
    if let Some(syntax::program::Definition {
        body: syntax::program::DefinitionBody::Struct { fields, .. },
        ..
    }) = snapshot.definitions().get(type_id.as_str())
        && fields.iter().any(|f| f.name == member)
    {
        return type_map.get(SemanticTokenType::PROPERTY);
    }

    // Check if it's a method
    let method_prefix = format!("{type_id}.{member}");
    if let Some(method_def) = snapshot.definitions().get(method_prefix.as_str())
        && matches!(
            &method_def.body,
            syntax::program::DefinitionBody::Value { .. }
        )
    {
        let func_ty = match &method_def.ty {
            syntax::types::Type::Forall { body, .. } => body,
            other => other,
        };
        if let syntax::types::Type::Function(f) = func_ty
            && !f.params.is_empty()
        {
            let first_param_type = crate::analysis::type_name(&f.params[0]);
            if first_param_type.as_deref() == Some(&type_id) {
                return type_map.get(SemanticTokenType::METHOD);
            }
        }
        return type_map.get(SemanticTokenType::FUNCTION);
    }

    type_map.get(SemanticTokenType::PROPERTY)
}
