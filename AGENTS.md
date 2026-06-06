# AGENTS.md - Courier

This document provides guidance for AI coding agents working in this repository.

## Project Overview

Courier is a Rust desktop application built with the Egui GUI framework. 

## Rules and Preferences
- Only comments when necessary. For example, a `// Settings` for a field `setting` is totally unnecessary and ugly.
- Prefer `to_owned()`, for example, than `to_string()` when trying to convert a `&str` to a `String`. Which is to say, `to_string()` should express exactly the "to_string" action, for example, convert an u32 to `String`
- Prefer `info!()` than `tracing::info!()`, but with one exception: ambiguity exists when declaring types, for example, `foo: tokio::sync::Mutex<T>` is better than `foo: Mutex<T>`. The complete namespace makes the intention clear.
- Prefer turbofish syntax than manual type inference. For example, `let bar = foo.try_deserialize::<T>()` is better than `let bar: T = foo.try_deserialize()`. 
- Carefully on visibility control for all fields, structs, functions, modules, etc. Use `pub` only when you have a good and enough reason.
- Always run `cargo +nightly fmt --all` to format the code after editing.
- No need to run any tests unless you are specifically asked to do so.

## Guidelines

### 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

### 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative. Unless you are specifically asked to do so.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

### 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

### 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```