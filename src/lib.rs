use std::collections::BTreeSet;
use std::fmt;

pub fn compile_to_latex(source: &str) -> Result<String, Diagnostic> {
    let document = parse(source)?;
    Ok(emit_document(&document))
}

pub fn parse(source: &str) -> Result<Document, Diagnostic> {
    Parser::new(source).parse_document()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    line: usize,
    message: String,
}

impl Diagnostic {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for Diagnostic {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    title: String,
    author: Option<String>,
    date: Option<DateValue>,
    definitions: Vec<Definition>,
    uses: BTreeSet<Feature>,
    body: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Definition {
    name: String,
    params: Vec<String>,
    latex: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DateValue {
    Literal(String),
    Today,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Feature {
    Bibliography,
    Graphics,
    Math,
    Theorem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Block {
    Abstract(Vec<Block>),
    Bibliography(String),
    Compare {
        latex: String,
        vitamins: Vec<Inline>,
    },
    Equation {
        label: Option<String>,
        expr: String,
    },
    Figure {
        path: String,
        width: Option<String>,
        caption: Option<String>,
        label: Option<String>,
    },
    Items(Vec<Vec<Inline>>),
    Paragraph(Vec<Inline>),
    Proof(Vec<Block>),
    Quote(Vec<Block>),
    RawLatex(String),
    Section {
        level: u8,
        title: String,
        children: Vec<Block>,
    },
    Steps(Vec<Vec<Inline>>),
    Table {
        columns: Vec<Align>,
        rows: Vec<Vec<Vec<Inline>>>,
        rules: Vec<usize>,
        caption: Option<String>,
    },
    Theorem {
        title: Option<String>,
        children: Vec<Block>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Align {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Inline {
    Bold(Vec<Inline>),
    Cite(String),
    Italic(Vec<Inline>),
    Latex(String),
    Math(String),
    Ref(String),
    SmallCaps(Vec<Inline>),
    Text(String),
}

#[derive(Debug, Clone)]
struct SourceLine {
    number: usize,
    text: String,
}

struct Parser {
    lines: Vec<SourceLine>,
    position: usize,
}

impl Parser {
    fn new(source: &str) -> Self {
        let lines = source
            .lines()
            .enumerate()
            .map(|(index, text)| SourceLine {
                number: index + 1,
                text: text.to_string(),
            })
            .collect();

        Self { lines, position: 0 }
    }

    fn parse_document(mut self) -> Result<Document, Diagnostic> {
        let opening = self
            .take_significant()
            .ok_or_else(|| Diagnostic::new(1, "expected `paper \"Title\" do`"))?;
        let title = parse_titled_do(opening.text.trim(), "paper", opening.number)?;

        let mut document = Document {
            title,
            author: None,
            date: None,
            definitions: Vec::new(),
            uses: BTreeSet::new(),
            body: Vec::new(),
        };

        loop {
            let line = self
                .take_significant()
                .ok_or_else(|| Diagnostic::new(opening.number, "unclosed paper block"))?;
            let trimmed = line.text.trim();

            if trimmed == "end" {
                return Ok(document);
            }

            if self.parse_metadata(trimmed, line.number, &mut document)? {
                continue;
            }

            document.body.push(self.parse_block_from_line(line)?);
        }
    }

    fn parse_metadata(
        &mut self,
        trimmed: &str,
        line: usize,
        document: &mut Document,
    ) -> Result<bool, Diagnostic> {
        if let Some(rest) = trimmed.strip_prefix("author ") {
            document.author = Some(parse_required_string(rest.trim(), line)?);
            return Ok(true);
        }

        if let Some(rest) = trimmed.strip_prefix("date ") {
            document.date = Some(if rest.trim() == "today" {
                DateValue::Today
            } else {
                DateValue::Literal(parse_required_string(rest.trim(), line)?)
            });
            return Ok(true);
        }

        if let Some(rest) = trimmed.strip_prefix("use ") {
            document.uses.insert(parse_feature(rest.trim(), line)?);
            return Ok(true);
        }

        if let Some(rest) = trimmed.strip_prefix("define ") {
            let definition = parse_definition(rest.trim(), line)?;
            if document
                .definitions
                .iter()
                .any(|existing| existing.name == definition.name)
            {
                return Err(Diagnostic::new(
                    line,
                    format!("macro `{}` is already defined", definition.name),
                ));
            }
            document.definitions.push(definition);
            return Ok(true);
        }

        Ok(false)
    }

    fn parse_blocks(&mut self, opener: usize) -> Result<Vec<Block>, Diagnostic> {
        let mut blocks = Vec::new();

        loop {
            let line = self
                .take_significant()
                .ok_or_else(|| Diagnostic::new(opener, "unclosed block"))?;

            if line.text.trim() == "end" {
                return Ok(blocks);
            }

            blocks.push(self.parse_block_from_line(line)?);
        }
    }

    fn parse_block_from_line(&mut self, line: SourceLine) -> Result<Block, Diagnostic> {
        let trimmed = line.text.trim();

        if let Some(rest) = trimmed.strip_prefix("p ") {
            return Ok(Block::Paragraph(parse_inline_list(rest, line.number)?));
        }

        if let Some(rest) = trimmed.strip_prefix("raw_latex ") {
            return Ok(Block::RawLatex(parse_required_string(
                rest.trim(),
                line.number,
            )?));
        }

        if let Some(rest) = trimmed.strip_prefix("latex ") {
            return Ok(Block::RawLatex(parse_required_string(
                rest.trim(),
                line.number,
            )?));
        }

        if let Some(rest) = trimmed.strip_prefix("bibliography ") {
            return Ok(Block::Bibliography(parse_required_string(
                rest.trim(),
                line.number,
            )?));
        }

        if trimmed == "abstract do" {
            return Ok(Block::Abstract(self.parse_blocks(line.number)?));
        }

        if trimmed == "quote do" {
            return Ok(Block::Quote(self.parse_blocks(line.number)?));
        }

        if trimmed == "proof do" {
            return Ok(Block::Proof(self.parse_blocks(line.number)?));
        }

        if trimmed == "items do" {
            return self.parse_list(false, line.number);
        }

        if trimmed == "steps do" {
            return self.parse_list(true, line.number);
        }

        if trimmed == "table do" {
            return self.parse_table(line.number);
        }

        if trimmed == "compare do" {
            return self.parse_compare(line.number);
        }

        if trimmed.starts_with("equation") {
            return self.parse_equation(trimmed, line.number);
        }

        if trimmed.starts_with("figure ") {
            return self.parse_figure(trimmed, line.number);
        }

        if trimmed.starts_with("theorem ") {
            let title = parse_titled_do(trimmed, "theorem", line.number)?;
            let children = self.parse_blocks(line.number)?;
            return Ok(Block::Theorem {
                title: Some(title),
                children,
            });
        }

        for (keyword, level) in [("section", 1), ("subsection", 2), ("subsubsection", 3)] {
            if trimmed.starts_with(keyword) {
                let title = parse_titled_do(trimmed, keyword, line.number)?;
                let children = self.parse_blocks(line.number)?;
                return Ok(Block::Section {
                    level,
                    title,
                    children,
                });
            }
        }

        Err(Diagnostic::new(
            line.number,
            format!("unknown Vitamins statement: `{trimmed}`"),
        ))
    }

    fn parse_list(&mut self, numbered: bool, opener: usize) -> Result<Block, Diagnostic> {
        let mut items = Vec::new();

        loop {
            let line = self
                .take_significant()
                .ok_or_else(|| Diagnostic::new(opener, "unclosed list block"))?;
            let trimmed = line.text.trim();

            if trimmed == "end" {
                return Ok(if numbered {
                    Block::Steps(items)
                } else {
                    Block::Items(items)
                });
            }

            let rest = trimmed
                .strip_prefix("item ")
                .ok_or_else(|| Diagnostic::new(line.number, "expected `item ...`"))?;
            items.push(parse_inline_list(rest, line.number)?);
        }
    }

    fn parse_equation(&mut self, trimmed: &str, line: usize) -> Result<Block, Diagnostic> {
        let header = strip_keyword_do(trimmed, "equation", line)?;
        let label = parse_symbol_option(header, "label", line)?;
        let expr = self.collect_raw_block(line)?;
        Ok(Block::Equation { label, expr })
    }

    fn parse_figure(&mut self, trimmed: &str, line: usize) -> Result<Block, Diagnostic> {
        let header = strip_keyword_do(trimmed, "figure", line)?;
        let path = parse_string_option(header, "path", line)?
            .ok_or_else(|| Diagnostic::new(line, "figure requires `path: \"...\"`"))?;
        let width = parse_width_option(header, "width");
        let mut caption = None;
        let mut label = None;

        loop {
            let body_line = self
                .take_significant()
                .ok_or_else(|| Diagnostic::new(line, "unclosed figure block"))?;
            let body = body_line.text.trim();

            if body == "end" {
                return Ok(Block::Figure {
                    path,
                    width,
                    caption,
                    label,
                });
            }

            if let Some(rest) = body.strip_prefix("caption ") {
                caption = Some(parse_required_string(rest.trim(), body_line.number)?);
            } else if let Some(rest) = body.strip_prefix("label ") {
                label = Some(parse_symbol(rest.trim(), body_line.number)?);
            } else {
                return Err(Diagnostic::new(
                    body_line.number,
                    "expected `caption ...` or `label ...` in figure",
                ));
            }
        }
    }

    fn parse_table(&mut self, opener: usize) -> Result<Block, Diagnostic> {
        let mut columns = Vec::new();
        let mut rows = Vec::new();
        let mut rules = Vec::new();
        let mut caption = None;

        loop {
            let line = self
                .take_significant()
                .ok_or_else(|| Diagnostic::new(opener, "unclosed table block"))?;
            let trimmed = line.text.trim();

            if trimmed == "end" {
                return Ok(Block::Table {
                    columns,
                    rows,
                    rules,
                    caption,
                });
            }

            if let Some(rest) = trimmed.strip_prefix("columns ") {
                columns = parse_columns(rest, line.number)?;
            } else if let Some(rest) = trimmed.strip_prefix("row ") {
                let cells = split_top_level_commas(rest)
                    .into_iter()
                    .map(|cell| parse_inline_list(cell.trim(), line.number))
                    .collect::<Result<Vec<_>, _>>()?;
                rows.push(cells);
            } else if trimmed == "rule" {
                rules.push(rows.len());
            } else if let Some(rest) = trimmed.strip_prefix("caption ") {
                caption = Some(parse_required_string(rest.trim(), line.number)?);
            } else {
                return Err(Diagnostic::new(
                    line.number,
                    "expected `columns`, `row`, `rule`, or `caption` in table",
                ));
            }
        }
    }

    fn parse_compare(&mut self, opener: usize) -> Result<Block, Diagnostic> {
        let mut latex = None;
        let mut vitamins = None;

        loop {
            let line = self
                .take_significant()
                .ok_or_else(|| Diagnostic::new(opener, "unclosed compare block"))?;
            let trimmed = line.text.trim();

            if trimmed == "end" {
                return Ok(Block::Compare {
                    latex: latex.unwrap_or_default(),
                    vitamins: vitamins.unwrap_or_default(),
                });
            }

            if let Some(rest) = trimmed.strip_prefix("latex ") {
                latex = Some(parse_required_string(rest.trim(), line.number)?);
            } else if let Some(rest) = trimmed.strip_prefix("vitamins ") {
                vitamins = Some(parse_vitamins_snippet(rest.trim(), line.number)?);
            } else {
                return Err(Diagnostic::new(
                    line.number,
                    "expected `latex ...` or `vitamins ...` in compare block",
                ));
            }
        }
    }

    fn collect_raw_block(&mut self, opener: usize) -> Result<String, Diagnostic> {
        let mut lines = Vec::new();
        let mut depth = 0usize;

        loop {
            let line = self
                .take_raw_line()
                .ok_or_else(|| Diagnostic::new(opener, "unclosed raw block"))?;
            let trimmed = line.text.trim();

            if trimmed == "end" && depth == 0 {
                return Ok(lines.join("\n"));
            }

            if trimmed.starts_with("end") && depth > 0 {
                depth -= 1;
                lines.push(trimmed.to_string());
                continue;
            }

            if has_do_word(trimmed) {
                depth += 1;
            }

            if !trimmed.is_empty() {
                lines.push(trimmed.to_string());
            }
        }
    }

    fn take_significant(&mut self) -> Option<SourceLine> {
        while let Some(line) = self.lines.get(self.position) {
            let trimmed = line.text.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                break;
            }
            self.position += 1;
        }

        self.take_raw_line()
    }

    fn take_raw_line(&mut self) -> Option<SourceLine> {
        let line = self.lines.get(self.position)?.clone();
        self.position += 1;
        Some(line)
    }
}

fn parse_titled_do(line: &str, keyword: &str, number: usize) -> Result<String, Diagnostic> {
    let rest = line
        .strip_prefix(keyword)
        .ok_or_else(|| Diagnostic::new(number, format!("expected `{keyword}`")))?;
    let (title, tail) = parse_string_literal(rest.trim(), number)?;

    if tail.trim() == "do" {
        Ok(title)
    } else {
        Err(Diagnostic::new(
            number,
            format!("expected `{keyword} \"...\" do`"),
        ))
    }
}

fn parse_required_string(input: &str, line: usize) -> Result<String, Diagnostic> {
    let (value, tail) = parse_string_literal(input, line)?;
    if tail.trim().is_empty() {
        Ok(value)
    } else {
        Err(Diagnostic::new(
            line,
            "unexpected text after string literal",
        ))
    }
}

fn parse_string_literal(input: &str, line: usize) -> Result<(String, &str), Diagnostic> {
    let input = input.trim_start();
    let Some(rest) = input.strip_prefix('"') else {
        return Err(Diagnostic::new(line, "expected string literal"));
    };

    let mut output = String::new();
    let mut chars = rest.char_indices();

    while let Some((index, character)) = chars.next() {
        match character {
            '"' => return Ok((output, &rest[index + character.len_utf8()..])),
            '\\' => {
                let Some((_, escaped)) = chars.next() else {
                    return Err(Diagnostic::new(line, "unterminated escape sequence"));
                };
                match escaped {
                    'n' => output.push('\n'),
                    't' => output.push('\t'),
                    '"' => output.push('"'),
                    '\\' => output.push('\\'),
                    other => {
                        output.push('\\');
                        output.push(other);
                    }
                }
            }
            other => output.push(other),
        }
    }

    Err(Diagnostic::new(line, "unterminated string literal"))
}

fn strip_keyword_do<'a>(
    line: &'a str,
    keyword: &str,
    number: usize,
) -> Result<&'a str, Diagnostic> {
    let rest = line
        .strip_prefix(keyword)
        .ok_or_else(|| Diagnostic::new(number, format!("expected `{keyword}`")))?
        .trim();

    if rest == "do" {
        Ok("")
    } else if let Some(header) = rest.strip_suffix(" do") {
        Ok(header.trim())
    } else {
        Err(Diagnostic::new(
            number,
            format!("expected `{keyword} ... do`"),
        ))
    }
}

fn parse_feature(input: &str, line: usize) -> Result<Feature, Diagnostic> {
    match input {
        "bibliography" => Ok(Feature::Bibliography),
        "graphics" => Ok(Feature::Graphics),
        "math" => Ok(Feature::Math),
        "theorem" => Ok(Feature::Theorem),
        other => Err(Diagnostic::new(
            line,
            format!("unknown feature `{other}`; expected math, theorem, graphics, or bibliography"),
        )),
    }
}

fn parse_definition(input: &str, line: usize) -> Result<Definition, Diagnostic> {
    let Some((name_start, name_end, name)) = next_identifier(input) else {
        return Err(Diagnostic::new(line, "expected macro name after `define`"));
    };

    if name_start != 0 {
        return Err(Diagnostic::new(line, "expected macro name after `define`"));
    }

    if !is_latex_command_name(name) {
        return Err(Diagnostic::new(
            line,
            "macro names must contain only ASCII letters",
        ));
    }

    let rest = input[name_end..].trim_start();
    let Some(after_open) = rest.strip_prefix('(') else {
        return Err(Diagnostic::new(
            line,
            "expected macro parameters in `(...)`",
        ));
    };
    let close = matching_close(after_open, '(', ')')
        .ok_or_else(|| Diagnostic::new(line, "unclosed macro parameter list"))?;
    let params = parse_definition_params(&after_open[..close], line)?;
    let latex = parse_required_string(after_open[close + 1..].trim(), line)?;

    Ok(Definition {
        name: name.to_string(),
        params,
        latex,
    })
}

fn parse_definition_params(input: &str, line: usize) -> Result<Vec<String>, Diagnostic> {
    let mut params = Vec::new();
    for param in split_top_level_commas(input)
        .into_iter()
        .map(str::trim)
        .filter(|param| !param.is_empty())
    {
        if !is_identifier(param) {
            return Err(Diagnostic::new(
                line,
                format!("invalid macro parameter `{param}`"),
            ));
        }
        if params.iter().any(|existing| existing == param) {
            return Err(Diagnostic::new(
                line,
                format!("duplicate macro parameter `{param}`"),
            ));
        }
        params.push(param.to_string());
    }
    Ok(params)
}

fn parse_symbol_option(input: &str, key: &str, line: usize) -> Result<Option<String>, Diagnostic> {
    let Some(value) = option_value(input, key) else {
        return Ok(None);
    };
    parse_symbol(value.trim(), line).map(Some)
}

fn parse_string_option(input: &str, key: &str, line: usize) -> Result<Option<String>, Diagnostic> {
    let Some(value) = option_value(input, key) else {
        return Ok(None);
    };
    let (parsed, _) = parse_string_literal(value.trim(), line)?;
    Ok(Some(parsed))
}

fn parse_width_option(input: &str, key: &str) -> Option<String> {
    let value = option_value(input, key)?.trim();
    Some(if let Some(number) = value.strip_suffix(".page") {
        format!("{}\\textwidth", number.trim())
    } else {
        value.to_string()
    })
}

fn option_value<'a>(input: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("{key}:");
    let start = input.find(&needle)? + needle.len();
    let rest = &input[start..];
    let end = find_top_level_comma(rest).unwrap_or(rest.len());
    Some(rest[..end].trim())
}

fn find_top_level_comma(input: &str) -> Option<usize> {
    split_index(input, ',')
}

fn parse_symbol(input: &str, line: usize) -> Result<String, Diagnostic> {
    let input = input.trim();

    if let Some(rest) = input.strip_prefix(':') {
        if is_identifier(rest) {
            return Ok(rest.to_string());
        }
    }

    if input.starts_with('"') {
        return parse_required_string(input, line);
    }

    Err(Diagnostic::new(line, "expected symbol like `:name`"))
}

fn parse_columns(input: &str, line: usize) -> Result<Vec<Align>, Diagnostic> {
    split_top_level_commas(input)
        .into_iter()
        .map(|column| match column.trim() {
            ":left" | "left" => Ok(Align::Left),
            ":center" | "center" => Ok(Align::Center),
            ":right" | "right" => Ok(Align::Right),
            other => Err(Diagnostic::new(
                line,
                format!("unknown column alignment `{other}`"),
            )),
        })
        .collect()
}

fn parse_vitamins_snippet(input: &str, line: usize) -> Result<Vec<Inline>, Diagnostic> {
    for (name, wrap) in [
        ("bold ", "bold"),
        ("italic ", "italic"),
        ("small_caps ", "small_caps"),
    ] {
        if let Some(rest) = input.strip_prefix(name) {
            let children = parse_inline_list(rest.trim(), line)?;
            return Ok(vec![match wrap {
                "bold" => Inline::Bold(children),
                "italic" => Inline::Italic(children),
                _ => Inline::SmallCaps(children),
            }]);
        }
    }

    parse_inline_list(input, line)
}

fn parse_inline_list(input: &str, line: usize) -> Result<Vec<Inline>, Diagnostic> {
    split_top_level_commas(input)
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .map(|part| parse_inline_atom(part.trim(), line))
        .collect()
}

fn parse_inline_atom(input: &str, line: usize) -> Result<Inline, Diagnostic> {
    if input.starts_with('"') {
        let (text, tail) = parse_string_literal(input, line)?;
        let mut inline = Inline::Text(text);
        let mut rest = tail.trim();

        while let Some(chain) = rest.strip_prefix('.') {
            if let Some(next) = chain.strip_prefix("bold") {
                inline = Inline::Bold(vec![inline]);
                rest = next.trim();
            } else if let Some(next) = chain.strip_prefix("italic") {
                inline = Inline::Italic(vec![inline]);
                rest = next.trim();
            } else if let Some(next) = chain.strip_prefix("small_caps") {
                inline = Inline::SmallCaps(vec![inline]);
                rest = next.trim();
            } else {
                return Err(Diagnostic::new(line, "unknown string formatting chain"));
            }
        }

        if rest.is_empty() {
            return Ok(inline);
        }

        return Err(Diagnostic::new(line, "unexpected text after inline string"));
    }

    for (name, constructor) in [
        ("bold", InlineConstructor::Bold),
        ("italic", InlineConstructor::Italic),
        ("small_caps", InlineConstructor::SmallCaps),
    ] {
        if let Some(body) = parse_call_body(input, name) {
            let children = parse_inline_list(body, line)?;
            return Ok(constructor.wrap(children));
        }

        if let Some(rest) = input.strip_prefix(&format!("{name} ")) {
            let children = parse_inline_list(rest.trim(), line)?;
            return Ok(constructor.wrap(children));
        }
    }

    if let Some(body) = parse_call_body(input, "cite") {
        return parse_symbol(body.trim(), line).map(Inline::Cite);
    }

    if let Some(body) = parse_call_body(input, "ref") {
        return parse_symbol(body.trim(), line).map(Inline::Ref);
    }

    if let Some(body) = parse_call_body(input, "latex") {
        return parse_required_string(body.trim(), line).map(Inline::Latex);
    }

    if let Some(rest) = input.strip_prefix("latex ") {
        return parse_required_string(rest.trim(), line).map(Inline::Latex);
    }

    if let Some(body) = parse_brace_body(input, "math") {
        return Ok(Inline::Math(body.trim().to_string()));
    }

    if is_plain_number(input) {
        return Ok(Inline::Text(input.to_string()));
    }

    Ok(Inline::Math(input.to_string()))
}

#[derive(Debug, Clone, Copy)]
enum InlineConstructor {
    Bold,
    Italic,
    SmallCaps,
}

impl InlineConstructor {
    fn wrap(self, children: Vec<Inline>) -> Inline {
        match self {
            Self::Bold => Inline::Bold(children),
            Self::Italic => Inline::Italic(children),
            Self::SmallCaps => Inline::SmallCaps(children),
        }
    }
}

fn parse_call_body<'a>(input: &'a str, name: &str) -> Option<&'a str> {
    let rest = input.strip_prefix(name)?;
    let rest = rest.strip_prefix('(')?;
    let close = matching_close(rest, '(', ')')?;

    if rest[close + 1..].trim().is_empty() {
        Some(&rest[..close])
    } else {
        None
    }
}

fn parse_brace_body<'a>(input: &'a str, name: &str) -> Option<&'a str> {
    let rest = input.strip_prefix(name)?.trim_start();
    let rest = rest.strip_prefix('{')?;
    let close = matching_close(rest, '{', '}')?;

