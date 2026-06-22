# Vitamins

Vitamins is a Ruby-like document language that compiles to LaTeX. It is not a
replacement for LaTeX's rendering engine; it is a calmer front-end for writing
documents that still become ordinary `.tex` files.

```vitamins
paper "A Tiny Note on Beautiful Typesetting" do
  author "Ada Lovelace"
  date today
  use math
  use theorem
  define norm(x) "\\left\\lVert #1 \\right\\rVert"

  abstract do
    p "This paper introduces Vitamins, a clean language for writing beautiful documents."
  end

  section "A theorem" do
    theorem "Euler" do
      p "For every real number", x, ","
      equation do
        norm(exp(i * pi) + 1) == 0
      end
    end
    proof do
      p "Use the complex exponential definition."
    end
  end
end
```

## Run

```sh
cargo run -- compile examples/tiny_note.vit
cargo run -- check examples/tiny_note.vit
cargo run -- emit examples/tiny_note.vit
```

`compile` writes a `.tex` file next to the input unless `-o output.tex` is
provided. `check` parses and emits LaTeX in memory without writing a file.

## Supported first slice

- `paper`, `author`, `date today`, and literal dates
- `use math`, `use theorem`, `use graphics`, and `use bibliography`
- `abstract`, `section`, `subsection`, `subsubsection`, `quote`
- `p` paragraphs with strings, bare math variables, `bold`, `italic`,
  `small_caps`, `cite`, `ref`, `latex`, and `math { ... }`
- `items`, `steps`, `theorem`, `proof`
- display `equation` blocks with optional `label: :name`
- custom `define name(args) "latex body"` math macros emitted as LaTeX
  `\newcommand` definitions
- math helpers for `frac`, `sqrt`, `exp`, `sin`, `cos`, `log`, `integral`,
  `sum`, `pi`, `infinity`, equality, inequalities, and multiplication
- `figure`, `table`, `bibliography`, `raw_latex`, and `compare`

User-defined environments are intentionally left for the next compiler slice;
`norm(...)` is still available as a built-in math helper when no custom macro
with that name is defined.
