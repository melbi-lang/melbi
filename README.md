# Melbi 🖖

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](Cargo.toml)

**Melbi** is a type-safe, functional, and embedded expression language designed for safe, dynamic logic.

Give your users or power systems the ability to define conditional behavior, data transformations, and automation rules—without ever modifying your application's source code.

## 🌟 Key Features

- **Sandboxed & Restricted**: Ensure runtime safety with a sandboxed environment. Perfect for tasks like defining email filters, user-defined triggers, or complex business rules.
- **Type-Safe**: Features Hindley-Milner type inference. Types are inferred automatically without annotations, catching errors at compile time.
- **Fast & Performant**: Expression-focused and arena-allocated for performance comparable to native code.
- **Functional & Pure**: Immutable by default with pure functional programming principles.
- **No Nulls**: Uses `Option[T]` instead of nulls, and pattern matching enforces handling of both cases.
- **Flexible & Embeddable**: Designed to be effortlessly embeddable in Rust applications, keeping dynamic user logic separate from your main codebase.

## 💻 Quick Example

Melbi's syntax is expression-based and highly readable. Here's a simple example of A/B testing rollout logic:

```melbi
// A/B test rollout decision
user.country == "US" and user.age >= 18 and
Hash.ConsistentHash(user.id, 100) < rollout
where {
    rollout = rollouts[feature_name] otherwise 0
}
```

Or complex string formatting with local variables:

```melbi
f"Hello { name }, your score is { score * 100 }!" where {
    name = "Alice",
    score = 0.95,
}
```

## 🚀 Getting Started

Melbi is currently in active alpha development.

### Installation

To run Melbi locally, you can clone the repository and build the CLI using [Cargo](https://doc.rust-lang.org/cargo/):

```bash
git clone https://github.com/melbi-lang/melbi.git
cd melbi
cargo build --release
```

You can then run the CLI using:
```bash
cargo run --bin melbi
```

### Online Playground

Want to try it without installing? An online playground is available as part of the [tutorial section on our website](https://melbi-lang.github.io/tutorial/).

## 🛠 Tooling Ecosystem

Melbi comes with a rich set of developer tools out of the box:

- **CLI**: For evaluating expressions, running files, checking types, and formatting code.
- **Language Server (LSP)**: Providing IDE features like diagnostics and formatting.
- **Editor Extensions**: Extensions are available for **VS Code** and **Zed**, providing syntax highlighting, auto-formatting (via Topiary), and language support.

## 📚 Documentation & Learning

- **Website**: [melbi-lang.github.io](https://melbi-lang.github.io/)
- **Tutorials & Examples**: Available on the website to help you get started.
- **Cheat Sheet**: A quick reference for Melbi's syntax is available in [`docs/melbi-lang-cheat-sheet.md`](docs/melbi-lang-cheat-sheet.md).
- **Design Docs**: See the `docs/design/` directory for architectural decisions.

## 🤝 Contributing

Melbi welcomes contributors! Whether you're interested in implementation, testing, documentation, or ecosystem tooling, your help is appreciated.

Please check the issue tracker, discussions, and the `docs` folder for areas where you can contribute.

## 📄 License

Melbi is dual-licensed under either of the following licenses, at your option:

- MIT License
- Apache License, Version 2.0