    if rest[close + 1..].trim().is_empty() {
        Some(&rest[..close])
    } else {
        None
    }
}

fn matching_close(input: &str, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        if character == '"' {
            in_string = true;
        } else if character == open {
            depth += 1;
        } else if character == close {
            if depth == 0 {
                return Some(index);
            }
            depth -= 1;
        }
    }

    None
}

fn split_top_level_commas(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0isize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&input[start..index]);
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }

    parts.push(&input[start..]);
    parts
}

fn split_index(input: &str, needle: char) -> Option<usize> {
    let mut depth = 0isize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            value if value == needle && depth == 0 => return Some(index),
            _ => {}
        }
    }

    None
}

fn has_do_word(input: &str) -> bool {
    input == "do" || input.ends_with(" do") || input.contains(" do ")
}

fn is_identifier(input: &str) -> bool {
    let mut chars = input.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn is_latex_command_name(input: &str) -> bool {
    !input.is_empty()
        && input
            .chars()
            .all(|character| character.is_ascii_alphabetic())
}

fn is_plain_number(input: &str) -> bool {
    !input.is_empty()
        && input
            .chars()
            .all(|character| character.is_ascii_digit() || character == '.')
        && input.chars().any(|character| character.is_ascii_digit())
}

fn emit_document(document: &Document) -> String {
    let features = detect_features(document);
    let mut out = String::new();

    push_line(&mut out, "\\documentclass{article}");
    if features.contains(&Feature::Math) {
        push_line(&mut out, "\\usepackage{amsmath}");
        push_line(&mut out, "\\usepackage{amssymb}");
    }
    if features.contains(&Feature::Theorem) {
        push_line(&mut out, "\\usepackage{amsthm}");
    }
    if features.contains(&Feature::Graphics) {
        push_line(&mut out, "\\usepackage{graphicx}");
    }
    if features.contains(&Feature::Theorem) {
        push_line(&mut out, "\\newtheorem{theorem}{Theorem}");
    }
    for definition in &document.definitions {
        emit_definition(&mut out, definition);
    }

    push_line(
        &mut out,
        &format!("\\title{{{}}}", escape_latex(&document.title)),
    );
    if let Some(author) = &document.author {
        push_line(&mut out, &format!("\\author{{{}}}", escape_latex(author)));
    }
    if let Some(date) = &document.date {
        match date {
            DateValue::Literal(value) => {
                push_line(&mut out, &format!("\\date{{{}}}", escape_latex(value)));
            }
            DateValue::Today => push_line(&mut out, "\\date{\\today}"),
        }
    }

    push_line(&mut out, "\\begin{document}");
    push_line(&mut out, "\\maketitle");
    emit_blocks(&mut out, &document.body, &document.definitions);
    push_line(&mut out, "\\end{document}");

    out
}

fn emit_definition(out: &mut String, definition: &Definition) {
    let arity = definition.params.len();
    if arity == 0 {
        push_line(
            out,
            &format!(
                "\\newcommand{{\\{}}}{{{}}}",
                definition.name, definition.latex
            ),
        );
    } else {
        push_line(
            out,
            &format!(
                "\\newcommand{{\\{}}}[{}]{{{}}}",
                definition.name, arity, definition.latex
            ),
        );
    }
}

fn emit_blocks(out: &mut String, blocks: &[Block], definitions: &[Definition]) {
    for block in blocks {
        emit_block(out, block, definitions);
    }
}

fn emit_block(out: &mut String, block: &Block, definitions: &[Definition]) {
    match block {
        Block::Abstract(children) => {
            push_line(out, "\\begin{abstract}");
            emit_blocks(out, children, definitions);
            push_line(out, "\\end{abstract}");
        }
        Block::Bibliography(path) => {
            let path = path.trim_end_matches(".bib");
            push_line(out, "\\bibliographystyle{plain}");
            push_line(out, &format!("\\bibliography{{{}}}", escape_latex(path)));
        }
        Block::Compare { latex, vitamins } => {
            push_line(out, "\\begin{quote}");
            push_line(
                out,
                &format!("LaTeX: \\texttt{{{}}}\\\\", escape_latex(latex)),
            );
            push_line(
                out,
                &format!(
                    "Vitamins: {}",
                    render_inline_sequence(vitamins, definitions)
                ),
            );
            push_line(out, "\\end{quote}");
        }
        Block::Equation { label, expr } => {
            let math = render_math(expr, definitions);
            if let Some(label) = label {
                push_line(out, "\\begin{equation}");
                push_line(out, &format!("\\label{{{}}}", latex_identifier(label)));
                push_line(out, &math);
                push_line(out, "\\end{equation}");
            } else {
                push_line(out, "\\[");
                push_line(out, &math);
                push_line(out, "\\]");
            }
        }
        Block::Figure {
            path,
            width,
            caption,
            label,
        } => {
            push_line(out, "\\begin{figure}");
            push_line(out, "\\centering");
            if let Some(width) = width {
                push_line(
                    out,
                    &format!(
                        "\\includegraphics[width={}]{{{}}}",
                        width,
                        escape_latex(path)
                    ),
                );
            } else {
                push_line(out, &format!("\\includegraphics{{{}}}", escape_latex(path)));
            }
            if let Some(caption) = caption {
                push_line(out, &format!("\\caption{{{}}}", escape_latex(caption)));
            }
            if let Some(label) = label {
                push_line(out, &format!("\\label{{{}}}", scoped_label("fig", label)));
            }
            push_line(out, "\\end{figure}");
        }
        Block::Items(items) => emit_list(out, "itemize", items, definitions),
        Block::Paragraph(inlines) => push_line(out, &render_inline_sequence(inlines, definitions)),
        Block::Proof(children) => {
            push_line(out, "\\begin{proof}");
            emit_blocks(out, children, definitions);
            push_line(out, "\\end{proof}");
        }
        Block::Quote(children) => {
            push_line(out, "\\begin{quote}");
            emit_blocks(out, children, definitions);
            push_line(out, "\\end{quote}");
        }
        Block::RawLatex(value) => push_line(out, value),
        Block::Section {
            level,
            title,
            children,
        } => {
            let command = match level {
                1 => "section",
                2 => "subsection",
                _ => "subsubsection",
            };
            push_line(out, &format!("\\{command}{{{}}}", escape_latex(title)));
            emit_blocks(out, children, definitions);
        }
        Block::Steps(items) => emit_list(out, "enumerate", items, definitions),
        Block::Table {
            columns,
            rows,
            rules,
            caption,
        } => emit_table(out, columns, rows, rules, caption.as_deref(), definitions),
        Block::Theorem { title, children } => {
            if let Some(title) = title {
                push_line(out, &format!("\\begin{{theorem}}[{}]", escape_latex(title)));
            } else {
                push_line(out, "\\begin{theorem}");
            }
            emit_blocks(out, children, definitions);
            push_line(out, "\\end{theorem}");
        }
    }
}

fn emit_list(
    out: &mut String,
    environment: &str,
    items: &[Vec<Inline>],
    definitions: &[Definition],
) {
    push_line(out, &format!("\\begin{{{environment}}}"));
    for item in items {
        push_line(
            out,
            &format!("\\item {}", render_inline_sequence(item, definitions)),
        );
    }
    push_line(out, &format!("\\end{{{environment}}}"));
}

fn emit_table(
    out: &mut String,
    columns: &[Align],
    rows: &[Vec<Vec<Inline>>],
    rules: &[usize],
    caption: Option<&str>,
    definitions: &[Definition],
) {
    let inferred_columns = rows.iter().map(Vec::len).max().unwrap_or(1);
    let column_spec = if columns.is_empty() {
        "l".repeat(inferred_columns)
    } else {
        columns
            .iter()
            .map(|align| match align {
                Align::Left => 'l',
                Align::Center => 'c',
                Align::Right => 'r',
            })
            .collect()
    };

    push_line(out, "\\begin{table}");
    push_line(out, "\\centering");
    push_line(out, &format!("\\begin{{tabular}}{{{column_spec}}}"));
    for (index, row) in rows.iter().enumerate() {
        if rules.contains(&index) {
            push_line(out, "\\hline");
        }
        let cells = row
            .iter()
            .map(|cell| render_inline_sequence(cell, definitions))
            .collect::<Vec<_>>()
            .join(" & ");
        push_line(out, &format!("{cells} \\\\"));
    }
    if rules.contains(&rows.len()) {
        push_line(out, "\\hline");
    }
    push_line(out, "\\end{tabular}");
    if let Some(caption) = caption {
        push_line(out, &format!("\\caption{{{}}}", escape_latex(caption)));
    }
    push_line(out, "\\end{table}");
}

fn render_inline_sequence(inlines: &[Inline], definitions: &[Definition]) -> String {
    let mut output = String::new();

    for inline in inlines {
        let rendered = render_inline(inline, definitions);
        if needs_space(&output, &rendered) {
            output.push(' ');
        }
        output.push_str(&rendered);
    }

    output
}

fn render_inline(inline: &Inline, definitions: &[Definition]) -> String {
    match inline {
        Inline::Bold(children) => format!(
            "\\textbf{{{}}}",
            render_inline_sequence(children, definitions)
        ),
        Inline::Cite(key) => format!("\\cite{{{}}}", latex_identifier(key)),
        Inline::Italic(children) => format!(
            "\\textit{{{}}}",
            render_inline_sequence(children, definitions)
        ),
        Inline::Latex(value) => value.clone(),
        Inline::Math(expr) => format!("${}$", render_math(expr, definitions)),
        Inline::Ref(key) => format!("\\ref{{{}}}", latex_identifier(key)),
        Inline::SmallCaps(children) => format!(
            "\\textsc{{{}}}",
            render_inline_sequence(children, definitions)
        ),
        Inline::Text(value) => escape_latex(value),
    }
}

fn needs_space(previous: &str, next: &str) -> bool {
    if previous.is_empty() || next.is_empty() {
        return false;
    }

    if previous.ends_with(char::is_whitespace) {
        return false;
    }

    let Some(first) = next.chars().next() else {
        return false;
    };

    !matches!(first, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']')
}

fn render_math(input: &str, definitions: &[Definition]) -> String {
    let normalized = input.split_whitespace().collect::<Vec<_>>().join(" ");
    render_math_expr(&normalized, definitions)
}

fn render_math_expr(input: &str, definitions: &[Definition]) -> String {
    let with_blocks = rewrite_block_operators(input, definitions);
    let with_calls = rewrite_math_calls(&with_blocks, definitions);
    rewrite_math_tokens(&with_calls)
}

fn rewrite_block_operators(input: &str, definitions: &[Definition]) -> String {
    let mut output = input.to_string();

    loop {
        let Some((start, keyword)) = find_block_operator(&output) else {
            return output;
        };

        let Some((replacement, end)) = parse_block_operator(&output[start..], keyword, definitions)
        else {
            return output;
        };

        output.replace_range(start..start + end, &replacement);
    }
}

fn find_block_operator(input: &str) -> Option<(usize, &'static str)> {
    [("integral", "integral"), ("sum", "sum")]
        .into_iter()
        .filter_map(|(needle, keyword)| find_word(input, needle).map(|index| (index, keyword)))
        .min_by_key(|(index, _)| *index)
}

fn parse_block_operator(
    input: &str,
    keyword: &str,
    definitions: &[Definition],
) -> Option<(String, usize)> {
    input.strip_prefix(keyword)?;

    let mut cursor = keyword.len();
    cursor += input[cursor..].len() - input[cursor..].trim_start().len();

    let colon = input[cursor..].find(':')? + cursor;
    let variable = input[cursor..colon].trim();
    cursor = colon + 1;
    cursor += input[cursor..].len() - input[cursor..].trim_start().len();

    let do_index = input[cursor..].find(" do ")? + cursor;
    let bounds = input[cursor..do_index].trim();
    let (lower, upper) = bounds.split_once("..")?;
    let body_start = do_index + " do ".len();
    let after_do = &input[body_start..];
    let end_index = find_word(after_do, "end")?;
    let body = after_do[..end_index].trim();
    let consumed = body_start + end_index + "end".len();

    let rendered_lower = render_math_expr(lower.trim(), definitions);
    let rendered_upper = render_math_expr(upper.trim(), definitions);
    let rendered_body = render_math_expr(body, definitions);
    let replacement = if keyword == "integral" {
        format!("\\int_{{{rendered_lower}}}^{{{rendered_upper}}} {rendered_body} \\, d{variable}")
    } else {
        format!("\\sum_{{{variable}={rendered_lower}}}^{{{rendered_upper}}} {rendered_body}")
    };

    Some((replacement, consumed))
}

fn rewrite_math_calls(input: &str, definitions: &[Definition]) -> String {
    let mut output = String::new();
    let mut index = 0usize;

    while index < input.len() {
        let rest = &input[index..];
        let Some((name_start, name_end, name)) = next_identifier(rest) else {
            output.push_str(rest);
            break;
        };

        output.push_str(&rest[..name_start]);
        if rest[..name_start].ends_with('\\') {
            output.push_str(name);
            index += name_end;
            continue;
        }

        let after_name = &rest[name_end..];
        if let Some(after_open) = after_name.strip_prefix('(') {
            if let Some(close) = matching_close(after_open, '(', ')') {
                let body = &after_open[..close];
                output.push_str(&render_math_call(name, body, definitions));
                index += name_end + 1 + close + 1;
                continue;
            }
        }

        output.push_str(name);
        index += name_end;
    }

    output
}

fn render_math_call(name: &str, body: &str, definitions: &[Definition]) -> String {
    let args = split_top_level_commas(body)
        .into_iter()
        .map(|arg| render_math_expr(arg.trim(), definitions))
        .collect::<Vec<_>>();

    if let Some(definition) = definitions
        .iter()
        .find(|definition| definition.name == name && definition.params.len() == args.len())
    {
        return render_macro_call(definition, &args);
    }

    match (name, args.as_slice()) {
        ("frac", [top, bottom]) => format!("\\frac{{{top}}}{{{bottom}}}"),
        ("sqrt", [value]) => format!("\\sqrt{{{value}}}"),
        ("exp", [value]) => format!("\\exp\\left({value}\\right)"),
        ("cos", [value]) => format!("\\cos\\left({value}\\right)"),
        ("sin", [value]) => format!("\\sin\\left({value}\\right)"),
        ("log", [value]) => format!("\\log\\left({value}\\right)"),
        ("mathbb", [value]) => format!("\\mathbb{{{value}}}"),
        ("norm", [value]) => format!("\\lVert {value} \\rVert"),
        (_, []) => format!("{name}\\left(\\right)"),
        _ => format!("{name}\\left({}\\right)", args.join(", ")),
    }
}

fn render_macro_call(definition: &Definition, args: &[String]) -> String {
    let mut output = format!("\\{}", definition.name);
    for arg in args {
        output.push_str(&format!("{{{arg}}}"));
    }
    output
}

fn rewrite_math_tokens(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();

    while let Some(character) = chars.next() {
        match character {
            '\\' => {
                output.push('\\');
                while let Some(next) = chars.peek().copied() {
                    if next.is_ascii_alphabetic() {
                        output.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
            '=' if chars.peek() == Some(&'=') => {
                chars.next();
                output.push('=');
            }
            '>' if chars.peek() == Some(&'=') => {
                chars.next();
                output.push_str("\\ge");
            }
            '<' if chars.peek() == Some(&'=') => {
                chars.next();
                output.push_str("\\le");
            }
            '!' if chars.peek() == Some(&'=') => {
                chars.next();
                output.push_str("\\ne");
            }
            '*' => output.push_str("\\cdot"),
            character if character.is_ascii_alphabetic() || character == '_' => {
                let mut word = String::from(character);
                while let Some(next) = chars.peek().copied() {
                    if next.is_ascii_alphanumeric() || next == '_' {
                        word.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }

                match word.as_str() {
                    "pi" => output.push_str("\\pi"),
                    "infinity" => output.push_str("\\infty"),
                    _ => output.push_str(&word),
                }
            }
            other => output.push(other),
        }
    }

    output
}

fn next_identifier(input: &str) -> Option<(usize, usize, &str)> {
    let mut start = None;
    let mut end = 0usize;

    for (index, character) in input.char_indices() {
        if start.is_none() {
            if character.is_ascii_alphabetic() || character == '_' {
                start = Some(index);
                end = index + character.len_utf8();
            }
        } else if character.is_ascii_alphanumeric() || character == '_' {
            end = index + character.len_utf8();
        } else {
            break;
        }
    }

    let start = start?;
    Some((start, end, &input[start..end]))
}

fn find_word(input: &str, word: &str) -> Option<usize> {
    let mut offset = 0usize;

    while let Some(index) = input[offset..].find(word) {
        let absolute = offset + index;
        let before = input[..absolute].chars().next_back();
        let after = input[absolute + word.len()..].chars().next();

        let before_ok = before.is_none_or(|character| !character.is_ascii_alphanumeric());
        let after_ok = after.is_none_or(|character| !character.is_ascii_alphanumeric());

        if before_ok && after_ok {
            return Some(absolute);
        }

        offset = absolute + word.len();
    }

    None
}

fn detect_features(document: &Document) -> BTreeSet<Feature> {
    let mut features = document.uses.clone();
    for block in &document.body {
        detect_block_features(block, &mut features);
    }
    features
}

fn detect_block_features(block: &Block, features: &mut BTreeSet<Feature>) {
    match block {
        Block::Abstract(children)
        | Block::Proof(children)
        | Block::Quote(children)
        | Block::Section { children, .. }
        | Block::Theorem { children, .. } => {
            if matches!(block, Block::Theorem { .. } | Block::Proof(_)) {
                features.insert(Feature::Theorem);
            }
            for child in children {
                detect_block_features(child, features);
            }
        }
        Block::Equation { .. } => {
            features.insert(Feature::Math);
        }
        Block::Figure { .. } => {
            features.insert(Feature::Graphics);
        }
        Block::Bibliography(_) => {
            features.insert(Feature::Bibliography);
        }
        Block::Items(items) | Block::Steps(items) => {
            detect_inline_features(items.iter().flatten(), features);
        }
        Block::Paragraph(inlines)
        | Block::Compare {
            vitamins: inlines, ..
        } => {
            detect_inline_features(inlines.iter(), features);
        }
        Block::Table { rows, .. } => {
            for inline in rows.iter().flatten().flatten() {
                detect_inline_feature(inline, features);
            }
        }
        Block::RawLatex(_) => {}
    }
}

fn detect_inline_features<'a>(
    inlines: impl Iterator<Item = &'a Inline>,
    features: &mut BTreeSet<Feature>,
) {
    for inline in inlines {
        detect_inline_feature(inline, features);
    }
}

fn detect_inline_feature(inline: &Inline, features: &mut BTreeSet<Feature>) {
    match inline {
        Inline::Bold(children) | Inline::Italic(children) | Inline::SmallCaps(children) => {
            detect_inline_features(children.iter(), features);
        }
        Inline::Math(_) => {
            features.insert(Feature::Math);
        }
        Inline::Cite(_) => {
            features.insert(Feature::Bibliography);
        }
        Inline::Latex(_) | Inline::Ref(_) | Inline::Text(_) => {}
    }
}

fn scoped_label(scope: &str, label: &str) -> String {
    if label.contains(':') {
        latex_identifier(label)
    } else {
        format!("{scope}:{}", latex_identifier(label))
    }
}

fn latex_identifier(input: &str) -> String {
    input
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, ':' | '_' | '-' | '.' | '/')
            {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn push_line(output: &mut String, line: &str) {
    output.push_str(line);
    output.push('\n');
}

fn escape_latex(input: &str) -> String {
    let mut output = String::new();

    for character in input.chars() {
        match character {
            '&' => output.push_str("\\&"),
            '%' => output.push_str("\\%"),
            '$' => output.push_str("\\$"),
            '#' => output.push_str("\\#"),
            '_' => output.push_str("\\_"),
            '{' => output.push_str("\\{"),
            '}' => output.push_str("\\}"),
            '~' => output.push_str("\\textasciitilde{}"),
            '^' => output.push_str("\\textasciicircum{}"),
            '\\' => output.push_str("\\textbackslash{}"),
            other => output.push(other),
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_minimal_paper() {
        let source = r#"
paper "The Title" do
  author "Grace Hopper"
  date "June 22, 2026"
  section "Introduction" do
    p "Documents should be pleasant to write."
  end
end
"#;

        let latex = compile_to_latex(source).expect("document compiles");
        assert!(latex.contains("\\documentclass{article}"));
        assert!(latex.contains("\\title{The Title}"));
        assert!(latex.contains("\\author{Grace Hopper}"));
        assert!(latex.contains("\\date{June 22, 2026}"));
        assert!(latex.contains("\\section{Introduction}"));
        assert!(latex.contains("Documents should be pleasant to write."));
    }

    #[test]
    fn compiles_inline_formatting_and_math() {
        let source = r#"
paper "Math" do
  use math
  section "A theorem" do
    p "This is", bold("important"), "and", "very important".bold.small_caps, "."
    equation do
      frac(1, 2) + frac(1, 3) == frac(5, 6)
    end
  end
end
"#;

        let latex = compile_to_latex(source).expect("document compiles");
        assert!(latex.contains("\\textbf{important}"));
        assert!(latex.contains("\\textsc{\\textbf{very important}}"));
        assert!(latex.contains("\\frac{1}{2} + \\frac{1}{3} = \\frac{5}{6}"));
    }

    #[test]
    fn compiles_integrals_sums_and_figures() {
        let source = r#"
paper "Features" do
  equation label: :gaussian do
    integral x: -infinity..infinity do
      exp(-x^2)
    end == sqrt(pi)
  end
  figure path: "diagram.pdf", width: 0.8.page do
    caption "A helpful diagram."
    label :helpful_diagram
  end
end
"#;

        let latex = compile_to_latex(source).expect("document compiles");
        assert!(latex.contains("\\label{gaussian}"));
        assert!(
            latex.contains(
                "\\int_{-\\infty}^{\\infty} \\exp\\left(-x^2\\right) \\, dx = \\sqrt{\\pi}"
            )
        );
        assert!(latex.contains("\\includegraphics[width=0.8\\textwidth]{diagram.pdf}"));
        assert!(latex.contains("\\label{fig:helpful_diagram}"));
    }

    #[test]
    fn compiles_custom_macro_definitions() {
        let source = r#"
paper "Macros" do
  use math
  define norm(x) "\\left\\lVert #1 \\right\\rVert"
  define inner(x, y) "\\left\\langle #1, #2 \\right\\rangle"

  equation do
    norm(v) == sqrt(inner(v, v))
  end
end
"#;

        let latex = compile_to_latex(source).expect("document compiles");
        assert!(latex.contains("\\newcommand{\\norm}[1]{\\left\\lVert #1 \\right\\rVert}"));
        assert!(latex.contains("\\newcommand{\\inner}[2]{\\left\\langle #1, #2 \\right\\rangle}"));
        assert!(latex.contains("\\norm{v} = \\sqrt{\\inner{v}{v}}"));
    }

    #[test]
    fn reports_duplicate_macro_definitions() {
        let source = r#"
paper "Oops" do
  define norm(x) "\\lVert #1 \\rVert"
  define norm(x) "\\lvert #1 \\rvert"
end
"#;

        let error = compile_to_latex(source).expect_err("invalid document fails");
        assert_eq!(error.line, 4);
        assert!(error.message.contains("already defined"));
    }

    #[test]
    fn reports_unknown_statement() {
        let source = r#"
paper "Oops" do
  nonsense
end
"#;

        let error = compile_to_latex(source).expect_err("invalid document fails");
        assert_eq!(error.line, 3);
        assert!(error.message.contains("unknown Vitamins statement"));
    }
}
